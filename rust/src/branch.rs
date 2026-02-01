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
    #[allow(dead_code)] // Available for future LOD control
    pub radial_segments: i32,
    pub crown_shape: CrownShape,
    pub crown_influence: f32,
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

        // Phyllotaxis spiral + randomness (from modular_tree pattern)
        current_angle += config.phyllotaxis_angle + rng.range(-10.0, 10.0);
        let angle_rad = current_angle.to_radians();

        // Trunk radius at this height (tapered)
        let t = height / config.trunk_height;
        let trunk_r = lerp(config.trunk_radius, config.trunk_radius * 0.7, t);

        // Start position on trunk surface
        let start = Vector3::new(angle_rad.cos() * trunk_r, height, angle_rad.sin() * trunk_r);

        // Direction: lerp from up to outward based on branch_angle
        let outward = Vector3::new(angle_rad.cos(), 0.0, angle_rad.sin());
        let up = Vector3::UP;
        let base_dir = lerp_vec3(up, outward, config.branch_angle / 90.0);

        // Add randomness + up_attraction
        let random_offset = random_vec(rng) * config.branch_randomness;
        let up_offset = Vector3::UP * config.up_attraction;
        let direction = (base_dir + random_offset + up_offset).normalized();

        // Calculate radii
        let base_radius = trunk_r * config.branch_radius_ratio;
        let tip_radius = base_radius * (1.0 - config.branch_taper);

        // Apply crown shape envelope
        let height_ratio = (height - start_height) / zone_length;
        let shape_mult = config.crown_shape.get_length_multiplier(height_ratio);
        let final_mult = lerp(1.0, shape_mult, config.crown_influence);
        let length = config.trunk_height * config.branch_length * final_mult * rng.range(0.7, 1.0);

        // Determine if this branch will be terminal (no sub-branches)
        let is_terminal = config.branch_recursion == 0 || config.sub_branch_count == 0;

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
        // Position along parent (avoid base and tip)
        let t = rng.range(0.3, 0.8);
        let start = parent.start + parent.direction * parent.length * t;

        // Direction diverging from parent
        let spread = rng.range(30.0, 60.0).to_radians();
        let rotation = rng.range(0.0, TAU);

        // Perpendicular basis
        let perp = get_perpendicular(parent.direction);
        let perp2 = parent.direction.cross(perp);
        let offset = perp * rotation.cos() + perp2 * rotation.sin();

        let direction = (parent.direction * spread.cos() + offset * spread.sin()).normalized();

        // Scale down
        let length = parent.length * config.sub_branch_scale;
        let base_radius = parent.tip_radius * config.branch_radius_ratio;
        let tip_radius = base_radius * (1.0 - config.branch_taper);

        // Check if this will be a terminal branch (no further recursion)
        let is_terminal = depth + 1 >= config.branch_recursion;

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

/// Generate mesh data for a single branch segment (tapered cylinder)
pub fn generate_branch_mesh(branch: &BranchSegment, radial_segments: i32) -> MeshData {
    let mut mesh = MeshData::new();

    let segments = radial_segments.max(3) as usize;
    let rings = 2usize; // Just base and tip for branches

    // Calculate end position
    let end = branch.start + branch.direction * branch.length;

    // Build rotation basis to orient cylinder along direction
    let up = branch.direction;
    let right = get_perpendicular(up);
    let forward = up.cross(right);

    // Generate rings of vertices
    for ring in 0..rings {
        let t = ring as f32 / (rings - 1) as f32;
        let pos = lerp_vec3(branch.start, end, t);
        let radius = lerp(branch.base_radius, branch.tip_radius, t);
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

    // Add tip cap
    let tip_center_idx = mesh.vertices.len() as i32;
    mesh.vertices.push(end);
    mesh.normals.push(up);
    mesh.uvs.push(Vector2::new(0.5, 0.5));

    // Tip ring vertices (reusing direction for cap)
    for seg in 0..=segments {
        let angle = (seg as f32 / segments as f32) * TAU;
        let local_x = angle.cos() * branch.tip_radius;
        let local_z = angle.sin() * branch.tip_radius;

        let offset = right * local_x + forward * local_z;
        let vertex = end + offset;

        mesh.vertices.push(vertex);
        mesh.normals.push(up);
        mesh.uvs.push(Vector2::new(
            0.5 + angle.cos() * 0.5,
            0.5 + angle.sin() * 0.5,
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
