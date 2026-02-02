//! L-System Growth Simulation
//!
//! Simulates biological tree growth with vigor distribution, threshold-based rules,
//! lateral branching from dormant buds, and multi-iteration growth.

use godot::prelude::*;
use std::f32::consts::TAU;

use crate::branch::{BranchSegment, SeededRng};

/// Node growth state types
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GrowthNodeType {
    /// Active growing tip that can extend
    Meristem,
    /// Interior node that has already grown
    Branch,
    /// Pruned branch (vigor fell below cut_threshold)
    Cut,
    /// Original trunk structure (not subject to growth rules)
    Ignored,
    /// Inactive bud waiting for activation
    Dormant,
    /// Converted to flower attachment point
    Flower,
}

/// Growth state information for a node
#[derive(Clone, Debug)]
pub struct GrowthNodeInfo {
    /// Type of growth node
    pub node_type: GrowthNodeType,
    /// Energy available to this node (0.0 - 1.0+)
    pub vigor: f32,
    /// Fraction of parent vigor this node requests
    pub vigor_ratio: f32,
    /// Number of iterations this node has existed
    pub age: u32,
    /// Phyllotaxis angle for child placement (radians)
    pub phyllotaxis_angle: f32,
}

impl Default for GrowthNodeInfo {
    fn default() -> Self {
        Self {
            node_type: GrowthNodeType::Meristem,
            vigor: 1.0,
            vigor_ratio: 1.0,
            age: 0,
            phyllotaxis_angle: 0.0,
        }
    }
}

/// A node in the growth simulation tree
#[derive(Clone, Debug)]
pub struct GrowthNode {
    /// Direction of growth (normalized)
    pub direction: Vector3,
    /// Length of this segment
    pub length: f32,
    /// Radius at base of segment
    pub radius: f32,
    /// Position along parent (0.0-1.0) for lateral buds
    pub position_in_parent: f32,
    /// World position of this node's start
    pub position: Vector3,
    /// Growth state information
    pub info: GrowthNodeInfo,
    /// Child nodes
    pub children: Vec<GrowthNode>,
}

impl GrowthNode {
    /// Create a new growth node
    pub fn new(
        position: Vector3,
        direction: Vector3,
        length: f32,
        radius: f32,
        node_type: GrowthNodeType,
    ) -> Self {
        Self {
            direction,
            length,
            radius,
            position_in_parent: 0.0,
            position,
            info: GrowthNodeInfo {
                node_type,
                vigor: 1.0,
                vigor_ratio: 1.0,
                age: 0,
                phyllotaxis_angle: 0.0,
            },
            children: Vec::new(),
        }
    }

    /// Get the end position of this segment
    pub fn end_position(&self) -> Vector3 {
        self.position + self.direction * self.length
    }

    /// Add a child node
    pub fn add_child(&mut self, child: GrowthNode) {
        self.children.push(child);
    }

    /// Check if this node should be rendered
    pub fn is_renderable(&self) -> bool {
        matches!(
            self.info.node_type,
            GrowthNodeType::Meristem
                | GrowthNodeType::Branch
                | GrowthNodeType::Ignored
                | GrowthNodeType::Flower
        )
    }
}

/// Configuration for growth simulation
#[derive(Clone, Debug)]
pub struct GrowthConfig {
    // Core thresholds
    /// Minimum vigor to extend a branch
    pub grow_threshold: f32,
    /// Vigor below which branches are pruned
    pub cut_threshold: f32,
    /// Vigor above which branches bifurcate
    pub split_threshold: f32,
    /// Vigor for flower conversion
    pub flower_threshold: f32,

    // Apical dominance
    /// Leader dominance (0-1, higher = stronger leader)
    pub apical_dominance: f32,

    // Lateral branching
    /// Enable lateral branching from dormant buds
    pub enable_lateral: bool,
    /// Where buds begin on trunk (0-1)
    pub lateral_start: f32,
    /// Where buds end on trunk (0-1)
    pub lateral_end: f32,
    /// Buds per unit length
    pub lateral_density: f32,
    /// Vigor threshold to activate dormant buds
    pub lateral_activation: f32,
    /// Initial angle from parent (degrees)
    pub lateral_angle: f32,

    // Growth parameters
    /// Number of growth iterations (years)
    pub iterations: u32,
    /// Base segment length
    pub branch_length: f32,
    /// Upward/downward growth tendency
    pub gravitropism: f32,
    /// Direction variation
    pub randomness: f32,

    // Physics
    /// Gravity bending strength
    pub gravity_strength: f32,
    /// Resistance to bending
    pub stiffness: f32,

    // Flowering
    /// Enable flowering at low-vigor tips
    pub enable_flowering: bool,

    // Trunk parameters (for initial structure)
    pub trunk_height: f32,
    pub trunk_radius: f32,
    pub trunk_taper: f32,

    /// Random seed
    pub seed: i32,
}

impl Default for GrowthConfig {
    fn default() -> Self {
        Self {
            grow_threshold: 0.3,
            cut_threshold: 0.1,
            split_threshold: 0.7,
            flower_threshold: 0.15,
            apical_dominance: 0.5,
            enable_lateral: true,
            lateral_start: 0.1,
            lateral_end: 0.9,
            lateral_density: 2.0,
            lateral_activation: 0.4,
            lateral_angle: 45.0,
            iterations: 5,
            branch_length: 0.5,
            gravitropism: 0.1,
            randomness: 0.2,
            gravity_strength: 0.0,
            stiffness: 0.5,
            enable_flowering: false,
            trunk_height: 5.0,
            trunk_radius: 0.5,
            trunk_taper: 0.3,
            seed: 42,
        }
    }
}

/// Create initial trunk structure as a GrowthNode tree
pub fn create_trunk_structure(config: &GrowthConfig) -> GrowthNode {
    // Create trunk as a single Ignored node (not subject to growth rules)
    let mut trunk = GrowthNode::new(
        Vector3::ZERO,
        Vector3::UP,
        config.trunk_height,
        config.trunk_radius,
        GrowthNodeType::Ignored,
    );

    // Add a meristem at the top of the trunk to start growth
    let top_position = trunk.end_position();
    let tip_radius = config.trunk_radius * config.trunk_taper;

    let mut apical_meristem = GrowthNode::new(
        top_position,
        Vector3::UP,
        0.0, // Will be extended during growth
        tip_radius,
        GrowthNodeType::Meristem,
    );
    apical_meristem.info.vigor = 1.0;

    trunk.add_child(apical_meristem);
    trunk
}

/// Calculate vigor ratios for all nodes in the tree (recursive)
/// Returns the total vigor requested by this subtree
fn calculate_vigor_ratios(node: &mut GrowthNode, apical_dominance: f32) -> f32 {
    // Base vigor request depends on node type
    let base_request = match node.info.node_type {
        GrowthNodeType::Meristem => 1.0,
        GrowthNodeType::Dormant => 0.3, // Dormant buds request less
        GrowthNodeType::Cut | GrowthNodeType::Flower => 0.0,
        _ => 0.5, // Interior nodes
    };

    if node.children.is_empty() {
        node.info.vigor_ratio = base_request;
        return base_request;
    }

    // Calculate children's requests
    let mut child_requests: Vec<f32> = Vec::with_capacity(node.children.len());
    for child in &mut node.children {
        let request = calculate_vigor_ratios(child, apical_dominance);
        child_requests.push(request);
    }

    let total_child_request: f32 = child_requests.iter().sum();

    if total_child_request > 0.0 {
        // Apply apical dominance: first child (leader) gets more
        // Higher apical_dominance means stronger leader preference
        for (i, (child, &request)) in node
            .children
            .iter_mut()
            .zip(child_requests.iter())
            .enumerate()
        {
            let is_leader = i == 0;
            let dominance_factor = if is_leader {
                1.0 + apical_dominance
            } else {
                1.0 - apical_dominance * 0.5
            };
            child.info.vigor_ratio = (request / total_child_request) * dominance_factor;
        }
    }

    node.info.vigor_ratio = base_request;
    base_request + total_child_request
}

/// Distribute vigor from parent to children
fn distribute_vigor(node: &mut GrowthNode, available_vigor: f32) {
    node.info.vigor = available_vigor * node.info.vigor_ratio;

    if node.children.is_empty() {
        return;
    }

    // Calculate total ratio of children
    let total_ratio: f32 = node.children.iter().map(|c| c.info.vigor_ratio).sum();

    if total_ratio <= 0.0 {
        return;
    }

    // Distribute vigor to children
    for child in &mut node.children {
        let child_vigor = node.info.vigor * (child.info.vigor_ratio / total_ratio);
        distribute_vigor(child, child_vigor);
    }
}

/// Apply growth rules to transform the tree based on vigor
/// This uses an iterative approach to avoid stack overflow from deep recursion
fn apply_growth_rules(root: &mut GrowthNode, config: &GrowthConfig, rng: &mut SeededRng) {
    // Use a work queue to process nodes iteratively
    // We collect indices to process to avoid borrow issues
    apply_growth_rules_recursive(root, config, rng, 0);
}

/// Recursive helper with depth limit to prevent stack overflow
fn apply_growth_rules_recursive(
    node: &mut GrowthNode,
    config: &GrowthConfig,
    rng: &mut SeededRng,
    depth: u32,
) {
    // Prevent infinite recursion with depth limit
    const MAX_DEPTH: u32 = 50;
    if depth > MAX_DEPTH {
        return;
    }

    let vigor = node.info.vigor;

    // Collect new children to add (to avoid modifying while iterating)
    let mut new_children: Vec<GrowthNode> = Vec::new();

    // Apply rules based on node type
    match node.info.node_type {
        GrowthNodeType::Meristem => {
            // Check if vigor is too low - prune
            if vigor < config.cut_threshold {
                node.info.node_type = GrowthNodeType::Cut;
                node.children.clear();
                return;
            }

            // Check for flowering
            if config.enable_flowering && vigor < config.flower_threshold {
                node.info.node_type = GrowthNodeType::Flower;
                return;
            }

            // Check for growth
            if vigor >= config.grow_threshold {
                // Create extension
                let new_direction = calculate_growth_direction(node, config, rng);
                let new_length = config.branch_length
                    * vigor
                    * rng.range(0.8, 1.2)
                    * (1.0 + config.gravitropism);
                let new_radius = node.radius * 0.8;

                let mut extension = GrowthNode::new(
                    node.end_position(),
                    new_direction,
                    new_length,
                    new_radius,
                    GrowthNodeType::Meristem,
                );
                extension.info.vigor = vigor;
                extension.info.age = 0;
                extension.info.phyllotaxis_angle =
                    node.info.phyllotaxis_angle + 137.5f32.to_radians();

                // Convert current meristem to branch
                node.info.node_type = GrowthNodeType::Branch;
                node.length = new_length;
                new_children.push(extension);
            }

            // Check for splitting (bifurcation) - only if we grew
            if vigor >= config.split_threshold && node.children.len() + new_children.len() < 2 {
                let split_angle = 30.0f32.to_radians();
                let rotation = rng.range(0.0, TAU);

                // Create second branch at an angle
                let perp = get_perpendicular(node.direction);
                let perp2 = node.direction.cross(perp);
                let rotated_perp = perp * rotation.cos() + perp2 * rotation.sin();

                let split_direction = (node.direction * split_angle.cos()
                    + rotated_perp * split_angle.sin())
                .normalized();

                let split_length = config.branch_length * vigor * 0.8 * rng.range(0.7, 1.0);
                let split_radius = node.radius * 0.6;

                let mut split_branch = GrowthNode::new(
                    node.end_position(),
                    split_direction,
                    split_length,
                    split_radius,
                    GrowthNodeType::Meristem,
                );
                split_branch.info.vigor = vigor * 0.5;
                split_branch.info.phyllotaxis_angle =
                    node.info.phyllotaxis_angle + 137.5f32.to_radians() + std::f32::consts::PI;

                new_children.push(split_branch);
            }
        }

        GrowthNodeType::Dormant => {
            // Check for activation
            if vigor >= config.lateral_activation {
                node.info.node_type = GrowthNodeType::Meristem;
                node.info.vigor = vigor;
            }
        }

        GrowthNodeType::Branch | GrowthNodeType::Ignored => {
            // Interior nodes just pass vigor through
            // They might get pruned if vigor is too low
            if vigor < config.cut_threshold
                && node.info.node_type == GrowthNodeType::Branch
                && node.info.age > 2
            {
                node.info.node_type = GrowthNodeType::Cut;
                node.children.clear();
                return;
            }
        }

        GrowthNodeType::Cut | GrowthNodeType::Flower => {
            // Dead ends - do nothing
            return;
        }
    }

    // Increment age
    node.info.age += 1;

    // Add new children (don't recurse into them this iteration - they'll be processed next iteration)
    for child in new_children {
        node.add_child(child);
    }

    // Recurse to existing children only (not newly added ones)
    let child_count = node.children.len();
    for i in 0..child_count {
        // Check if this child was already in the tree (not newly added)
        if node.children[i].info.age > 0
            || node.children[i].info.node_type == GrowthNodeType::Dormant
        {
            apply_growth_rules_recursive(&mut node.children[i], config, rng, depth + 1);
        }
    }
}

/// Calculate growth direction with gravitropism and randomness
fn calculate_growth_direction(
    node: &GrowthNode,
    config: &GrowthConfig,
    rng: &mut SeededRng,
) -> Vector3 {
    let mut direction = node.direction;

    // Apply gravitropism (positive = upward, negative = downward)
    direction.y += config.gravitropism;

    // Apply randomness
    direction.x += rng.range(-config.randomness, config.randomness);
    direction.y += rng.range(-config.randomness * 0.5, config.randomness * 0.5);
    direction.z += rng.range(-config.randomness, config.randomness);

    direction.normalized()
}

/// Get a perpendicular vector to the given direction
fn get_perpendicular(dir: Vector3) -> Vector3 {
    let up = if dir.y.abs() < 0.9 {
        Vector3::UP
    } else {
        Vector3::RIGHT
    };
    dir.cross(up).normalized()
}

/// Create lateral buds along trunk segments
fn create_lateral_buds(node: &mut GrowthNode, config: &GrowthConfig, rng: &mut SeededRng) {
    // Only create buds on Ignored (trunk) segments
    if node.info.node_type != GrowthNodeType::Ignored || !config.enable_lateral {
        // Recurse to children
        for child in &mut node.children {
            create_lateral_buds(child, config, rng);
        }
        return;
    }

    // Calculate bud positions along this segment
    let segment_length = node.length;
    let spacing = 1.0 / config.lateral_density.max(0.1);
    let num_buds = (segment_length / spacing).floor() as i32;

    let lateral_angle_rad = config.lateral_angle.to_radians();
    let mut current_phyllotaxis = rng.range(0.0, TAU);

    for i in 0..num_buds {
        let t = (i as f32 + 0.5) / num_buds as f32;

        // Check if within lateral range
        if t < config.lateral_start || t > config.lateral_end {
            continue;
        }

        // Calculate bud position
        let bud_position = node.position + node.direction * node.length * t;

        // Calculate outward direction using phyllotaxis
        current_phyllotaxis += 137.5f32.to_radians() + rng.range(-0.1, 0.1);
        let perp = get_perpendicular(node.direction);
        let perp2 = node.direction.cross(perp);
        let outward = perp * current_phyllotaxis.cos() + perp2 * current_phyllotaxis.sin();

        // Bud direction: angled outward from trunk
        let bud_direction = (node.direction * lateral_angle_rad.cos()
            + outward * lateral_angle_rad.sin())
        .normalized();

        // Calculate radius at this position (with taper)
        let tip_radius = node.radius * config.trunk_taper;
        let bud_radius = (node.radius * (1.0 - t) + tip_radius * t) * 0.3;

        let mut bud = GrowthNode::new(
            bud_position,
            bud_direction,
            0.1, // Very short initial length
            bud_radius,
            GrowthNodeType::Dormant,
        );
        bud.position_in_parent = t;
        bud.info.vigor = 0.0; // Will be set during vigor distribution
        bud.info.phyllotaxis_angle = current_phyllotaxis;

        node.add_child(bud);
    }

    // Recurse to children
    for child in &mut node.children {
        create_lateral_buds(child, config, rng);
    }
}

/// Apply gravity bending to growth nodes
fn apply_gravity_to_growth(node: &mut GrowthNode, config: &GrowthConfig) {
    if config.gravity_strength <= 0.0 {
        return;
    }

    // Apply gravity based on age and vigor (older, weaker branches droop more)
    let age_factor = (node.info.age as f32 / 10.0).min(1.0);
    let vigor_factor = 1.0 - node.info.vigor.min(1.0);
    let stiffness_factor = 1.0 - config.stiffness;

    let bend_amount = config.gravity_strength * age_factor * vigor_factor * stiffness_factor;

    if bend_amount > 0.01 {
        let mut new_direction = node.direction;
        new_direction.y -= bend_amount;
        node.direction = new_direction.normalized();
    }

    // Recurse to children
    for child in &mut node.children {
        apply_gravity_to_growth(child, config);
    }
}

/// Main growth simulation function
pub fn simulate_growth(mut trunk: GrowthNode, config: &GrowthConfig) -> GrowthNode {
    let mut rng = SeededRng::new(config.seed);

    // Create lateral buds on trunk
    create_lateral_buds(&mut trunk, config, &mut rng);

    // Run growth iterations
    for iteration in 0..config.iterations {
        // Calculate target energy based on iteration (increases over time)
        let base_energy = 1.0 + (iteration as f32).powf(1.5) * 0.2;

        // Calculate vigor ratios
        calculate_vigor_ratios(&mut trunk, config.apical_dominance);

        // Distribute vigor from root
        distribute_vigor(&mut trunk, base_energy);

        // Apply growth rules
        apply_growth_rules(&mut trunk, config, &mut rng);

        // Apply gravity
        apply_gravity_to_growth(&mut trunk, config);
    }

    trunk
}

/// Convert growth tree to branch segments for mesh generation
pub fn convert_growth_to_branches(
    node: &GrowthNode,
    trunk_height: f32,
    depth: u8,
) -> Vec<BranchSegment> {
    let mut branches = Vec::new();

    // Only add renderable nodes
    if node.is_renderable() && node.length > 0.01 {
        let _end_pos = node.end_position();
        let height_ratio = (node.position.y / trunk_height).clamp(0.0, 1.0);

        // Determine if terminal (no renderable children)
        let is_terminal = node.children.is_empty()
            || node
                .children
                .iter()
                .all(|c| !c.is_renderable() || c.length < 0.01);

        branches.push(BranchSegment {
            start: node.position,
            direction: node.direction,
            length: node.length,
            base_radius: node.radius,
            tip_radius: node.radius * 0.7,
            depth,
            is_terminal,
            height_ratio,
        });

        // For flower nodes, we could mark them specially for foliage generation
        // (This is handled by is_terminal for now)
    }

    // Recurse to children
    for child in &node.children {
        let child_depth = if node.info.node_type == GrowthNodeType::Ignored {
            0 // Children of trunk start at depth 0
        } else {
            depth + 1
        };
        branches.extend(convert_growth_to_branches(child, trunk_height, child_depth));
    }

    branches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_growth_node_creation() {
        let node = GrowthNode::new(
            Vector3::ZERO,
            Vector3::UP,
            1.0,
            0.5,
            GrowthNodeType::Meristem,
        );
        assert_eq!(node.info.node_type, GrowthNodeType::Meristem);
        assert!(node.is_renderable());
        assert_eq!(node.end_position(), Vector3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_trunk_structure_creation() {
        let config = GrowthConfig {
            trunk_height: 5.0,
            trunk_radius: 0.5,
            ..Default::default()
        };
        let trunk = create_trunk_structure(&config);

        assert_eq!(trunk.info.node_type, GrowthNodeType::Ignored);
        assert_eq!(trunk.length, 5.0);
        assert_eq!(trunk.children.len(), 1);
        assert_eq!(trunk.children[0].info.node_type, GrowthNodeType::Meristem);
    }

    #[test]
    fn test_vigor_distribution() {
        let config = GrowthConfig::default();
        let mut trunk = create_trunk_structure(&config);

        calculate_vigor_ratios(&mut trunk, config.apical_dominance);
        distribute_vigor(&mut trunk, 1.0);

        // Trunk should have vigor
        assert!(trunk.info.vigor > 0.0);
        // Children should have vigor
        assert!(trunk.children[0].info.vigor > 0.0);
    }

    #[test]
    fn test_growth_simulation() {
        let config = GrowthConfig {
            iterations: 3,
            grow_threshold: 0.2,
            ..Default::default()
        };
        let trunk = create_trunk_structure(&config);
        let grown = simulate_growth(trunk, &config);

        // Should have more structure after growth
        assert!(count_nodes(&grown) > 2);
    }

    #[test]
    fn test_lateral_buds() {
        let config = GrowthConfig {
            enable_lateral: true,
            lateral_density: 2.0,
            trunk_height: 5.0,
            iterations: 5,
            lateral_activation: 0.2, // Lower threshold to activate buds
            grow_threshold: 0.2,
            ..Default::default()
        };
        let trunk = create_trunk_structure(&config);
        let grown = simulate_growth(trunk, &config);

        // Should have more nodes than initial structure
        let total_nodes = count_nodes(&grown);
        // Initial trunk has 2 nodes (trunk + apical meristem)
        // After growth with lateral buds, should have more
        assert!(
            total_nodes > 3,
            "Expected more than 3 nodes, got {}",
            total_nodes
        );
    }

    #[test]
    fn test_convert_to_branches() {
        let config = GrowthConfig {
            iterations: 2,
            ..Default::default()
        };
        let trunk = create_trunk_structure(&config);
        let grown = simulate_growth(trunk, &config);
        let branches = convert_growth_to_branches(&grown, config.trunk_height, 0);

        assert!(!branches.is_empty());
    }

    fn count_nodes(node: &GrowthNode) -> usize {
        1 + node.children.iter().map(count_nodes).sum::<usize>()
    }
}
