use godot::prelude::*;
use std::f32::consts::TAU;

use crate::crown_shape::CrownShape;

/// Accumulated mesh data for combining trunk + branches
#[derive(Default, Clone)]
pub struct MeshData {
    pub vertices: Vec<Vector3>,
    pub normals: Vec<Vector3>,
    pub uvs: Vec<Vector2>,
    pub indices: Vec<i32>,
}

impl MeshData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Extend this mesh with another, adjusting indices
    pub fn extend(&mut self, other: &MeshData) {
        let index_offset = self.vertices.len() as i32;

        self.vertices.extend_from_slice(&other.vertices);
        self.normals.extend_from_slice(&other.normals);
        self.uvs.extend_from_slice(&other.uvs);

        // Offset indices to account for existing vertices
        for idx in &other.indices {
            self.indices.push(idx + index_offset);
        }
    }
}

/// A single branch segment to be meshed
#[derive(Clone)]
pub struct BranchSegment {
    pub start: Vector3,
    pub direction: Vector3,
    pub length: f32,
    pub base_radius: f32,
    pub tip_radius: f32,
    #[allow(dead_code)] // Used for future LOD/detail variation
    pub depth: u8,
    /// Whether this branch has no sub-branches (is a terminal/leaf branch)
    pub is_terminal: bool,
    /// Height ratio within crown (0.0 at branch_start, 1.0 at branch_end)
    pub height_ratio: f32,
}

/// Configuration for branch generation (extracted from PixyTree exports)
pub struct BranchConfig {
    pub trunk_height: f32,
    pub trunk_radius: f32,
    pub branch_start: f32,
    pub branch_end: f32,
    pub branch_density: f32,
    pub branch_length: f32,
    pub branch_angle: f32,
    pub branch_radius_ratio: f32,
    pub branch_taper: f32,
    pub phyllotaxis_angle: f32,
    pub branch_randomness: f32,
    pub up_attraction: f32,
    pub branch_recursion: i32,
    pub sub_branch_count: i32,
    pub sub_branch_scale: f32,
    pub branch_length_variation: f32,
    pub sub_branch_position_bias: f32,
    pub apical_dominance: f32,
    pub branch_flatness: f32,
    pub branch_angle_curve: f32,
    pub crown_angle_variation: f32,
    #[allow(dead_code)] // Available for future LOD control
    pub radial_segments: i32,
    pub crown_shape: CrownShape,
    pub crown_influence: f32,
    // Trunk taper settings for branch origin calculation
    pub trunk_taper: f32,
    pub trunk_taper_curve: f32,
    pub trunk_flare: f32,
    pub trunk_randomness: f32,
    pub seed: i32,
    // Twist settings (passed to mesh generation directly, not used in BranchConfig)
    #[allow(dead_code)]
    pub trunk_twist: f32,
    #[allow(dead_code)]
    pub branch_twist: f32,
    // Gravity settings (passed to mesh generation directly)
    #[allow(dead_code)]
    pub gravity_strength: f32,
    #[allow(dead_code)]
    pub stiffness: f32,
    // Branch randomness
    pub break_chance: f32,
    // Branch splitting
    pub split_enabled: bool,
    pub split_probability: f32,
    pub split_angle: f32,
    pub split_position: f32,
    pub split_radius_threshold: f32,
    // Floor avoidance
    pub floor_avoidance: bool,
    pub floor_level: f32,
}

/// Xorshift64 RNG for deterministic generation
pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    pub fn new(seed: i32) -> Self {
        // Ensure non-zero state
        let state = if seed == 0 { 1 } else { seed as u64 };
        Self { state }
    }

    /// Generate next random u64
    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generate random f32 in [0, 1)
    pub fn next_f32(&mut self) -> f32 {
        (self.next() as f32) / (u64::MAX as f32)
    }

    /// Generate random f32 in [min, max)
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

/// Linear interpolation for f32
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Linear interpolation for Vector3
fn lerp_vec3(a: Vector3, b: Vector3, t: f32) -> Vector3 {
    Vector3::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t), lerp(a.z, b.z, t))
}

/// Generate a random unit vector offset
fn random_vec(rng: &mut SeededRng) -> Vector3 {
    Vector3::new(
        rng.range(-1.0, 1.0),
        rng.range(-1.0, 1.0),
        rng.range(-1.0, 1.0),
    )
    .normalized()
}

/// Get a perpendicular vector to the given direction
fn get_perpendicular(dir: Vector3) -> Vector3 {
    // Choose a vector that's not parallel to dir
    let up = if dir.y.abs() < 0.9 {
        Vector3::UP
    } else {
        Vector3::RIGHT
    };
    dir.cross(up).normalized()
}

/// Generate split branches from a parent branch (Y-junction)
/// Returns the stub segment (up to split point) and two diverging branches
pub fn generate_split_branches(
    parent: &BranchSegment,
    config: &BranchConfig,
    rng: &mut SeededRng,
) -> Option<(BranchSegment, BranchSegment, BranchSegment)> {
    if !config.split_enabled || rng.next_f32() > config.split_probability {
        return None;
    }

    // Only allow thicker branches to split
    if parent.base_radius < config.split_radius_threshold {
        return None;
    }

    // Calculate split point along the parent branch
    let split_t = config.split_position;
    let split_point = parent.start + parent.direction * parent.length * split_t;

    // Remaining length after split
    let remaining_length = parent.length * (1.0 - split_t);

    // Radius at split point (tapered)
    let split_radius = lerp(parent.base_radius, parent.tip_radius, split_t);

    // Create the stub segment (from start to split point)
    let stub = BranchSegment {
        start: parent.start,
        direction: parent.direction,
        length: parent.length * split_t,
        base_radius: parent.base_radius,
        tip_radius: split_radius,
        depth: parent.depth,
        is_terminal: false, // Never terminal - it has children
        height_ratio: parent.height_ratio,
    };

    // Calculate divergent directions for the two split branches
    let half_angle = (config.split_angle / 2.0).to_radians();

    // Get perpendicular basis for spreading
    let perp = get_perpendicular(parent.direction);

    // Random rotation around the parent direction for variety
    let rotation = rng.range(0.0, TAU);
    let cos_rot = rotation.cos();
    let sin_rot = rotation.sin();
    let perp2 = parent.direction.cross(perp);
    let rotated_perp = perp * cos_rot + perp2 * sin_rot;

    // Branch 1: Deflect in one direction
    let dir1 = (parent.direction * half_angle.cos() + rotated_perp * half_angle.sin()).normalized();

    // Branch 2: Deflect in opposite direction
    let dir2 = (parent.direction * half_angle.cos() - rotated_perp * half_angle.sin()).normalized();

    // Each split branch gets roughly half the remaining radius reduction
    let branch_tip_radius = parent.tip_radius * 0.9; // Slightly smaller tips

    let branch1 = BranchSegment {
        start: split_point,
        direction: dir1,
        length: remaining_length * rng.range(0.9, 1.1), // Slight length variation
        base_radius: split_radius * 0.7,                // Each split branch is thinner
        tip_radius: branch_tip_radius * 0.7,
        depth: parent.depth,
        is_terminal: parent.is_terminal,
        height_ratio: parent.height_ratio,
    };

    let branch2 = BranchSegment {
        start: split_point,
        direction: dir2,
        length: remaining_length * rng.range(0.9, 1.1),
        base_radius: split_radius * 0.7,
        tip_radius: branch_tip_radius * 0.7,
        depth: parent.depth,
        is_terminal: parent.is_terminal,
        height_ratio: parent.height_ratio,
    };

    Some((stub, branch1, branch2))
}

/// Generate primary branch origins along the trunk using phyllotaxis spiral
pub fn generate_branch_origins(config: &BranchConfig, rng: &mut SeededRng) -> Vec<BranchSegment> {
    let mut branches = Vec::new();
    let mut current_angle = 0.0f32;

    // Calculate branch zone
    let start_height = config.trunk_height * config.branch_start;
    let end_height = config.trunk_height * config.branch_end;
    let zone_length = end_height - start_height;

    if zone_length <= 0.0 {
        return branches;
    }

    // Spacing from density
    let spacing = 1.0 / (config.branch_density + 0.001);
    let branch_count = (zone_length / spacing).floor() as i32;

    for i in 0..branch_count {
        // Height with small random offset within spacing
        let base_height = start_height + spacing * i as f32;
        let height = base_height + rng.range(0.0, spacing * 0.5);

        if height > end_height {
            break;
        }

        // Check for break_chance (branch terminates/breaks off early)
        if config.break_chance > 0.0 && rng.next_f32() < config.break_chance {
            continue; // Skip this branch (it "broke off")
        }

        // Phyllotaxis spiral + randomness (from modular_tree pattern)
        current_angle += config.phyllotaxis_angle + rng.range(-10.0, 10.0);
        let angle_rad = current_angle.to_radians();

        // Trunk radius at this height (using proper taper parameters)
        let t = height / config.trunk_height;
        let taper_factor = t.powf(config.trunk_taper_curve);
        let base_radius = config.trunk_radius * config.trunk_flare;
        let tip_radius = config.trunk_radius * config.trunk_taper;
        let trunk_r = lerp(base_radius, tip_radius, taper_factor);

        // Calculate trunk wobble offset at this height (must match create_trunk_mesh_data)
        let seed_f = config.seed as f32;
        let wobble_x = if config.trunk_randomness > 0.0 {
            (t * std::f32::consts::PI + seed_f * 0.1).sin()
                * config.trunk_randomness
                * t
                * config.trunk_height
                * 0.3
        } else {
            0.0
        };
        let wobble_z = if config.trunk_randomness > 0.0 {
            (t * std::f32::consts::E + seed_f * 0.2).cos()
                * config.trunk_randomness
                * t
                * config.trunk_height
                * 0.3
        } else {
            0.0
        };

        // Start position on trunk surface (with wobble offset)
        let start = Vector3::new(
            angle_rad.cos() * trunk_r + wobble_x,
            height,
            angle_rad.sin() * trunk_r + wobble_z,
        );

        // Direction: lerp from up to outward based on branch_angle
        // Apply crown angle variation: varies base angle with height
        // Positive: lower branches more vertical, upper more horizontal
        // Negative: lower branches more horizontal, upper more vertical
        let height_ratio = (height - start_height) / zone_length;
        let angle_adjustment = config.crown_angle_variation * (height_ratio - 0.5) * 30.0;
        let effective_angle = (config.branch_angle + angle_adjustment).clamp(0.0, 90.0);

        let outward = Vector3::new(angle_rad.cos(), 0.0, angle_rad.sin());
        let up = Vector3::UP;
        let base_dir = lerp_vec3(up, outward, effective_angle / 90.0);

        // Apply flatness: reduce y component toward horizontal
        let flattened_dir = if config.branch_flatness > 0.0 {
            Vector3::new(
                base_dir.x,
                base_dir.y * (1.0 - config.branch_flatness),
                base_dir.z,
            )
            .normalized()
        } else {
            base_dir
        };

        // Add randomness + up_attraction
        let random_offset = random_vec(rng) * config.branch_randomness;
        let up_offset = Vector3::UP * config.up_attraction;
        let direction = (flattened_dir + random_offset + up_offset).normalized();

        // Calculate radii
        let base_radius = trunk_r * config.branch_radius_ratio;
        let tip_radius = base_radius * (1.0 - config.branch_taper);

        // Apply crown shape envelope
        let height_ratio = (height - start_height) / zone_length;
        let shape_mult = config.crown_shape.get_length_multiplier(height_ratio);
        let final_mult = lerp(1.0, shape_mult, config.crown_influence);

        // Apply length variation: higher variation = wider range
        let variation_min = 1.0 - config.branch_length_variation;

        // Apply apical dominance: higher branches are suppressed (shorter)
        // Formula: length_mult = 1.0 - (apical_dominance * height_ratio * 0.5)
        let dominance_mult = 1.0 - (config.apical_dominance * height_ratio * 0.5);

        let length = config.trunk_height
            * config.branch_length
            * final_mult
            * dominance_mult
            * rng.range(variation_min, 1.0);

        // Apply angle curve based on height (adjust direction)
        let direction = if config.branch_angle_curve.abs() > 0.001 {
            // Positive curve = upper branches reach upward, negative = droop at top
            let angle_adjustment = config.branch_angle_curve * height_ratio * 30.0f32.to_radians();
            Vector3::new(
                direction.x,
                direction.y + angle_adjustment.sin(),
                direction.z,
            )
            .normalized()
        } else {
            direction
        };

        // Floor avoidance: check if branch end would be below floor level
        let (direction, length, is_terminal) = if config.floor_avoidance {
            let end_y = start.y + direction.y * length;
            if end_y < config.floor_level {
                if direction.y < -0.3 {
                    // Branch points steeply downward - skip it entirely
                    continue;
                } else if direction.y < 0.0 {
                    // Clamp direction to be more horizontal and shorten if needed
                    let clamped_dir = Vector3::new(direction.x, 0.0, direction.z).normalized();
                    // Calculate max length that keeps end above floor
                    let max_length = if clamped_dir.y.abs() < 0.001 {
                        length // Horizontal branch, keep original length
                    } else {
                        ((start.y - config.floor_level) / -clamped_dir.y).max(0.1)
                    };
                    (clamped_dir, length.min(max_length), true) // Mark as terminal
                } else {
                    (
                        direction,
                        length,
                        config.branch_recursion == 0 || config.sub_branch_count == 0,
                    )
                }
            } else {
                (
                    direction,
                    length,
                    config.branch_recursion == 0 || config.sub_branch_count == 0,
                )
            }
        } else {
            // Determine if this branch will be terminal (no sub-branches)
            (
                direction,
                length,
                config.branch_recursion == 0 || config.sub_branch_count == 0,
            )
        };

        branches.push(BranchSegment {
            start,
            direction,
            length,
            base_radius,
            tip_radius,
            depth: 0,
            is_terminal,
            height_ratio,
        });
    }

    branches
}

/// Generate sub-branches recursively from a parent branch
pub fn generate_sub_branches(
    parent: &BranchSegment,
    config: &BranchConfig,
    rng: &mut SeededRng,
    depth: i32,
) -> Vec<BranchSegment> {
    if depth >= config.branch_recursion {
        return vec![];
    }

    let mut branches = Vec::new();

    for _ in 0..config.sub_branch_count {
        // Check for break_chance (sub-branch terminates early)
        if config.break_chance > 0.0 && rng.next_f32() < config.break_chance {
            continue; // Skip this sub-branch (it "broke off")
        }

        // Position along parent (avoid base and tip)
        // Apply bias: positive = toward tip (0.8), negative = toward base (0.3)
        let base_t = rng.range(0.3, 0.8);
        let t = if config.sub_branch_position_bias > 0.0 {
            lerp(base_t, 0.8, config.sub_branch_position_bias)
        } else {
            lerp(base_t, 0.3, -config.sub_branch_position_bias)
        };
        let start = parent.start + parent.direction * parent.length * t;

        // Direction diverging from parent
        let spread = rng.range(30.0, 60.0).to_radians();
        let rotation = rng.range(0.0, TAU);

        // Perpendicular basis
        let perp = get_perpendicular(parent.direction);
        let perp2 = parent.direction.cross(perp);
        let offset = perp * rotation.cos() + perp2 * rotation.sin();

        let base_direction = (parent.direction * spread.cos() + offset * spread.sin()).normalized();

        // Apply flatness to sub-branches as well
        let direction = if config.branch_flatness > 0.0 {
            Vector3::new(
                base_direction.x,
                base_direction.y * (1.0 - config.branch_flatness),
                base_direction.z,
            )
            .normalized()
        } else {
            base_direction
        };

        // Scale down with apical dominance affecting sub-branch length
        // Higher parent branches get shorter sub-branches
        let dominance_mult = 1.0 - (config.apical_dominance * parent.height_ratio * 0.4);
        let length = parent.length * config.sub_branch_scale * dominance_mult;
        let base_radius = parent.tip_radius * config.branch_radius_ratio;
        let tip_radius = base_radius * (1.0 - config.branch_taper);

        // Floor avoidance for sub-branches
        let (direction, length, force_terminal) = if config.floor_avoidance {
            let end_y = start.y + direction.y * length;
            if end_y < config.floor_level {
                if direction.y < -0.3 {
                    // Branch points steeply downward - skip it
                    continue;
                } else if direction.y < 0.0 {
                    // Clamp direction to be more horizontal
                    let clamped_dir = Vector3::new(direction.x, 0.0, direction.z).normalized();
                    (clamped_dir, length, true)
                } else {
                    (direction, length, false)
                }
            } else {
                (direction, length, false)
            }
        } else {
            (direction, length, false)
        };

        // Check if this will be a terminal branch (no further recursion)
        let is_terminal = force_terminal || depth + 1 >= config.branch_recursion;

        let segment = BranchSegment {
            start,
            direction,
            length,
            base_radius,
            tip_radius,
            depth: (depth + 1) as u8,
            is_terminal,
            height_ratio: parent.height_ratio, // Inherit parent's height ratio
        };

        // If we're going to recurse, mark parent as non-terminal
        let mut final_segment = segment.clone();

        // Recurse
        let sub_branches = generate_sub_branches(&segment, config, rng, depth + 1);

        // If we generated sub-branches, this segment is not terminal
        if !sub_branches.is_empty() {
            final_segment.is_terminal = false;
        }

        branches.push(final_segment);
        branches.extend(sub_branches);
    }

    branches
}

/// Apply gravity bending to a direction vector
/// stiffness (0 = flexible, 1 = stiff) reduces the gravity effect
fn apply_gravity(direction: Vector3, t: f32, gravity_strength: f32, stiffness: f32) -> Vector3 {
    if gravity_strength <= 0.0 {
        return direction;
    }
    // Apply stiffness to reduce gravity effect
    let effective_gravity = gravity_strength * (1.0 - stiffness);
    if effective_gravity <= 0.0 {
        return direction;
    }
    // Quadratic bend - more effect toward the tip
    let bend = t * t * effective_gravity;
    let bent = Vector3::new(direction.x, direction.y - bend, direction.z);
    bent.normalized()
}

/// Generate mesh data for a single branch segment with twist and gravity support
pub fn generate_branch_mesh_with_config(
    branch: &BranchSegment,
    radial_segments: i32,
    branch_twist: f32,
    gravity_strength: f32,
    stiffness: f32,
) -> MeshData {
    let mut mesh = MeshData::new();

    let segments = radial_segments.max(3) as usize;
    // Use more rings when twist or gravity is applied for smoother curves
    let rings = if branch_twist.abs() > 0.1 || gravity_strength > 0.01 {
        6usize // More rings for curved branches
    } else {
        2usize // Simple branches need only base and tip
    };

    // Track cumulative position along the curved branch
    let mut positions: Vec<Vector3> = Vec::with_capacity(rings);
    let mut directions: Vec<Vector3> = Vec::with_capacity(rings);

    // Pre-compute positions along the branch with gravity
    let step_length = branch.length / (rings - 1) as f32;
    let mut current_pos = branch.start;
    let mut current_dir;

    for ring in 0..rings {
        let t = ring as f32 / (rings - 1) as f32;
        positions.push(current_pos);

        // Apply gravity bending to direction (stiffness reduces effect)
        current_dir = apply_gravity(branch.direction, t, gravity_strength, stiffness);
        directions.push(current_dir);

        if ring < rings - 1 {
            current_pos += current_dir * step_length;
        }
    }

    let end_pos = positions[rings - 1];
    let end_dir = directions[rings - 1];

    // Generate rings of vertices
    for ring in 0..rings {
        let t = ring as f32 / (rings - 1) as f32;
        let pos = positions[ring];
        let dir = directions[ring];
        let radius = lerp(branch.base_radius, branch.tip_radius, t);
        let v = t;

        // Calculate twist angle for this ring
        let twist_angle = t * branch_twist.to_radians();

        // Build rotation basis from current direction
        let right = get_perpendicular(dir);
        let forward = dir.cross(right);

        for seg in 0..=segments {
            let base_angle = (seg as f32 / segments as f32) * TAU;
            // Apply twist by rotating around the branch axis
            let angle = base_angle + twist_angle;

            let local_x = angle.cos() * radius;
            let local_z = angle.sin() * radius;

            // Transform to world space using our basis
            let offset = right * local_x + forward * local_z;
            let vertex = pos + offset;

            // Normal points outward in the local frame (also twisted)
            let normal = (right * angle.cos() + forward * angle.sin()).normalized();

            mesh.vertices.push(vertex);
            mesh.normals.push(normal);
            mesh.uvs.push(Vector2::new(seg as f32 / segments as f32, v));
        }
    }

    // Generate indices for cylinder sides
    let verts_per_ring = segments + 1;
    for ring in 0..(rings - 1) {
        for seg in 0..segments {
            let current = (ring * verts_per_ring + seg) as i32;
            let next = (ring * verts_per_ring + seg + 1) as i32;
            let above = ((ring + 1) * verts_per_ring + seg) as i32;
            let above_next = ((ring + 1) * verts_per_ring + seg + 1) as i32;

            // Two triangles per quad
            mesh.indices.extend_from_slice(&[current, next, above]);
            mesh.indices.extend_from_slice(&[next, above_next, above]);
        }
    }

    // Build end basis for tip cap
    let end_right = get_perpendicular(end_dir);
    let end_forward = end_dir.cross(end_right);
    let tip_twist_angle = branch_twist.to_radians();

    // Add tip cap
    let tip_center_idx = mesh.vertices.len() as i32;
    mesh.vertices.push(end_pos);
    mesh.normals.push(end_dir);
    mesh.uvs.push(Vector2::new(0.5, 0.5));

    // Tip ring vertices
    for seg in 0..=segments {
        let base_angle = (seg as f32 / segments as f32) * TAU;
        let angle = base_angle + tip_twist_angle;
        let local_x = angle.cos() * branch.tip_radius;
        let local_z = angle.sin() * branch.tip_radius;

        let offset = end_right * local_x + end_forward * local_z;
        let vertex = end_pos + offset;

        mesh.vertices.push(vertex);
        mesh.normals.push(end_dir);
        mesh.uvs.push(Vector2::new(
            0.5 + base_angle.cos() * 0.5,
            0.5 + base_angle.sin() * 0.5,
        ));
    }

    // Tip cap triangles
    let tip_ring_start = tip_center_idx + 1;
    for seg in 0..segments {
        let current = tip_ring_start + seg as i32;
        let next = tip_ring_start + (seg + 1) as i32;
        mesh.indices
            .extend_from_slice(&[tip_center_idx, current, next]);
    }

    mesh
}

/// Generate mesh data for a single branch segment (tapered cylinder)
/// Legacy version without twist/gravity - calls new version with defaults
#[allow(dead_code)]
pub fn generate_branch_mesh(branch: &BranchSegment, radial_segments: i32) -> MeshData {
    generate_branch_mesh_with_config(branch, radial_segments, 0.0, 0.0, 0.5)
}

/// Calculate parent radius using the pipe model (Da Vinci's rule).
/// The sum of cross-sectional areas of child branches equals the parent's area.
///
/// Formula: parent_radius^exponent = sum(child_radius^exponent)
/// Typical exponent is 2.0 (Da Vinci's rule) to 2.5 (Murray's law)
///
/// # Arguments
/// * `child_radii` - Radii of all child branches
/// * `exponent` - Power-law exponent (default 2.0)
/// * `min_radius` - Minimum radius to prevent branches from disappearing
pub fn calculate_pipe_radius(child_radii: &[f32], exponent: f32, min_radius: f32) -> f32 {
    if child_radii.is_empty() {
        return min_radius;
    }

    // Sum of radius^exponent for all children
    let accumulated: f32 = child_radii.iter().map(|r| r.powf(exponent)).sum();

    // Parent radius is the nth root of the accumulated sum
    let radius = accumulated.powf(1.0 / exponent);

    radius.max(min_radius)
}

/// Apply pipe radius model to a collection of branches.
/// This performs a bottom-up traversal to calculate parent radii from children.
///
/// # Arguments
/// * `branches` - Mutable slice of branches (will be modified in place)
/// * `exponent` - Power-law exponent for radius calculation
/// * `min_radius` - Minimum radius threshold
///
/// # Note
/// This function identifies parent-child relationships by position matching.
/// A branch B is a child of A if B.start is close to A's endpoint (start + direction * length).
pub fn apply_pipe_radius_model(branches: &mut [BranchSegment], exponent: f32, min_radius: f32) {
    if branches.is_empty() {
        return;
    }

    // Build parent-child relationships based on position matching
    // For each branch, find which branch it connects to (parent)
    let epsilon = 0.01; // Position matching tolerance

    // Create a map of branch index to its children's indices
    let mut children_map: std::collections::HashMap<usize, Vec<usize>> =
        std::collections::HashMap::new();

    for i in 0..branches.len() {
        children_map.insert(i, Vec::new());
    }

    // Find parent for each branch
    for (child_idx, child) in branches.iter().enumerate() {
        for (parent_idx, parent) in branches.iter().enumerate() {
            if parent_idx == child_idx {
                continue;
            }

            // Check if child starts near parent's endpoint
            let parent_end = parent.start + parent.direction * parent.length;
            let distance = (child.start - parent_end).length();

            if distance < epsilon {
                if let Some(children) = children_map.get_mut(&parent_idx) {
                    children.push(child_idx);
                }
                break; // Each child has at most one parent
            }
        }
    }

    // Sort branches by depth (deepest first for bottom-up traversal)
    let mut sorted_indices: Vec<usize> = (0..branches.len()).collect();
    sorted_indices.sort_by(|&a, &b| branches[b].depth.cmp(&branches[a].depth));

    // Bottom-up: recalculate radii from children
    for &idx in &sorted_indices {
        if let Some(child_indices) = children_map.get(&idx) {
            if !child_indices.is_empty() {
                // Collect child base radii
                let child_radii: Vec<f32> = child_indices
                    .iter()
                    .map(|&c| branches[c].base_radius)
                    .collect();

                // Calculate new radius using pipe model
                let new_radius = calculate_pipe_radius(&child_radii, exponent, min_radius);

                // Update this branch's tip radius to match children
                // and propagate upward
                branches[idx].tip_radius = new_radius;

                // Optionally adjust base radius if it's smaller than tip
                if branches[idx].base_radius < branches[idx].tip_radius {
                    branches[idx].base_radius = branches[idx].tip_radius * 1.2;
                }
            }
        }
    }

    // Ensure no branch has tip larger than base
    for branch in branches.iter_mut() {
        if branch.tip_radius > branch.base_radius {
            branch.tip_radius = branch.base_radius * 0.8;
        }
        // Apply minimum radius
        branch.base_radius = branch.base_radius.max(min_radius);
        branch.tip_radius = branch.tip_radius.max(min_radius * 0.5);
    }
}

/// Generate a collar mesh that smoothly connects trunk surface to branch base.
/// This creates a tapered cone-like transition that bridges the gap between
/// the trunk and branch, eliminating visible seams.
///
/// # Arguments
/// * `branch_start` - Branch attachment point on trunk surface
/// * `branch_direction` - Branch growth direction (normalized)
/// * `trunk_radius` - Trunk radius at this height
/// * `branch_radius` - Branch base radius
/// * `collar_length` - How far collar extends along branch direction
/// * `radial_segments` - Segments around circumference
pub fn generate_branch_collar(
    branch_start: Vector3,
    branch_direction: Vector3,
    _trunk_radius: f32,
    branch_radius: f32,
    collar_length: f32,
    radial_segments: i32,
) -> MeshData {
    let mut mesh = MeshData::new();

    let segments = radial_segments.max(3) as usize;
    let rings = 3usize; // Base ring on trunk, middle blend, end at branch start

    // Build rotation basis from branch direction
    let right = get_perpendicular(branch_direction);
    let forward = branch_direction.cross(right);

    // Calculate the collar's starting position (slightly embedded in trunk)
    // and ending position (at branch start)
    let collar_start = branch_start - branch_direction * collar_length * 0.1;
    let collar_end = branch_start + branch_direction * collar_length * 0.5;

    // The collar starts with a wider "flare" at the trunk and tapers to branch radius
    // Start radius is larger to blend with trunk surface
    let start_radius = branch_radius * 1.3;
    let end_radius = branch_radius;

    // Generate rings of vertices
    for ring in 0..rings {
        let t = ring as f32 / (rings - 1) as f32;

        // Position along collar (lerp from start to end)
        let pos = Vector3::new(
            lerp(collar_start.x, collar_end.x, t),
            lerp(collar_start.y, collar_end.y, t),
            lerp(collar_start.z, collar_end.z, t),
        );

        // Radius tapers from start_radius to end_radius
        // Use smooth interpolation for natural look
        let smooth_t = t * t * (3.0 - 2.0 * t); // Smoothstep
        let radius = lerp(start_radius, end_radius, smooth_t);

        let v = t;

        for seg in 0..=segments {
            let angle = (seg as f32 / segments as f32) * TAU;

            let local_x = angle.cos() * radius;
            let local_z = angle.sin() * radius;

            // Transform to world space using our basis
            let offset = right * local_x + forward * local_z;
            let vertex = pos + offset;

            // Normal points outward in the local frame
            let normal = (right * angle.cos() + forward * angle.sin()).normalized();

            mesh.vertices.push(vertex);
            mesh.normals.push(normal);
            mesh.uvs.push(Vector2::new(seg as f32 / segments as f32, v));
        }
    }

    // Generate indices for collar sides (connect rings with triangles)
    let verts_per_ring = segments + 1;
    for ring in 0..(rings - 1) {
        for seg in 0..segments {
            let current = (ring * verts_per_ring + seg) as i32;
            let next = (ring * verts_per_ring + seg + 1) as i32;
            let above = ((ring + 1) * verts_per_ring + seg) as i32;
            let above_next = ((ring + 1) * verts_per_ring + seg + 1) as i32;

            // Two triangles per quad (counter-clockwise winding for front faces)
            mesh.indices.extend_from_slice(&[current, next, above]);
            mesh.indices.extend_from_slice(&[next, above_next, above]);
        }
    }

    mesh
}
