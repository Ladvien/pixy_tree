//! Manifold Mesher — watertight mesh generation with proper branch junctions.
//!
//! Ports the C++ ManifoldMesher from modular_tree. Instead of generating separate
//! cylinders per branch segment, this mesher creates a single continuous mesh with
//! proper topology at branch attachment points (no overlapping geometry).
//!
//! The key data structures are:
//! - `BranchNode`: a tree graph node with children (continuation + side branches)
//! - `BranchNodeChild`: wraps a `BranchNode` with `position_in_parent`
//! - The mesher traverses this tree recursively, placing radial vertex circles
//!   and cutting holes where side branches attach.

use godot::prelude::*;
use std::f32::consts::TAU;

use crate::branch::{BranchSegment, MeshData};
use crate::tree::TrunkTermination;

// ════════════════════════════════════════════════════════════
// Trunk Configuration for Unified Mesh
// ════════════════════════════════════════════════════════════

/// Configuration for creating a trunk as a BranchNode chain.
/// Used by the unified trunk-branch manifold meshing path.
#[derive(Clone, Debug)]
pub struct TrunkNodeConfig {
    pub height: f32,
    pub base_radius: f32,
    pub tip_radius: f32,
    pub taper_curve: f32,
    pub height_segments: usize,
    pub trunk_randomness: f32,
    pub trunk_twist: f32,
    pub seed: i32,
    /// Root flare settings
    pub root_flare_count: i32,
    pub root_flare_spread: f32,
    pub root_flare_height: f32,
    /// Trunk termination mode
    pub termination: TrunkTermination,
    /// Leader branch (if termination == LeaderBranch)
    pub leader_segment: Option<BranchSegment>,
}

/// Helper: compute trunk wobble offset at a given height.
/// Must match tree.rs trunk_center_at_height formula.
fn trunk_wobble_at_height(height: f32, config: &TrunkNodeConfig) -> (f32, f32) {
    let trunk_height = config.height.max(0.001);
    let t = height / trunk_height;
    let seed_f = config.seed as f32;

    let wobble_x = if config.trunk_randomness > 0.0 {
        (t * std::f32::consts::PI + seed_f * 0.1).sin()
            * config.trunk_randomness
            * t
            * trunk_height
            * 0.3
    } else {
        0.0
    };
    let wobble_z = if config.trunk_randomness > 0.0 {
        (t * std::f32::consts::E + seed_f * 0.2).cos()
            * config.trunk_randomness
            * t
            * trunk_height
            * 0.3
    } else {
        0.0
    };

    (wobble_x, wobble_z)
}

/// Helper: interpolate trunk radius at height t (0-1), including root flare.
fn trunk_radius_at_t(t: f32, angle: f32, config: &TrunkNodeConfig) -> f32 {
    // Base taper interpolation
    let taper_factor = t.powf(config.taper_curve);
    let nominal_radius =
        config.base_radius + (config.tip_radius - config.base_radius) * taper_factor;

    // Root flare bulge
    let root_height_ratio = config.root_flare_height;
    let has_root_flares = config.root_flare_count > 0 && config.root_flare_spread > 0.0;

    if !has_root_flares || t >= root_height_ratio {
        return nominal_radius;
    }

    let root_blend = 1.0 - (t / root_height_ratio);
    let bulge_angle = angle * config.root_flare_count as f32;
    let raw_bulge = (bulge_angle.cos() * 0.5 + 0.5).powi(2);
    let root_bulge = raw_bulge * config.root_flare_spread * config.base_radius * root_blend;

    nominal_radius + root_bulge
}

/// Create a trunk as a chain of BranchNodes for manifold meshing.
///
/// Each height segment becomes a BranchNode linked by continuation children.
/// Segments are created at branch attachment heights for proper junctions.
/// The trunk direction includes wobble perturbation between segments.
pub fn create_trunk_node_chain_with_branch_heights(
    config: &TrunkNodeConfig,
    branch_heights: &[f32],
) -> BranchNode {
    // Create segments that end at branch attachment heights
    // This ensures branches attach at position_in_parent close to 1.0
    let mut heights: Vec<f32> = vec![0.0];

    // Add branch attachment heights
    for &h in branch_heights {
        if h > 0.001 && h < config.height - 0.001 {
            heights.push(h);
        }
    }

    // Add final height
    heights.push(config.height);

    // Sort and deduplicate
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap());
    heights.dedup_by(|a, b| (*a - *b).abs() < 0.01);

    // Add intermediate segments for visual quality
    // This ensures trunk segments are short enough for good junction geometry
    let min_segment_height = config.height / config.height_segments.max(1) as f32;
    let mut i = 0;
    while i < heights.len() - 1 {
        let gap = heights[i + 1] - heights[i];
        if gap > min_segment_height * 1.5 {
            // Add intermediate point
            let mid = heights[i] + min_segment_height;
            if mid < heights[i + 1] - 0.01 {
                heights.insert(i + 1, mid);
                // Don't increment i - we need to check the new gap
                continue;
            }
        }
        i += 1;
    }

    // Remove duplicates and sort
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap());
    heights.dedup_by(|a, b| (*a - *b).abs() < 0.01);

    let segments = heights.len() - 1;
    if segments == 0 {
        // Degenerate case: single segment
        return create_single_trunk_node(config);
    }

    // Build from bottom to top
    let mut nodes: Vec<BranchNode> = Vec::with_capacity(segments);

    for seg in 0..segments {
        let height_start = heights[seg];
        let height_end = heights[seg + 1];
        let t_start = height_start / config.height;
        let t_end = height_end / config.height;

        // Wobble at start and end of segment
        let (wobble_x_start, wobble_z_start) = trunk_wobble_at_height(height_start, config);
        let (wobble_x_end, wobble_z_end) = trunk_wobble_at_height(height_end, config);

        // Direction: from segment start to segment end (accounts for wobble)
        let start_pos = Vector3::new(wobble_x_start, height_start, wobble_z_start);
        let end_pos = Vector3::new(wobble_x_end, height_end, wobble_z_end);
        let segment_dir = (end_pos - start_pos).normalized();
        let actual_length = (end_pos - start_pos).length();

        // Use average direction including wobble
        let direction = if segment_dir.length_squared() > 0.01 {
            segment_dir
        } else {
            Vector3::UP
        };

        // Apply trunk twist to tangent
        let twist_angle = t_start * config.trunk_twist.to_radians();
        let tangent = Vector3::new(twist_angle.cos(), 0.0, twist_angle.sin());

        // Radius at segment start (average over angles for root flare)
        let avg_radius = (trunk_radius_at_t(t_start, 0.0, config)
            + trunk_radius_at_t(t_start, std::f32::consts::FRAC_PI_2, config)
            + trunk_radius_at_t(t_start, std::f32::consts::PI, config)
            + trunk_radius_at_t(t_start, 3.0 * std::f32::consts::FRAC_PI_2, config))
            / 4.0;

        nodes.push(BranchNode {
            direction,
            tangent,
            length: actual_length.max(0.01),
            radius: avg_radius,
            depth: 0,
            height_ratio: t_start,
            children: vec![],
        });
    }

    // Handle trunk termination for the top node
    let top_node_idx = nodes.len() - 1;

    match config.termination {
        TrunkTermination::PointedTip => {
            nodes[top_node_idx].radius = config.tip_radius * 0.1;
        }
        TrunkTermination::LeaderBranch => {}
        TrunkTermination::FlatCap => {}
    }

    // Link segments: each node's continuation child is the next node
    let mut current = nodes.pop().unwrap();

    // Add leader as continuation child if enabled
    if config.termination == TrunkTermination::LeaderBranch {
        if let Some(leader_seg) = &config.leader_segment {
            let leader_node = BranchNode {
                direction: leader_seg.direction,
                tangent: get_perpendicular(leader_seg.direction),
                length: leader_seg.length,
                radius: leader_seg.base_radius,
                depth: 0,
                height_ratio: 1.0,
                children: vec![],
            };
            current.children.insert(
                0,
                BranchNodeChild {
                    node: leader_node,
                    position_in_parent: 1.0,
                },
            );
        }
    }

    while let Some(mut parent) = nodes.pop() {
        parent.children.insert(
            0,
            BranchNodeChild {
                node: current,
                position_in_parent: 1.0,
            },
        );
        current = parent;
    }

    current
}

/// Create a single trunk node (degenerate case).
fn create_single_trunk_node(config: &TrunkNodeConfig) -> BranchNode {
    let (wobble_x, wobble_z) = trunk_wobble_at_height(config.height * 0.5, config);
    let direction =
        Vector3::new(wobble_x / config.height, 1.0, wobble_z / config.height).normalized();

    BranchNode {
        direction,
        tangent: Vector3::RIGHT,
        length: config.height,
        radius: config.base_radius,
        depth: 0,
        height_ratio: 0.0,
        children: vec![],
    }
}

/// Create a trunk as a chain of BranchNodes for manifold meshing (legacy fixed segments).
pub fn create_trunk_node_chain(config: &TrunkNodeConfig) -> BranchNode {
    // Use empty branch heights for backward compatibility
    create_trunk_node_chain_with_branch_heights(config, &[])
}

/// Attach branch segments to a trunk node chain created with branch heights.
///
/// For each depth-0 branch, finds the trunk segment that ends at the branch's height,
/// and attaches the branch at position_in_parent = 1.0 (end of segment).
/// Sub-branches are attached to their respective parent branches via segments_to_tree.
pub fn attach_branches_to_trunk(
    trunk: &mut BranchNode,
    branches: &[BranchSegment],
    config: &TrunkNodeConfig,
) {
    if branches.is_empty() {
        return;
    }

    // Build list of segment end heights (for finding correct segment)
    let segment_end_heights = collect_segment_end_heights(trunk, config.height);

    // Convert ALL branches to tree structure at once.
    // segments_to_tree uses spatial proximity to detect parent-child relationships,
    // producing root-level nodes (depth-0 primaries with their subtrees).
    let branch_trees = segments_to_tree(branches);

    // Attach each root-level branch tree to the appropriate trunk segment
    for branch_node in branch_trees {
        // Find the trunk segment for this branch's start height
        // We need to find the original segment's start position.
        // The branch_node's position is determined by where it attaches to the trunk.
        // Use the branch's direction and length to estimate start height.
        // For depth-0 branches, we can find matching segment by checking all depth-0 segments.
        let branch_height = find_branch_start_height(&branch_node, branches);

        let (segment_idx, position_in_parent) =
            find_segment_for_height(branch_height, &segment_end_heights);

        // Navigate to the correct trunk segment
        let trunk_node = get_trunk_node_at_depth(trunk, segment_idx);

        // Add branch as side child
        trunk_node.children.push(BranchNodeChild {
            node: branch_node,
            position_in_parent,
        });
    }
}

/// Find the start height (Y position) of a branch node by matching it back to the original segments.
fn find_branch_start_height(node: &BranchNode, segments: &[BranchSegment]) -> f32 {
    // Match by direction and radius (the root of each tree from segments_to_tree)
    let epsilon = 0.001;
    for seg in segments {
        if seg.depth == 0
            && (seg.base_radius - node.radius).abs() < epsilon
            && (seg.direction - node.direction).length() < epsilon
        {
            return seg.start.y;
        }
    }
    // Fallback: use the midpoint (shouldn't happen)
    0.0
}

/// Collect the end heights of each trunk segment.
fn collect_segment_end_heights(trunk: &BranchNode, total_height: f32) -> Vec<f32> {
    let mut heights = Vec::new();
    let mut current = trunk;
    let mut accumulated_height = 0.0;

    loop {
        accumulated_height += current.length;
        heights.push(accumulated_height.min(total_height));

        // Follow continuation child
        if current.children.is_empty() {
            break;
        }
        current = &current.children[0].node;
    }

    heights
}

/// Find the best trunk segment index and position_in_parent for a given branch height.
fn find_segment_for_height(branch_height: f32, segment_end_heights: &[f32]) -> (usize, f32) {
    // Find segment whose end is closest to (but >= ) branch_height
    for (i, &end_height) in segment_end_heights.iter().enumerate() {
        let start_height = if i == 0 {
            0.0
        } else {
            segment_end_heights[i - 1]
        };

        if branch_height >= start_height && branch_height <= end_height + 0.001 {
            // Branch is within this segment
            let segment_length = end_height - start_height;
            let position = if segment_length > 0.001 {
                ((branch_height - start_height) / segment_length).clamp(0.0, 1.0)
            } else {
                1.0
            };
            return (i, position);
        }
    }

    // Fallback: attach to last segment
    (segment_end_heights.len().saturating_sub(1), 1.0)
}

/// Navigate down the trunk chain to find the node at a given segment index.
/// Returns a mutable reference to that node.
fn get_trunk_node_at_depth(trunk: &mut BranchNode, depth: usize) -> &mut BranchNode {
    if depth == 0 {
        return trunk;
    }

    // The continuation child is always at index 0
    if trunk.children.is_empty() {
        return trunk;
    }

    get_trunk_node_at_depth(&mut trunk.children[0].node, depth - 1)
}

// ════════════════════════════════════════════════════════════
// Tree Graph Types
// ════════════════════════════════════════════════════════════

/// A node in the branch tree graph for manifold meshing.
/// `children[0]` is always the continuation; `children[1..]` are side branches.
#[derive(Clone, Debug)]
pub struct BranchNode {
    pub direction: Vector3,
    pub tangent: Vector3,
    pub length: f32,
    pub radius: f32,
    #[allow(dead_code)]
    pub depth: u8,
    #[allow(dead_code)]
    pub height_ratio: f32,
    pub children: Vec<BranchNodeChild>,
}

/// A child attachment on a parent BranchNode.
#[derive(Clone, Debug)]
pub struct BranchNodeChild {
    pub node: BranchNode,
    /// Where along the parent this child attaches (0.0-1.0)
    pub position_in_parent: f32,
}

impl BranchNode {
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

// ════════════════════════════════════════════════════════════
// Internal Mesher Types
// ════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
struct CircleDesignator {
    vertex_index: usize,
    #[allow(dead_code)] // Used for UV offset tracking, read indirectly via struct patterns
    uv_index: usize,
    radial_n: usize,
}

#[derive(Clone, Copy, Debug)]
struct IndexRange {
    min_index: i32,
    max_index: i32,
}

/// Pivot Painter 2.0 per-vertex attributes
#[derive(Clone, Debug)]
pub struct PivotPainterContext {
    pub stem_id: i32,
    pub hierarchy_depth: i32,
    pub pivot_position: Vector3,
    pub branch_extent: f32,
}

/// Extended mesh data that also collects pivot painter attributes
#[derive(Default)]
pub struct ManifoldMeshData {
    pub mesh: MeshData,
    /// Per-vertex: [stem_id, hierarchy_depth, branch_extent, 0.0]
    pub custom0: Vec<[f32; 4]>,
    /// Per-vertex: [pivot_x, pivot_y, pivot_z, 0.0]
    pub custom1: Vec<[f32; 4]>,
    /// Per-vertex branch direction (for direction-based shaders)
    pub directions: Vec<Vector3>,
    /// L4: Per-vertex radius (for radius-based shaders)
    pub radii: Vec<f32>,
    /// Separate UV tracking (ManifoldMesher generates UVs separately from vertices)
    uv_list: Vec<Vector2>,
}

impl ManifoldMeshData {
    fn new() -> Self {
        Self::default()
    }

    /// Add a vertex and return its index
    fn add_vertex(&mut self, position: Vector3) -> usize {
        let idx = self.mesh.vertices.len();
        self.mesh.vertices.push(position);
        self.mesh.normals.push(Vector3::ZERO); // Will be recalculated
        self.mesh.smooth_weights.push(1.0);
        self.custom0.push([0.0; 4]);
        self.custom1.push([0.0; 4]);
        self.directions.push(Vector3::UP);
        self.radii.push(0.0); // L4: Default radius, will be set by caller
        idx
    }

    /// Add a vertex with radius and return its index (L4)
    #[allow(dead_code)]
    fn add_vertex_with_radius(&mut self, position: Vector3, radius: f32) -> usize {
        let idx = self.add_vertex(position);
        self.radii[idx] = radius;
        idx
    }

    /// Add a quad (4 vertex indices) as two triangles
    fn add_quad(&mut self, v0: usize, v1: usize, v2: usize, v3: usize) {
        // Quad v0-v1-v2-v3 → triangles v0-v2-v1 and v0-v3-v2
        // Reversed winding for outward-facing normals (Godot CCW = front face)
        self.mesh.indices.push(v0 as i32);
        self.mesh.indices.push(v2 as i32);
        self.mesh.indices.push(v1 as i32);

        self.mesh.indices.push(v0 as i32);
        self.mesh.indices.push(v3 as i32);
        self.mesh.indices.push(v2 as i32);
    }
}

// ════════════════════════════════════════════════════════════
// Segment-to-Tree Conversion
// ════════════════════════════════════════════════════════════

/// Get a perpendicular vector to the given direction
/// Must match branch.rs get_perpendicular for collar-branch alignment
fn get_perpendicular(dir: Vector3) -> Vector3 {
    // Choose a vector that's not parallel to dir
    // If dir is nearly vertical (Y-aligned), use RIGHT; otherwise use UP
    let tmp = if dir.y.abs() > 0.95 {
        Vector3::RIGHT
    } else {
        Vector3::UP
    };
    // Use same cross product order as branch.rs: tmp.cross(dir)
    tmp.cross(dir).normalized()
}

/// Convert a flat list of BranchSegments into a tree of BranchNodes.
/// Uses position matching (same logic as `apply_pipe_radius_model`) to detect parent-child.
/// Returns root-level nodes (stems).
pub fn segments_to_tree(segments: &[BranchSegment]) -> Vec<BranchNode> {
    if segments.is_empty() {
        return vec![];
    }

    let epsilon = 0.01f32;

    // Build parent lookup: for each segment index, which segment is its parent?
    let mut parent_of: Vec<Option<usize>> = vec![None; segments.len()];
    let mut children_of: Vec<Vec<usize>> = vec![Vec::new(); segments.len()];

    // M6 fix: Match children to parents using closest-point-on-segment, not just endpoint
    // This allows children to attach anywhere along the parent segment
    let mut child_position_in_parent: Vec<f32> = vec![1.0; segments.len()];

    // Spatial hash grid for O(n) parent-child matching instead of O(n²).
    // Each segment is inserted into all grid cells it passes through.
    // Child lookup only checks segments in nearby cells.
    let cell_size = 1.0f32;
    let grid_key = |pos: Vector3| -> (i32, i32, i32) {
        (
            (pos.x / cell_size).floor() as i32,
            (pos.y / cell_size).floor() as i32,
            (pos.z / cell_size).floor() as i32,
        )
    };

    // Insert each segment into grid cells along its length
    let mut grid: std::collections::HashMap<(i32, i32, i32), Vec<usize>> =
        std::collections::HashMap::new();
    for (idx, seg) in segments.iter().enumerate() {
        let end = seg.start + seg.direction * seg.length;
        // Number of steps to rasterize segment into cells
        let steps = ((seg.length / cell_size).ceil() as usize).max(1);
        let mut inserted = std::collections::HashSet::new();
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let point = seg.start + (end - seg.start) * t;
            let key = grid_key(point);
            if inserted.insert(key) {
                grid.entry(key).or_default().push(idx);
            }
        }
    }

    for (child_idx, child) in segments.iter().enumerate() {
        let mut best_dist = f32::MAX;
        let mut best_parent = None;
        let mut best_pos = 1.0f32;

        // Only check segments in nearby grid cells (3x3x3 neighborhood)
        let center = grid_key(child.start);
        let mut candidates_checked = std::collections::HashSet::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let key = (center.0 + dx, center.1 + dy, center.2 + dz);
                    if let Some(cell_segments) = grid.get(&key) {
                        for &parent_idx in cell_segments {
                            if parent_idx == child_idx || !candidates_checked.insert(parent_idx) {
                                continue;
                            }
                            let parent = &segments[parent_idx];
                            // Project child.start onto parent segment to find closest point
                            let to_child = child.start - parent.start;
                            let along = to_child.dot(parent.direction) / parent.length;
                            let along_clamped = along.clamp(0.0, 1.0);
                            let closest_point =
                                parent.start + parent.direction * parent.length * along_clamped;
                            let distance = (child.start - closest_point).length();

                            if distance < epsilon && distance < best_dist {
                                best_dist = distance;
                                best_parent = Some(parent_idx);
                                best_pos = along_clamped;
                            }
                        }
                    }
                }
            }
        }

        if let Some(parent_idx) = best_parent {
            parent_of[child_idx] = Some(parent_idx);
            children_of[parent_idx].push(child_idx);
            child_position_in_parent[child_idx] = best_pos;
        }
    }

    // Build nodes bottom-up (leaves first) using topological sort.
    // This ensures children are always constructed before their parents,
    // regardless of depth values (critical for multi-segment continuation chains).
    let mut sorted: Vec<usize> = Vec::with_capacity(segments.len());
    let mut processed = vec![false; segments.len()];
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();

    // Start with leaves (segments that have no children)
    for i in 0..segments.len() {
        if children_of[i].is_empty() {
            queue.push_back(i);
        }
    }
    while let Some(idx) = queue.pop_front() {
        sorted.push(idx);
        processed[idx] = true;
        if let Some(parent) = parent_of[idx] {
            // Add parent to queue once all its children are processed
            if !processed[parent] && children_of[parent].iter().all(|&c| processed[c]) {
                queue.push_back(parent);
            }
        }
    }
    // Add any remaining unprocessed nodes (disconnected roots)
    for i in 0..segments.len() {
        if !processed[i] {
            sorted.push(i);
        }
    }

    let mut nodes: Vec<Option<BranchNode>> = vec![None; segments.len()];

    for &idx in &sorted {
        let seg = &segments[idx];
        let tangent = get_perpendicular(seg.direction);

        let mut children: Vec<BranchNodeChild> = Vec::new();

        // Collect children, putting the continuation child first (closest to parent end direction)
        let child_indices = &children_of[idx];
        if !child_indices.is_empty() {
            // Find continuation child: the one whose direction is most aligned with parent
            let mut best_continuation = 0usize;
            let mut best_dot = -2.0f32;
            for (i, &ci) in child_indices.iter().enumerate() {
                let dot = seg.direction.dot(segments[ci].direction);
                if dot > best_dot {
                    best_dot = dot;
                    best_continuation = i;
                }
            }

            // Add continuation first
            let cont_idx = child_indices[best_continuation];
            if let Some(cont_node) = nodes[cont_idx].take() {
                children.push(BranchNodeChild {
                    node: cont_node,
                    position_in_parent: child_position_in_parent[cont_idx],
                });
            }

            // Add side branches
            for (i, &ci) in child_indices.iter().enumerate() {
                if i == best_continuation {
                    continue;
                }
                if let Some(side_node) = nodes[ci].take() {
                    // M6 fix: Use computed position_in_parent from segment projection
                    children.push(BranchNodeChild {
                        node: side_node,
                        position_in_parent: child_position_in_parent[ci],
                    });
                }
            }
        }

        nodes[idx] = Some(BranchNode {
            direction: seg.direction,
            tangent,
            length: seg.length,
            radius: seg.base_radius,
            depth: seg.depth,
            height_ratio: seg.height_ratio,
            children,
        });
    }

    // Collect root nodes (those with no parent)
    let mut roots = Vec::new();
    for (idx, parent) in parent_of.iter().enumerate() {
        if parent.is_none() {
            if let Some(node) = nodes[idx].take() {
                roots.push(node);
            }
        }
    }

    roots
}

/// Convert a GrowthNode tree directly into a BranchNode tree (already hierarchical).
#[allow(dead_code)]
pub fn growth_node_to_branch_node(node: &crate::growth::GrowthNode) -> Option<BranchNode> {
    if !node.is_renderable() || node.length < 0.01 {
        // Still process children to find renderable subtrees
        let child_nodes: Vec<BranchNode> = node
            .children
            .iter()
            .filter_map(growth_node_to_branch_node)
            .collect();
        if child_nodes.is_empty() {
            return None;
        }
        // If this node isn't renderable but has renderable children, return first child
        // with remaining as its side branches (best effort)
        if child_nodes.len() == 1 {
            return Some(child_nodes.into_iter().next().unwrap());
        }
        let mut first = child_nodes.into_iter();
        let mut root = first.next().unwrap();
        for sibling in first {
            root.children.push(BranchNodeChild {
                node: sibling,
                position_in_parent: 0.0,
            });
        }
        return Some(root);
    }

    let tangent = get_perpendicular(node.direction);

    // Convert children
    let mut children: Vec<BranchNodeChild> = Vec::new();
    for child in &node.children {
        if let Some(child_node) = growth_node_to_branch_node(child) {
            children.push(BranchNodeChild {
                node: child_node,
                position_in_parent: child.position_in_parent,
            });
        }
    }

    // Sort: continuation (highest direction alignment) first
    if children.len() > 1 {
        let parent_dir = node.direction;
        children.sort_by(|a, b| {
            let dot_a = parent_dir.dot(a.node.direction);
            let dot_b = parent_dir.dot(b.node.direction);
            dot_b
                .partial_cmp(&dot_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    Some(BranchNode {
        direction: node.direction,
        tangent,
        length: node.length,
        radius: node.radius,
        depth: 0, // Will be set by caller if needed
        height_ratio: (node.position.y / 5.0).clamp(0.0, 1.0), // Approximate
        children,
    })
}

// ════════════════════════════════════════════════════════════
// Core Mesher Functions (ported from C++ ManifoldMesher)
// ════════════════════════════════════════════════════════════

fn get_smooth_amount(radius: f32, node_length: f32) -> f32 {
    (radius / node_length.max(0.001)).min(1.0)
}

/// Calculate total extent (length) of a branch following continuation children
fn calculate_branch_extent(node: &BranchNode) -> f32 {
    let mut extent = node.length;
    if !node.is_leaf() && !node.children.is_empty() {
        extent += calculate_branch_extent(&node.children[0].node);
    }
    extent
}

/// Get position at a factor along a node
fn get_position_in_node(node_position: Vector3, node: &BranchNode, factor: f32) -> Vector3 {
    node_position + node.direction * node.length * factor
}

/// Create a ring of vertices around a cross-section of the branch.
fn add_circle(
    node_position: Vector3,
    node: &BranchNode,
    factor: f32,
    radial_n: usize,
    mesh: &mut ManifoldMeshData,
    uv_y: f32,
    pp_ctx: &PivotPainterContext,
    _pivot_painter_enabled: bool,
) -> CircleDesignator {
    let right = node.tangent;
    let vertex_index = mesh.mesh.vertices.len();
    let uv_index = mesh.uv_list.len();
    let up = node.tangent.cross(node.direction);
    let circle_position = node_position + node.length * factor * node.direction;

    // Interpolate radius along node
    let radius = if node.is_leaf() {
        node.radius
    } else {
        let child_radius = node.children[0].node.radius;
        node.radius + (child_radius - node.radius) * factor
    };

    let smooth_amount = get_smooth_amount(radius, node.length);

    // Generate radial_n + 1 vertices: duplicate vertex 0 at the end for UV seam
    for i in 0..=radial_n {
        let angle = (i % radial_n) as f32 / radial_n as f32 * TAU;
        let point = right * angle.cos() + up * angle.sin();
        let vertex = point * radius + circle_position;

        let idx = mesh.add_vertex(vertex);
        mesh.mesh.smooth_weights[idx] = smooth_amount;
        mesh.radii[idx] = radius; // L4: Store per-vertex radius

        // C++ always assigns pivot painter attributes unconditionally
        mesh.custom0[idx] = [
            pp_ctx.stem_id as f32,
            pp_ctx.hierarchy_depth as f32,
            pp_ctx.branch_extent,
            0.0,
        ];
        mesh.custom1[idx] = [
            pp_ctx.pivot_position.x,
            pp_ctx.pivot_position.y,
            pp_ctx.pivot_position.z,
            0.0,
        ];
        mesh.directions[idx] = node.direction;

        // UV: seam vertex (i == radial_n) gets u=1.0
        let uv_x = i as f32 / radial_n as f32;
        mesh.uv_list.push(Vector2::new(uv_x, uv_y));
    }

    CircleDesignator {
        vertex_index,
        uv_index,
        radial_n,
    }
}

fn is_index_in_branch_mask(mask: &[IndexRange], index: i32, radial_n: i32) -> bool {
    let offset = radial_n / 2;
    for range in mask {
        let mut i = index;
        let mut min_idx = range.min_index;
        let mut max_idx = range.max_index;
        if max_idx < min_idx {
            i = (i + offset) % radial_n;
            min_idx = (min_idx + offset) % radial_n;
            max_idx = (max_idx + offset) % radial_n;
        }
        if i >= min_idx && i < max_idx {
            return true;
        }
    }
    false
}

/// Connect two vertex rings with quads, optionally skipping masked regions where branches attach.
fn bridge_circles(
    first: &CircleDesignator,
    second: &CircleDesignator,
    radial_n: usize,
    mesh: &mut ManifoldMeshData,
    mask: Option<&[IndexRange]>,
) {
    for i in 0..radial_n as i32 {
        if let Some(m) = mask {
            if is_index_in_branch_mask(m, i, radial_n as i32) {
                continue;
            }
        }
        // With radial_n+1 vertices per circle, seam vertex exists at index radial_n
        let v0 = first.vertex_index + i as usize;
        let v1 = first.vertex_index + (i + 1) as usize;
        let v2 = second.vertex_index + (i + 1) as usize;
        let v3 = second.vertex_index + i as usize;
        mesh.add_quad(v0, v1, v2, v3);
    }
}

/// Project branch direction onto plane perpendicular to parent, return angle via atan2.
fn get_branch_angle_around_parent(parent: &BranchNode, branch: &BranchNode) -> f32 {
    // Project branch direction onto plane perpendicular to parent direction
    let dot = branch.direction.dot(parent.direction);
    let projected = (branch.direction - parent.direction * dot).normalized();

    let right = parent.tangent;
    let up = right.cross(parent.direction);
    let cos_angle = projected.dot(right);
    let sin_angle = projected.dot(up);
    (sin_angle.atan2(cos_angle) + TAU) % TAU
}

/// Map child branch angular footprint to radial index range on parent circle.
fn get_branch_indices_on_circle(
    radial_n: usize,
    circle_radius: f32,
    branch_radius: f32,
    branch_angle: f32,
) -> IndexRange {
    let angle_delta = (branch_radius / circle_radius).clamp(-1.0, 1.0).asin();
    let increment = TAU / radial_n as f32;
    let min_index = ((branch_angle - angle_delta + TAU) % TAU / increment) as i32;
    let max_index = ((branch_angle + angle_delta + increment + TAU) % TAU / increment) as i32;
    IndexRange {
        min_index,
        max_index,
    }
}

/// For each side branch child, compute its IndexRange on the parent circle.
fn get_children_ranges(node: &BranchNode, radial_n: usize) -> Vec<IndexRange> {
    let mut ranges = Vec::new();
    for i in 1..node.children.len() {
        let child = &node.children[i].node;
        let angle = get_branch_angle_around_parent(node, child);
        let range = get_branch_indices_on_circle(radial_n, node.radius, child.radius, angle);
        ranges.push(range);
    }
    ranges
}

/// Map parent circle vertices to child circle vertices at junction.
fn get_child_index_order(
    parent_base: &CircleDesignator,
    child_radial_n: usize,
    child_range: &IndexRange,
) -> Vec<usize> {
    let mut child_base_indices = vec![0usize; child_radial_n];
    // Stride is radial_n + 1 due to duplicated seam vertex
    let stride = parent_base.radial_n + 1;

    for i in 0..(child_radial_n / 2) {
        let lower_index = ((child_range.min_index as usize + i) % parent_base.radial_n)
            + parent_base.vertex_index;
        let upper_index = lower_index + stride;

        child_base_indices[i] = lower_index;
        child_base_indices[child_radial_n - i - 1] = upper_index;
    }
    child_base_indices
}

/// Create junction quads connecting parent circle to child circle.
fn add_child_base_geometry(
    child_base_indices: &[usize],
    child_base: &CircleDesignator,
    child_node: &BranchNode,
    child_pos: Vector3,
    offset: usize,
    smooth_amount: f32,
    mesh: &mut ManifoldMeshData,
    child_uvs: &[Vector2],
    pp_ctx: &PivotPainterContext,
    _pivot_painter_enabled: bool,
) {
    let radial_n = child_base.radial_n;
    let child_radius = child_node.radius;

    // Use child node's tangent for basis vectors — must match add_circle() exactly
    // so that bridge_circles() between child_base and subsequent circles has
    // consistent vertex winding (outward-facing normals).
    let right = child_node.tangent;
    let up = right.cross(child_node.direction);

    // Generate radial_n + 1 vertices (seam duplicate at end) matching add_circle convention.
    // Vertices are placed at standard angles (0, TAU/n, 2*TAU/n, ...) so that
    // bridge_circles to subsequent add_circle rings produces aligned quads.
    for i in 0..=radial_n {
        let vi = i % radial_n;
        let angle = vi as f32 / radial_n as f32 * TAU;
        let point = right * angle.cos() + up * angle.sin();
        let vertex = point * child_radius + child_pos;
        let added_idx = mesh.add_vertex(vertex);
        mesh.mesh.smooth_weights[added_idx] = smooth_amount;
        mesh.radii[added_idx] = child_radius;

        // UV: seam vertex (i == radial_n) gets u=1.0
        let uv_x = i as f32 / radial_n as f32;
        let uv_y_val = if vi < child_uvs.len() {
            child_uvs[vi].y
        } else {
            0.0
        };
        mesh.uv_list.push(Vector2::new(uv_x, uv_y_val));

        mesh.custom0[added_idx] = [
            pp_ctx.stem_id as f32,
            pp_ctx.hierarchy_depth as f32,
            pp_ctx.branch_extent,
            0.0,
        ];
        mesh.custom1[added_idx] = [
            pp_ctx.pivot_position.x,
            pp_ctx.pivot_position.y,
            pp_ctx.pivot_position.z,
            0.0,
        ];
        mesh.directions[added_idx] = child_node.direction;
    }

    // Create junction quads connecting parent hole to child base ring.
    // Use offset to map between parent polygon ordering and child ring ordering.
    for i in 0..radial_n {
        let parent_idx = (i + offset) % radial_n;
        let v0 = child_base_indices[parent_idx];
        let v1 = child_base_indices[(parent_idx + 1) % radial_n];
        let v2 = child_base.vertex_index + (i + 1); // uses seam vertex for last quad
        let v3 = child_base.vertex_index + i;
        mesh.add_quad(v0, v1, v2, v3);
    }
}

/// Get twist angle of child relative to parent
fn get_child_twist(child: &BranchNode, parent: &BranchNode) -> f32 {
    // Project parent direction onto plane perpendicular to child direction
    let dot = parent.direction.dot(child.direction);
    let projected = (parent.direction - child.direction * dot).normalized();

    let right = projected;
    let up = right.cross(child.direction);
    let cos_angle = child.tangent.dot(right);
    let sin_angle = child.tangent.dot(up);
    (sin_angle.atan2(cos_angle) + TAU) % TAU
}

/// H5: Compute UVs for child base vertices at junction.
///
/// Godot requires 1:1 vertex-to-UV mapping, so we can't use C++'s per-polygon UV loops.
/// This improved version computes UVs based on the junction's position in UV space,
/// accounting for the parent's UV growth rate.
///
/// Parameters:
/// - parent_uv_y: Current V coordinate on the parent branch
/// - child_radial_n: Number of radial segments on child
/// - parent_radius: Parent branch radius at junction (for UV scale)
/// - child_radius: Child branch radius (for UV offset)
fn compute_child_base_uvs(
    parent_uv_y: f32,
    child_radial_n: usize,
    parent_radius: f32,
    child_radius: f32,
) -> Vec<Vector2> {
    let mut uvs = Vec::with_capacity(child_radial_n);

    // H5: Compute UV growth factor based on circumference ratio
    // This creates a smoother UV transition at the junction
    let uv_growth = child_radius / (parent_radius + 0.001) / std::f32::consts::TAU;
    let base_v = parent_uv_y + uv_growth * 0.5; // Offset by half the UV growth

    for i in 0..child_radial_n {
        let u = i as f32 / child_radial_n as f32;
        uvs.push(Vector2::new(u, base_v));
    }
    uvs
}

/// Orchestrate junction creation for a side branch child.
///
/// This matches the C++ ManifoldMesher approach:
/// 1. Get parent junction polygon vertices (child_base_indices)
/// 2. Project them to form a circle at child_pos with child_radius
/// 3. Create quads connecting parent junction to child base
///
/// The collar "bulge" effect comes from mesh smoothing, not explicit geometry.
fn add_child_circle(
    parent: &BranchNode,
    child: &BranchNodeChild,
    child_pos: Vector3,
    _parent_pos: Vector3,
    parent_base: &CircleDesignator,
    child_range: &IndexRange,
    uv_y: f32,
    mesh: &mut ManifoldMeshData,
    pp_ctx: &PivotPainterContext,
    pivot_painter_enabled: bool,
) -> CircleDesignator {
    let smooth_amount = get_smooth_amount(child.node.radius, parent.length);

    // Number of vertices in child circle (C++ formula)
    let child_radial_n = 2
        * (((child_range.max_index - child_range.min_index + parent_base.radial_n as i32)
            % parent_base.radial_n as i32
            + 1) as usize);
    let child_radial_n = child_radial_n.max(4); // Minimum 4 vertices

    // Get parent junction polygon vertex indices
    let child_base_indices = get_child_index_order(parent_base, child_radial_n, child_range);

    // Compute twist offset for proper vertex alignment
    let child_twist = get_child_twist(&child.node, parent);
    let offset = ((child_twist / TAU * child_radial_n as f32 - child_radial_n as f32 / 4.0
        + child_radial_n as f32) as usize)
        % child_radial_n;

    // H5: Compute child base UVs
    let child_uvs = compute_child_base_uvs(uv_y, child_radial_n, parent.radius, child.node.radius);

    // Create the child base circle designator
    let child_base = CircleDesignator {
        vertex_index: mesh.mesh.vertices.len(),
        uv_index: mesh.uv_list.len(),
        radial_n: child_radial_n,
    };

    // Create junction geometry: child base ring + bridging quads to parent hole
    add_child_base_geometry(
        &child_base_indices,
        &child_base,
        &child.node,
        child_pos,
        offset,
        smooth_amount,
        mesh,
        &child_uvs,
        pp_ctx,
        pivot_painter_enabled,
    );

    child_base
}

/// Get position where a side branch attaches to parent surface
fn get_side_child_position(
    parent: &BranchNode,
    child: &BranchNodeChild,
    node_position: Vector3,
) -> Vector3 {
    // Project child direction onto plane perpendicular to parent
    let dot = child.node.direction.dot(parent.direction);
    let tangent = (child.node.direction - parent.direction * dot).normalized();

    // Interpolate parent radius at attachment point (not at segment start)
    let interpolated_radius = if parent.is_leaf() || parent.children.is_empty() {
        parent.radius
    } else {
        let child_radius = parent.children[0].node.radius;
        parent.radius + (child_radius - parent.radius) * child.position_in_parent
    };

    node_position
        + parent.direction * parent.length * child.position_in_parent
        + tangent * interpolated_radius
}

/// Recursive traversal to build the manifold mesh.
fn mesh_node_rec(
    node: &BranchNode,
    node_position: Vector3,
    base: &CircleDesignator,
    mesh: &mut ManifoldMeshData,
    uv_y: f32,
    pp_ctx: &PivotPainterContext,
    stem_id_counter: &mut i32,
    radial_resolution: usize,
    pivot_painter_enabled: bool,
) {
    let uv_growth = node.length / (node.radius + 0.001) / TAU;

    if node.children.len() < 2 {
        // 0-1 children: simple bridge from base to end circle, recurse continuation
        let child_circle = add_circle(
            node_position,
            node,
            1.0,
            base.radial_n,
            mesh,
            uv_y + uv_growth,
            pp_ctx,
            pivot_painter_enabled,
        );
        bridge_circles(base, &child_circle, base.radial_n, mesh, None);

        let child_pos = get_position_in_node(node_position, node, 1.0);

        if !node.is_leaf() {
            mesh_node_rec(
                &node.children[0].node,
                child_pos,
                &child_circle,
                mesh,
                uv_y + uv_growth,
                pp_ctx,
                stem_id_counter,
                radial_resolution,
                pivot_painter_enabled,
            );
        }
    } else {
        // 2+ children: bridge with masks, create junction geometry for side branches
        let end_circle = add_circle(
            node_position,
            node,
            1.0,
            base.radial_n,
            mesh,
            uv_y + uv_growth,
            pp_ctx,
            pivot_painter_enabled,
        );
        let children_ranges = get_children_ranges(node, base.radial_n);
        bridge_circles(
            base,
            &end_circle,
            base.radial_n,
            mesh,
            Some(&children_ranges),
        );

        for i in 0..node.children.len() {
            if i == 0 {
                // Continuation child
                let child_pos = get_position_in_node(node_position, node, 1.0);
                mesh_node_rec(
                    &node.children[0].node,
                    child_pos,
                    &end_circle,
                    mesh,
                    uv_y + uv_growth,
                    pp_ctx,
                    stem_id_counter,
                    radial_resolution,
                    pivot_painter_enabled,
                );
            } else {
                // Side branch
                let child = &node.children[i];
                let child_pos = get_side_child_position(node, child, node_position);

                // Create new Pivot Painter context for side branch
                let child_pp_ctx = PivotPainterContext {
                    stem_id: {
                        *stem_id_counter += 1;
                        *stem_id_counter
                    },
                    hierarchy_depth: pp_ctx.hierarchy_depth + 1,
                    pivot_position: child_pos,
                    branch_extent: calculate_branch_extent(&child.node),
                };

                // Use add_child_circle to create junction geometry connecting
                // parent trunk/branch to child branch base. This creates bridging
                // quads between the parent polygon hole and the child base ring.
                let child_base = add_child_circle(
                    node,
                    child,
                    child_pos,
                    node_position,
                    base,
                    &children_ranges[i - 1],
                    uv_y,
                    mesh,
                    &child_pp_ctx,
                    pivot_painter_enabled,
                );
                mesh_node_rec(
                    &child.node,
                    child_pos,
                    &child_base,
                    mesh,
                    uv_y + uv_growth,
                    &child_pp_ctx,
                    stem_id_counter,
                    radial_resolution,
                    pivot_painter_enabled,
                );
            }
        }
    }
}

// ════════════════════════════════════════════════════════════
// Public API
// ════════════════════════════════════════════════════════════

/// Configuration for manifold meshing
pub struct ManifoldMesherConfig {
    pub radial_resolution: usize,
    pub smooth_iterations: u32,
    pub smooth_factor: f32,
    pub pivot_painter_enabled: bool,
}

impl Default for ManifoldMesherConfig {
    fn default() -> Self {
        Self {
            radial_resolution: 8,
            smooth_iterations: 4,
            smooth_factor: 1.0,
            pivot_painter_enabled: false,
        }
    }
}

/// Generate a manifold mesh from a list of branch tree roots (stems).
///
/// Returns ManifoldMeshData with the mesh and optional pivot painter attributes.
pub fn mesh_tree(
    stems: &[BranchNode],
    stem_positions: &[Vector3],
    config: &ManifoldMesherConfig,
) -> ManifoldMeshData {
    let mut mesh = ManifoldMeshData::new();
    let mut stem_id_counter = 0i32;

    for (stem, &position) in stems.iter().zip(stem_positions.iter()) {
        if stem.children.is_empty() {
            continue;
        }

        let pp_ctx = PivotPainterContext {
            stem_id: {
                let id = stem_id_counter;
                stem_id_counter += 1;
                id
            },
            hierarchy_depth: 0,
            pivot_position: position,
            branch_extent: calculate_branch_extent(stem),
        };

        let start_circle = add_circle(
            position,
            stem,
            0.0,
            config.radial_resolution,
            &mut mesh,
            0.0,
            &pp_ctx,
            config.pivot_painter_enabled,
        );

        mesh_node_rec(
            stem,
            position,
            &start_circle,
            &mut mesh,
            0.0,
            &pp_ctx,
            &mut stem_id_counter,
            config.radial_resolution,
            config.pivot_painter_enabled,
        );
    }

    // Apply smoothing via the existing smoothing module
    if config.smooth_iterations > 0 && !mesh.mesh.vertices.is_empty() {
        crate::smoothing::laplacian_smooth_weighted(
            &mut mesh.mesh.vertices,
            &mesh.mesh.indices,
            &mesh.mesh.smooth_weights,
            config.smooth_iterations,
            config.smooth_factor,
        );
    }

    // Recalculate normals after smoothing
    if !mesh.mesh.vertices.is_empty() {
        crate::smoothing::recalculate_normals(
            &mesh.mesh.vertices,
            &mesh.mesh.indices,
            &mut mesh.mesh.normals,
        );
    }

    // M10 fix: UVs are now pushed inline with vertices (1:1 mapping maintained).
    // Copy uv_list directly; pad or truncate to match vertex count as safety net.
    mesh.mesh.uvs = mesh.uv_list.clone();
    mesh.mesh
        .uvs
        .resize(mesh.mesh.vertices.len(), Vector2::ZERO);

    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segments_to_tree_single() {
        let segments = vec![BranchSegment {
            start: Vector3::ZERO,
            direction: Vector3::UP,
            length: 1.0,
            base_radius: 0.1,
            tip_radius: 0.05,
            depth: 0,
            is_terminal: true,
            height_ratio: 0.5,
            subtree_weight: 1.0,
        }];
        let tree = segments_to_tree(&segments);
        assert_eq!(tree.len(), 1);
        assert!(tree[0].is_leaf());
    }

    #[test]
    fn test_segments_to_tree_parent_child() {
        let segments = vec![
            BranchSegment {
                start: Vector3::ZERO,
                direction: Vector3::UP,
                length: 1.0,
                base_radius: 0.2,
                tip_radius: 0.1,
                depth: 0,
                is_terminal: false,
                height_ratio: 0.5,
                subtree_weight: 1.5,
            },
            BranchSegment {
                start: Vector3::new(0.0, 1.0, 0.0),
                direction: Vector3::new(1.0, 1.0, 0.0).normalized(),
                length: 0.5,
                base_radius: 0.1,
                tip_radius: 0.05,
                depth: 1,
                is_terminal: true,
                height_ratio: 0.8,
                subtree_weight: 0.5,
            },
        ];
        let tree = segments_to_tree(&segments);
        assert_eq!(tree.len(), 1); // One root
        assert_eq!(tree[0].children.len(), 1); // With one child
    }

    #[test]
    fn test_mesh_tree_produces_geometry() {
        let node = BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 1.0,
            radius: 0.1,
            depth: 0,
            height_ratio: 0.5,
            children: vec![BranchNodeChild {
                node: BranchNode {
                    direction: Vector3::UP,
                    tangent: Vector3::RIGHT,
                    length: 0.5,
                    radius: 0.05,
                    depth: 1,
                    height_ratio: 0.8,
                    children: vec![],
                },
                position_in_parent: 1.0,
            }],
        };

        let config = ManifoldMesherConfig {
            radial_resolution: 6,
            smooth_iterations: 0,
            ..Default::default()
        };

        let result = mesh_tree(&[node], &[Vector3::ZERO], &config);
        assert!(!result.mesh.vertices.is_empty());
        assert!(!result.mesh.indices.is_empty());
    }

    #[test]
    fn test_mesh_tree_with_side_branch() {
        let node = BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 1.0,
            radius: 0.2,
            depth: 0,
            height_ratio: 0.5,
            children: vec![
                // Continuation
                BranchNodeChild {
                    node: BranchNode {
                        direction: Vector3::UP,
                        tangent: Vector3::RIGHT,
                        length: 0.5,
                        radius: 0.15,
                        depth: 0,
                        height_ratio: 0.7,
                        children: vec![],
                    },
                    position_in_parent: 1.0,
                },
                // Side branch
                BranchNodeChild {
                    node: BranchNode {
                        direction: Vector3::new(1.0, 0.5, 0.0).normalized(),
                        tangent: Vector3::new(0.0, 0.0, 1.0),
                        length: 0.4,
                        radius: 0.08,
                        depth: 1,
                        height_ratio: 0.6,
                        children: vec![],
                    },
                    position_in_parent: 0.7,
                },
            ],
        };

        let config = ManifoldMesherConfig {
            radial_resolution: 8,
            smooth_iterations: 0,
            ..Default::default()
        };

        let result = mesh_tree(&[node], &[Vector3::ZERO], &config);
        assert!(!result.mesh.vertices.is_empty());
        assert!(!result.mesh.indices.is_empty());
        // Side branch junction should create more geometry than simple bridge
        assert!(result.mesh.vertices.len() > 20);
    }

    #[test]
    fn test_branch_angle_around_parent() {
        let parent = BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 1.0,
            radius: 0.2,
            depth: 0,
            height_ratio: 0.5,
            children: vec![],
        };
        let branch = BranchNode {
            direction: Vector3::RIGHT,
            tangent: Vector3::UP,
            length: 0.5,
            radius: 0.1,
            depth: 1,
            height_ratio: 0.5,
            children: vec![],
        };
        let angle = get_branch_angle_around_parent(&parent, &branch);
        assert!(angle >= 0.0 && angle < TAU);
    }

    #[test]
    fn test_create_trunk_node_chain_basic() {
        let config = TrunkNodeConfig {
            height: 10.0,
            base_radius: 0.5,
            tip_radius: 0.2,
            taper_curve: 0.5,
            height_segments: 4,
            trunk_randomness: 0.0,
            trunk_twist: 0.0,
            seed: 42,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.0,
            termination: TrunkTermination::FlatCap,
            leader_segment: None,
        };

        let trunk = create_trunk_node_chain(&config);

        // Should have continuation children linking 4 segments
        assert!(!trunk.children.is_empty());
        assert_eq!(trunk.children[0].position_in_parent, 1.0); // Continuation at end

        // Count depth of trunk chain
        let mut depth = 1;
        let mut current = &trunk;
        while !current.children.is_empty() {
            current = &current.children[0].node;
            depth += 1;
        }
        assert_eq!(depth, 4); // 4 segments
    }

    #[test]
    fn test_create_trunk_with_leader() {
        let leader = BranchSegment {
            start: Vector3::new(0.0, 10.0, 0.0),
            direction: Vector3::UP,
            length: 3.0,
            base_radius: 0.2,
            tip_radius: 0.05,
            depth: 0,
            is_terminal: true,
            height_ratio: 1.0,
            subtree_weight: 3.0,
        };

        let config = TrunkNodeConfig {
            height: 10.0,
            base_radius: 0.5,
            tip_radius: 0.2,
            taper_curve: 0.5,
            height_segments: 4,
            trunk_randomness: 0.0,
            trunk_twist: 0.0,
            seed: 42,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.0,
            termination: TrunkTermination::LeaderBranch,
            leader_segment: Some(leader),
        };

        let trunk = create_trunk_node_chain(&config);

        // Count depth including leader
        let mut depth = 1;
        let mut current = &trunk;
        while !current.children.is_empty() {
            current = &current.children[0].node;
            depth += 1;
        }
        assert_eq!(depth, 5); // 4 trunk segments + 1 leader
    }

    #[test]
    fn test_attach_branches_to_trunk() {
        let config = TrunkNodeConfig {
            height: 10.0,
            base_radius: 0.5,
            tip_radius: 0.2,
            taper_curve: 0.5,
            height_segments: 4,
            trunk_randomness: 0.0,
            trunk_twist: 0.0,
            seed: 42,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.0,
            termination: TrunkTermination::FlatCap,
            leader_segment: None,
        };

        // Create a branch at height 5.0 (mid-trunk)
        let branches = vec![BranchSegment {
            start: Vector3::new(0.5, 5.0, 0.0),
            direction: Vector3::new(1.0, 0.3, 0.0).normalized(),
            length: 2.0,
            base_radius: 0.1,
            tip_radius: 0.05,
            depth: 0,
            is_terminal: true,
            height_ratio: 0.5,
            subtree_weight: 2.0,
        }];

        // Use branch height to create trunk with proper segment at that height
        let branch_heights: Vec<f32> = branches.iter().map(|b| b.start.y).collect();
        let mut trunk = create_trunk_node_chain_with_branch_heights(&config, &branch_heights);

        attach_branches_to_trunk(&mut trunk, &branches, &config);

        // Find the trunk segment that ends at branch height (5.0)
        // With branch_heights = [5.0], segments are created ending at 5.0
        // So branch should attach to the segment ending at 5.0
        let segment_end_heights = collect_segment_end_heights(&trunk, config.height);
        let (segment_idx, _) = find_segment_for_height(5.0, &segment_end_heights);
        let segment = get_trunk_node_at_depth(&mut trunk, segment_idx);

        // Should have continuation child (index 0) plus side branch
        assert!(
            segment.children.len() >= 2,
            "Expected at least 2 children (continuation + branch), got {}",
            segment.children.len()
        );
    }

    #[test]
    fn test_unified_trunk_branch_mesh() {
        let config = TrunkNodeConfig {
            height: 10.0,
            base_radius: 0.5,
            tip_radius: 0.2,
            taper_curve: 0.5,
            height_segments: 4,
            trunk_randomness: 0.0,
            trunk_twist: 0.0,
            seed: 42,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.0,
            termination: TrunkTermination::FlatCap,
            leader_segment: None,
        };

        let branches = vec![BranchSegment {
            start: Vector3::new(0.5, 5.0, 0.0),
            direction: Vector3::new(1.0, 0.3, 0.0).normalized(),
            length: 2.0,
            base_radius: 0.1,
            tip_radius: 0.05,
            depth: 0,
            is_terminal: true,
            height_ratio: 0.5,
            subtree_weight: 2.0,
        }];

        // Create trunk with segments at branch heights
        let branch_heights: Vec<f32> = branches.iter().map(|b| b.start.y).collect();
        let mut trunk = create_trunk_node_chain_with_branch_heights(&config, &branch_heights);

        attach_branches_to_trunk(&mut trunk, &branches, &config);

        let mesh_config = ManifoldMesherConfig {
            radial_resolution: 8,
            smooth_iterations: 0,
            ..Default::default()
        };

        let result = mesh_tree(&[trunk], &[Vector3::ZERO], &mesh_config);

        // Should produce valid mesh with vertices and indices
        assert!(!result.mesh.vertices.is_empty());
        assert!(!result.mesh.indices.is_empty());
        // Should have more geometry than just trunk (junction geometry)
        assert!(result.mesh.vertices.len() > 50);
    }

    #[test]
    fn test_export_branch_mesh_obj() {
        // Single trunk segment + one large side branch
        let mut trunk = BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 5.0,
            radius: 0.3,
            depth: 0,
            height_ratio: 0.0,
            children: vec![],
        };

        let continuation = BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 5.0,
            radius: 0.2,
            depth: 0,
            height_ratio: 0.5,
            children: vec![],
        };
        trunk.children.push(BranchNodeChild {
            node: continuation,
            position_in_parent: 1.0,
        });

        // Big side branch for visibility
        let branch_dir = Vector3::new(1.0, 0.3, 0.0).normalized();
        let branch = BranchNode {
            direction: branch_dir,
            tangent: get_perpendicular(branch_dir),
            length: 3.0,
            radius: 0.2, // Large radius
            depth: 1,
            height_ratio: 0.5,
            children: vec![],
        };
        trunk.children.push(BranchNodeChild {
            node: branch,
            position_in_parent: 0.8,
        });

        let config = ManifoldMesherConfig {
            radial_resolution: 8,
            smooth_iterations: 0,
            ..Default::default()
        };

        let result = mesh_tree(&[trunk], &[Vector3::ZERO], &config);

        // Export as OBJ for external inspection
        let mut obj = String::new();
        obj.push_str("# Manifold mesh test export\n");
        for v in &result.mesh.vertices {
            obj.push_str(&format!("v {:.6} {:.6} {:.6}\n", v.x, v.y, v.z));
        }
        for n in &result.mesh.normals {
            obj.push_str(&format!("vn {:.6} {:.6} {:.6}\n", n.x, n.y, n.z));
        }
        // OBJ indices are 1-based
        for chunk in result.mesh.indices.chunks(3) {
            let (a, b, c) = (chunk[0] + 1, chunk[1] + 1, chunk[2] + 1);
            obj.push_str(&format!("f {}//{} {}//{} {}//{}\n", a, a, b, b, c, c));
        }
        std::fs::write(
            "/private/tmp/claude-501/-Users-ladvien-pixy-tree/vlm_debug/test_mesh.obj",
            &obj,
        )
        .expect("Failed to write OBJ");
        println!(
            "Exported OBJ: {} verts, {} faces",
            result.mesh.vertices.len(),
            result.mesh.indices.len() / 3
        );

        // Also dump all branch-area vertices for analysis
        println!("=== ALL vertices ===");
        for (i, v) in result.mesh.vertices.iter().enumerate() {
            let n = &result.mesh.normals[i];
            println!(
                "v[{:3}]: pos=({:8.4},{:8.4},{:8.4}) norm=({:6.3},{:6.3},{:6.3})",
                i, v.x, v.y, v.z, n.x, n.y, n.z
            );
        }
        println!("=== ALL faces ===");
        for (fi, chunk) in result.mesh.indices.chunks(3).enumerate() {
            println!("f[{:3}]: {} {} {}", fi, chunk[0], chunk[1], chunk[2]);
        }
    }

    #[test]
    fn test_dump_minimal_branch_mesh() {
        // Minimal case: one trunk segment + one side branch
        let mut trunk = BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 5.0,
            radius: 0.3,
            depth: 0,
            height_ratio: 0.0,
            children: vec![],
        };

        // Continuation child (so trunk has 2+ children when branch added)
        let continuation = BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 5.0,
            radius: 0.2,
            depth: 0,
            height_ratio: 0.5,
            children: vec![],
        };
        trunk.children.push(BranchNodeChild {
            node: continuation,
            position_in_parent: 1.0,
        });

        // Side branch
        let branch = BranchNode {
            direction: Vector3::new(1.0, 0.3, 0.0).normalized(),
            tangent: get_perpendicular(Vector3::new(1.0, 0.3, 0.0).normalized()),
            length: 3.0,
            radius: 0.06,
            depth: 1,
            height_ratio: 0.5,
            children: vec![],
        };
        trunk.children.push(BranchNodeChild {
            node: branch,
            position_in_parent: 0.8,
        });

        let config = ManifoldMesherConfig {
            radial_resolution: 8,
            smooth_iterations: 0,
            ..Default::default()
        };

        let result = mesh_tree(&[trunk], &[Vector3::ZERO], &config);

        println!(
            "=== Minimal branch mesh: {} verts, {} tris ===",
            result.mesh.vertices.len(),
            result.mesh.indices.len() / 3
        );

        // Find branch vertices (X > 0.5 means they're in the branch direction)
        let mut branch_verts: Vec<(usize, Vector3)> = vec![];
        let mut trunk_verts: Vec<(usize, Vector3)> = vec![];
        for (i, v) in result.mesh.vertices.iter().enumerate() {
            if v.x > 0.5 {
                branch_verts.push((i, *v));
            } else {
                trunk_verts.push((i, *v));
            }
        }

        println!("Trunk vertices: {}", trunk_verts.len());
        println!("Branch vertices (x>0.5): {}", branch_verts.len());

        for (i, v) in &branch_verts {
            println!("  v[{}]: ({:.4}, {:.4}, {:.4})", i, v.x, v.y, v.z);
        }

        // Check branch vertex spread (should form a tube)
        if branch_verts.len() >= 2 {
            let min_y = branch_verts
                .iter()
                .map(|(_, v)| v.y)
                .fold(f32::MAX, f32::min);
            let max_y = branch_verts
                .iter()
                .map(|(_, v)| v.y)
                .fold(f32::MIN, f32::max);
            let min_z = branch_verts
                .iter()
                .map(|(_, v)| v.z)
                .fold(f32::MAX, f32::min);
            let max_z = branch_verts
                .iter()
                .map(|(_, v)| v.z)
                .fold(f32::MIN, f32::max);
            println!(
                "Branch Y range: {:.4} to {:.4} (spread: {:.4})",
                min_y,
                max_y,
                max_y - min_y
            );
            println!(
                "Branch Z range: {:.4} to {:.4} (spread: {:.4})",
                min_z,
                max_z,
                max_z - min_z
            );

            // For a tube with radius 0.06, the Y or Z spread should be ~0.12
            assert!(
                max_y - min_y > 0.01 || max_z - min_z > 0.01,
                "Branch vertices have zero spread - tube is degenerate!"
            );
        }
    }

    #[test]
    fn test_diagnose_branch_tree_structure() {
        // Replicate what tree.rs does for the unified manifold path
        // using realistic test scene values
        let config = TrunkNodeConfig {
            height: 10.0,
            base_radius: 0.3, // trunk_radius * trunk_flare (default flare=1.0)
            tip_radius: 0.12, // trunk_radius * trunk_taper (0.3 * 0.4)
            taper_curve: 0.4,
            height_segments: 4,
            trunk_randomness: 0.1,
            trunk_twist: 10.0,
            seed: 42,
            root_flare_count: 4,
            root_flare_spread: 0.4,
            root_flare_height: 0.1,
            termination: TrunkTermination::FlatCap,
            leader_segment: None,
        };

        // Simulate branches at various heights (branch_start=0.4, so starts at height 4.0)
        let branches = vec![
            BranchSegment {
                start: Vector3::new(0.2, 4.0, 0.0),
                direction: Vector3::new(1.0, -0.3, 0.0).normalized(),
                length: 3.0,
                base_radius: 0.06, // trunk_radius * branch_radius_ratio (0.3 * 0.2)
                tip_radius: 0.009,
                depth: 0,
                is_terminal: false,
                height_ratio: 0.4,
                subtree_weight: 5.0,
            },
            BranchSegment {
                start: Vector3::new(-0.1, 6.0, 0.15),
                direction: Vector3::new(-0.8, 0.1, 0.6).normalized(),
                length: 2.5,
                base_radius: 0.05,
                tip_radius: 0.008,
                depth: 0,
                is_terminal: false,
                height_ratio: 0.6,
                subtree_weight: 3.0,
            },
            // Sub-branch of first branch
            BranchSegment {
                start: Vector3::new(2.0, 3.5, 0.0),
                direction: Vector3::new(0.7, 0.5, 0.3).normalized(),
                length: 1.5,
                base_radius: 0.03,
                tip_radius: 0.005,
                depth: 1,
                is_terminal: true,
                height_ratio: 0.35,
                subtree_weight: 1.0,
            },
        ];

        let branch_heights: Vec<f32> = branches
            .iter()
            .filter(|b| b.depth == 0)
            .map(|b| b.start.y)
            .collect();

        let mut trunk = create_trunk_node_chain_with_branch_heights(&config, &branch_heights);

        // Print trunk structure before attaching branches
        println!("=== Trunk chain before branches ===");
        print_node_tree(&trunk, 0);

        attach_branches_to_trunk(&mut trunk, &branches, &config);

        // Print full tree after attaching branches
        println!("\n=== Full tree after attaching branches ===");
        print_node_tree(&trunk, 0);

        // Generate mesh and check vertex count
        let mesh_config = ManifoldMesherConfig {
            radial_resolution: 8,
            smooth_iterations: 0,
            ..Default::default()
        };
        let result = mesh_tree(&[trunk], &[Vector3::ZERO], &mesh_config);
        println!(
            "\n=== Mesh stats: {} vertices, {} indices ===",
            result.mesh.vertices.len(),
            result.mesh.indices.len()
        );

        // Check vertex bounding box to see if branches extend outward
        let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);
        for v in &result.mesh.vertices {
            min.x = min.x.min(v.x);
            min.y = min.y.min(v.y);
            min.z = min.z.min(v.z);
            max.x = max.x.max(v.x);
            max.y = max.y.max(v.y);
            max.z = max.z.max(v.z);
        }
        println!("Bounding box: min={:?}, max={:?}", min, max);
        println!(
            "Extent: x={:.3}, y={:.3}, z={:.3}",
            max.x - min.x,
            max.y - min.y,
            max.z - min.z
        );

        // If branches are working, X extent should be > 1.0 (branches extend outward)
        assert!(
            max.x - min.x > 0.5,
            "X extent too small ({:.3}), branches may not be rendering",
            max.x - min.x
        );
    }
}

#[test]
fn test_branch_tube_geometry() {
    // Minimal: trunk with one side branch, check branch tube is proper cylinder
    let branch_dir = Vector3::new(1.0, 0.0, 0.0); // Pure X direction for easy analysis
    let branch_tangent = get_perpendicular(branch_dir);
    let branch_radius = 0.2;
    let branch_length = 3.0;

    let mut trunk = BranchNode {
        direction: Vector3::UP,
        tangent: Vector3::RIGHT,
        length: 5.0,
        radius: 0.5,
        depth: 0,
        height_ratio: 0.0,
        children: vec![],
    };

    // Continuation
    trunk.children.push(BranchNodeChild {
        node: BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 5.0,
            radius: 0.4,
            depth: 0,
            height_ratio: 0.5,
            children: vec![],
        },
        position_in_parent: 1.0,
    });

    // Side branch (leaf)
    trunk.children.push(BranchNodeChild {
        node: BranchNode {
            direction: branch_dir,
            tangent: branch_tangent,
            length: branch_length,
            radius: branch_radius,
            depth: 1,
            height_ratio: 0.5,
            children: vec![],
        },
        position_in_parent: 0.5,
    });

    let config = ManifoldMesherConfig {
        radial_resolution: 6,
        smooth_iterations: 0,
        ..Default::default()
    };

    let result = mesh_tree(&[trunk], &[Vector3::ZERO], &config);
    let verts = &result.mesh.vertices;
    let norms = &result.mesh.normals;
    let indices = &result.mesh.indices;

    println!(
        "=== Branch tube geometry: {} verts, {} tris ===",
        verts.len(),
        indices.len() / 3
    );

    // Find branch vertices (those with X significantly > trunk radius)
    let trunk_radius = 0.5;
    let mut base_ring: Vec<(usize, Vector3, Vector3)> = vec![];
    let mut tip_ring: Vec<(usize, Vector3, Vector3)> = vec![];

    for (i, v) in verts.iter().enumerate() {
        // Branch base is near trunk surface (x ≈ 0.5), tip is at x ≈ 3.5
        if v.x > trunk_radius - 0.1 && v.x < trunk_radius + branch_radius + 0.5 {
            // Near trunk surface — could be branch base
            if v.y > 1.0 && v.y < 4.0 {
                // Mid-trunk height
                base_ring.push((i, *v, norms[i]));
            }
        }
        if v.x > 2.0 {
            // Far from trunk — branch tip
            tip_ring.push((i, *v, norms[i]));
        }
    }

    println!("\nBranch BASE ring ({} verts):", base_ring.len());
    for (i, v, n) in &base_ring {
        println!(
            "  v[{}]: pos=({:.3},{:.3},{:.3}) norm=({:.3},{:.3},{:.3})",
            i, v.x, v.y, v.z, n.x, n.y, n.z
        );
    }

    println!("\nBranch TIP ring ({} verts):", tip_ring.len());
    for (i, v, n) in &tip_ring {
        println!(
            "  v[{}]: pos=({:.3},{:.3},{:.3}) norm=({:.3},{:.3},{:.3})",
            i, v.x, v.y, v.z, n.x, n.y, n.z
        );
    }

    // Check tip ring spread — should have Y and Z spread of ~2*radius
    if tip_ring.len() >= 3 {
        let min_y = tip_ring
            .iter()
            .map(|(_, v, _)| v.y)
            .fold(f32::MAX, f32::min);
        let max_y = tip_ring
            .iter()
            .map(|(_, v, _)| v.y)
            .fold(f32::MIN, f32::max);
        let min_z = tip_ring
            .iter()
            .map(|(_, v, _)| v.z)
            .fold(f32::MAX, f32::min);
        let max_z = tip_ring
            .iter()
            .map(|(_, v, _)| v.z)
            .fold(f32::MIN, f32::max);
        let y_spread = max_y - min_y;
        let z_spread = max_z - min_z;
        println!(
            "\nTip ring spread: Y={:.4}, Z={:.4} (expected ~{:.4})",
            y_spread,
            z_spread,
            branch_radius * 2.0
        );
        assert!(
            y_spread > branch_radius * 0.5,
            "Y spread {:.4} too small for radius {:.4}",
            y_spread,
            branch_radius
        );
        assert!(
            z_spread > branch_radius * 0.5,
            "Z spread {:.4} too small for radius {:.4}",
            z_spread,
            branch_radius
        );
    }

    // Find faces that connect branch vertices
    let branch_vert_start = base_ring.iter().map(|(i, _, _)| *i).min().unwrap_or(0);
    let mut branch_faces = 0;
    for tri in indices.chunks(3) {
        let all_branch = tri.iter().all(|&idx| idx as usize >= branch_vert_start);
        if all_branch {
            branch_faces += 1;
        }
    }
    println!("\nBranch-only triangles: {}", branch_faces);

    // Verify normals point outward for tip ring
    if !tip_ring.is_empty() {
        let center = tip_ring
            .iter()
            .map(|(_, v, _)| *v)
            .fold(Vector3::ZERO, |a, b| a + b)
            / tip_ring.len() as f32;
        let mut outward_count = 0;
        let mut inward_count = 0;
        for (_, v, n) in &tip_ring {
            let to_outside = (*v - center).normalized();
            let dot = to_outside.dot(*n);
            if dot > 0.0 {
                outward_count += 1;
            } else {
                inward_count += 1;
            }
        }
        println!(
            "Tip normals: {} outward, {} inward",
            outward_count, inward_count
        );
        assert!(
            outward_count > inward_count,
            "Most normals should point outward! Got {} inward vs {} outward",
            inward_count,
            outward_count
        );
    }
}

fn print_node_tree(node: &BranchNode, indent: usize) {
    let prefix = " ".repeat(indent * 2);
    println!(
        "{}Node: dir=({:.2},{:.2},{:.2}) len={:.3} r={:.4} depth={} children={}",
        prefix,
        node.direction.x,
        node.direction.y,
        node.direction.z,
        node.length,
        node.radius,
        node.depth,
        node.children.len()
    );
    for (i, child) in node.children.iter().enumerate() {
        let label = if i == 0 { "cont" } else { "side" };
        println!(
            "{}  [{}] pos_in_parent={:.3}",
            prefix, label, child.position_in_parent
        );
        print_node_tree(&child.node, indent + 2);
    }
}
