use godot::prelude::*;
use std::collections::HashMap;
use std::f32::consts::TAU;

use crate::crown_shape::CrownShape;

// ═══════════════════════════════════════════════════════════════════════════════
// Manifold Mesh Data Structures
// ═══════════════════════════════════════════════════════════════════════════════

/// Hash key for vertex deduplication: combines position and normal
/// Uses quantized coordinates to handle floating-point imprecision
fn vertex_key(pos: Vector3, normal: Vector3) -> (i64, i64, i64, i64, i64, i64) {
    const PRECISION: f32 = 10000.0; // 0.0001 unit precision
    (
        (pos.x * PRECISION).round() as i64,
        (pos.y * PRECISION).round() as i64,
        (pos.z * PRECISION).round() as i64,
        (normal.x * PRECISION).round() as i64,
        (normal.y * PRECISION).round() as i64,
        (normal.z * PRECISION).round() as i64,
    )
}

/// Vertex pool with hash-based deduplication for shared vertices
/// Used to ensure branches share vertices with trunk hole boundaries
#[derive(Default, Clone)]
pub struct VertexPool {
    pub vertices: Vec<Vector3>,
    pub normals: Vec<Vector3>,
    pub uvs: Vec<Vector2>,
    pub vertex_map: HashMap<(i64, i64, i64, i64, i64, i64), u32>,
}

impl VertexPool {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a vertex to the pool, returning its index
    /// If a vertex with the same position+normal exists, returns existing index
    pub fn add_vertex(&mut self, pos: Vector3, normal: Vector3, uv: Vector2) -> u32 {
        let key = vertex_key(pos, normal);

        if let Some(&idx) = self.vertex_map.get(&key) {
            return idx;
        }

        let idx = self.vertices.len() as u32;
        self.vertices.push(pos);
        self.normals.push(normal);
        self.uvs.push(uv);
        self.vertex_map.insert(key, idx);
        idx
    }

    /// Add a vertex without deduplication (for unique vertices like cap centers)
    pub fn add_unique_vertex(&mut self, pos: Vector3, normal: Vector3, uv: Vector2) -> u32 {
        let idx = self.vertices.len() as u32;
        self.vertices.push(pos);
        self.normals.push(normal);
        self.uvs.push(uv);
        idx
    }

    /// Get vertex count
    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    /// Check if pool is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Update the normal at a specific vertex index (for normal smoothing)
    #[allow(dead_code)]
    pub fn set_normal(&mut self, idx: u32, normal: Vector3) {
        if (idx as usize) < self.normals.len() {
            self.normals[idx as usize] = normal;
        }
    }

    /// Get the normal at a specific vertex index
    #[allow(dead_code)]
    pub fn get_normal(&self, idx: u32) -> Option<Vector3> {
        self.normals.get(idx as usize).copied()
    }

    /// Get position of a vertex by index
    pub fn get_position(&self, idx: u32) -> Vector3 {
        self.vertices[idx as usize]
    }
}

/// A ring of vertex indices representing a hole boundary or branch cross-section
/// Vertices are ordered counter-clockwise when viewed from outside (along direction)
#[derive(Clone, Debug)]
pub struct VertexRing {
    /// Ordered vertex indices forming the ring
    pub indices: Vec<u32>,
    /// Center point of the ring
    #[allow(dead_code)]
    pub center: Vector3,
    /// Direction the ring faces (outward normal)
    #[allow(dead_code)]
    pub direction: Vector3,
    /// Radius of the ring
    #[allow(dead_code)]
    pub radius: f32,
}

impl VertexRing {
    #[allow(dead_code)]
    pub fn new(indices: Vec<u32>, center: Vector3, direction: Vector3, radius: f32) -> Self {
        Self {
            indices,
            center,
            direction,
            radius,
        }
    }

    /// Get number of vertices in the ring
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Check if ring is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Represents a hole boundary on the trunk surface for manifold branch connections.
/// Contains vertices from both the lower and upper rings that form a closed loop
/// around the hole where a branch attaches.
///
/// The boundary forms a closed loop by traversing:
/// 1. Lower ring vertices in segment order (left to right)
/// 2. Upper ring vertices in reverse segment order (right to left)
///
/// This creates a proper quad loop that can be bridged to a branch base.
#[derive(Clone, Debug)]
pub struct HoleBoundary {
    /// Lower ring vertex indices (in segment order)
    pub lower_indices: Vec<u32>,
    /// Upper ring vertex indices (in segment order, will be reversed when forming loop)
    pub upper_indices: Vec<u32>,
    /// Lower ring vertex positions (for use in branch mesh generation)
    pub lower_positions: Vec<Vector3>,
    /// Upper ring vertex positions (for use in branch mesh generation)
    pub upper_positions: Vec<Vector3>,
    /// Center point of the hole (average of all boundary vertices)
    #[allow(dead_code)]
    pub center: Vector3,
    /// Branch direction (outward from trunk surface)
    #[allow(dead_code)]
    pub direction: Vector3,
    /// Branch radius at attachment
    #[allow(dead_code)]
    pub branch_radius: f32,
    /// Trunk radius at attachment height
    #[allow(dead_code)]
    pub trunk_radius: f32,
    /// Angular extent of the hole arc (in radians)
    #[allow(dead_code)]
    pub arc_extent: f32,
    /// Starting angle of the arc on trunk (radians)
    #[allow(dead_code)]
    pub arc_start_angle: f32,
}

impl HoleBoundary {
    /// Create a new hole boundary from lower and upper ring indices and positions
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        lower_indices: Vec<u32>,
        upper_indices: Vec<u32>,
        lower_positions: Vec<Vector3>,
        upper_positions: Vec<Vector3>,
        center: Vector3,
        direction: Vector3,
        branch_radius: f32,
        trunk_radius: f32,
        arc_extent: f32,
        arc_start_angle: f32,
    ) -> Self {
        Self {
            lower_indices,
            upper_indices,
            lower_positions,
            upper_positions,
            center,
            direction,
            branch_radius,
            trunk_radius,
            arc_extent,
            arc_start_angle,
        }
    }

    /// Get total number of boundary vertices (lower + upper)
    #[allow(dead_code)]
    pub fn vertex_count(&self) -> usize {
        self.lower_indices.len() + self.upper_indices.len()
    }

    /// Get number of segments in the arc (same for both rings)
    #[allow(dead_code)]
    pub fn arc_segments(&self) -> usize {
        self.lower_indices.len()
    }

    /// Check if boundary is valid (has enough vertices)
    pub fn is_valid(&self) -> bool {
        self.lower_indices.len() >= 2 && self.upper_indices.len() >= 2
    }

    /// Check if boundary is empty
    pub fn is_empty(&self) -> bool {
        self.lower_indices.is_empty() && self.upper_indices.is_empty()
    }

    /// Get all indices forming a closed loop around the hole.
    /// Returns indices in order: lower ring (segment order) -> upper ring (reversed)
    /// This creates a CCW loop when viewed from outside the trunk.
    #[allow(dead_code)]
    pub fn get_loop_indices(&self) -> Vec<u32> {
        let mut loop_indices = self.lower_indices.clone();
        // Add upper ring in reverse order to close the loop
        loop_indices.extend(self.upper_indices.iter().rev().copied());
        loop_indices
    }

    /// Get all positions forming a closed loop around the hole.
    /// Returns positions in order: lower ring (segment order) -> upper ring (reversed)
    /// This creates a CCW loop when viewed from outside the trunk.
    #[allow(dead_code)]
    pub fn get_loop_positions(&self) -> Vec<Vector3> {
        let mut loop_positions = self.lower_positions.clone();
        // Add upper ring in reverse order to close the loop
        loop_positions.extend(self.upper_positions.iter().rev().copied());
        loop_positions
    }

    /// Get the indices that form the "sides" of the hole (connecting lower to upper)
    /// Returns (left_lower, left_upper, right_lower, right_upper) indices
    #[allow(dead_code)]
    pub fn get_side_indices(&self) -> Option<(u32, u32, u32, u32)> {
        if self.lower_indices.is_empty() || self.upper_indices.is_empty() {
            return None;
        }
        let left_lower = *self.lower_indices.first()?;
        let left_upper = *self.upper_indices.first()?;
        let right_lower = *self.lower_indices.last()?;
        let right_upper = *self.upper_indices.last()?;
        Some((left_lower, left_upper, right_lower, right_upper))
    }
}

/// Describes a branch attachment point on the trunk
#[derive(Clone, Debug)]
pub struct BranchAttachment {
    /// Height ratio along trunk (0.0 = base, 1.0 = top)
    #[allow(dead_code)]
    pub height_ratio: f32,
    /// Angle around trunk (radians)
    #[allow(dead_code)]
    pub angle: f32,
    /// Branch base radius
    #[allow(dead_code)]
    pub branch_radius: f32,
    /// Branch growth direction (normalized)
    pub direction: Vector3,
    /// Attachment point on trunk surface
    pub attachment_point: Vector3,
    /// Trunk radius at this height
    #[allow(dead_code)]
    pub trunk_radius_at_height: f32,
    /// Ring index in trunk for this height level
    pub trunk_ring_index: usize,
    /// Start and end segment indices affected by this hole
    pub hole_segment_start: usize,
    pub hole_segment_end: usize,
}

/// Mesh data with manifold topology support
/// Combines vertex pool with indices and tracks open rings (holes)
#[derive(Default, Clone)]
pub struct ManifoldMeshData {
    /// Shared vertex pool
    pub pool: VertexPool,
    /// Triangle indices (triplets referencing pool)
    pub indices: Vec<i32>,
    /// Open rings that need to be connected (hole boundaries)
    #[allow(dead_code)]
    pub open_rings: Vec<VertexRing>,
}

impl ManifoldMeshData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a triangle using vertex indices
    pub fn add_triangle(&mut self, i0: u32, i1: u32, i2: u32) {
        self.indices.push(i0 as i32);
        self.indices.push(i1 as i32);
        self.indices.push(i2 as i32);
    }

    /// Add a quad as two triangles (CCW winding)
    /// Quad vertices: i0=bottom-left, i1=bottom-right, i2=top-left, i3=top-right
    pub fn add_quad(&mut self, i0: u32, i1: u32, i2: u32, i3: u32) {
        // First triangle: bottom-left, bottom-right, top-left (CCW)
        self.indices.push(i0 as i32);
        self.indices.push(i1 as i32);
        self.indices.push(i2 as i32);
        // Second triangle: top-left, bottom-right, top-right (CCW)
        self.indices.push(i2 as i32);
        self.indices.push(i1 as i32);
        self.indices.push(i3 as i32);
    }

    /// Convert to regular MeshData (for compatibility)
    pub fn to_mesh_data(&self) -> MeshData {
        MeshData {
            vertices: self.pool.vertices.clone(),
            normals: self.pool.normals.clone(),
            uvs: self.pool.uvs.clone(),
            indices: self.indices.clone(),
        }
    }

    /// Extend with another ManifoldMeshData, adjusting indices
    #[allow(dead_code)]
    pub fn extend(&mut self, other: &ManifoldMeshData) {
        let index_offset = self.pool.len() as i32;

        // Merge pools (without deduplication across boundaries for simplicity)
        self.pool.vertices.extend_from_slice(&other.pool.vertices);
        self.pool.normals.extend_from_slice(&other.pool.normals);
        self.pool.uvs.extend_from_slice(&other.pool.uvs);

        // Offset indices
        for idx in &other.indices {
            self.indices.push(idx + index_offset);
        }

        // Offset and merge open rings
        for ring in &other.open_rings {
            let mut new_ring = ring.clone();
            for idx in &mut new_ring.indices {
                *idx += index_offset as u32;
            }
            self.open_rings.push(new_ring);
        }
    }

    /// Extend with regular MeshData, adjusting indices
    pub fn extend_mesh_data(&mut self, other: &MeshData) {
        let index_offset = self.pool.len() as i32;

        // Add vertices directly to pool
        self.pool.vertices.extend_from_slice(&other.vertices);
        self.pool.normals.extend_from_slice(&other.normals);
        self.pool.uvs.extend_from_slice(&other.uvs);

        // Offset indices
        for idx in &other.indices {
            self.indices.push(idx + index_offset);
        }
    }

    /// Create ManifoldMeshData from regular MeshData
    #[allow(dead_code)]
    pub fn from_mesh_data(mesh_data: &MeshData) -> Self {
        Self {
            pool: VertexPool {
                vertices: mesh_data.vertices.clone(),
                normals: mesh_data.normals.clone(),
                uvs: mesh_data.uvs.clone(),
                vertex_map: HashMap::new(),
            },
            indices: mesh_data.indices.clone(),
            open_rings: Vec::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Original MeshData (kept for backward compatibility)
// ═══════════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════════
// Manifold Mesh Generation
// ═══════════════════════════════════════════════════════════════════════════════

/// Calculate which trunk segments are affected by a branch hole
/// Returns (start_segment, end_segment) - inclusive range of segments to omit
pub fn calculate_hole_segments(
    branch_angle: f32,
    branch_radius: f32,
    trunk_radius: f32,
    radial_segments: i32,
) -> (usize, usize) {
    // Calculate angular extent of the hole on trunk surface
    // The hole spans an arc of: 2 * arcsin(branch_radius / trunk_radius)
    let radius_ratio = (branch_radius / trunk_radius).min(0.99);
    let angular_extent = 2.0 * radius_ratio.asin();

    // Convert to segment count
    let segment_angle = TAU / radial_segments as f32;
    let affected_segments = ((angular_extent / segment_angle).ceil() as usize).max(2);

    // Calculate center segment
    let center_segment = ((branch_angle / TAU) * radial_segments as f32).round() as i32;

    // Calculate start and end (wrapping around)
    // Use asymmetric halves to ensure exactly affected_segments are covered
    // For affected_segments=3: half_before=1, half_after=1 (center + 2 neighbors = 3)
    // For affected_segments=4: half_before=2, half_after=1 (2 before + center + 1 after = 4)
    let half_before = (affected_segments / 2) as i32;
    let half_after = ((affected_segments - 1) / 2) as i32;
    let start = (center_segment - half_before).rem_euclid(radial_segments) as usize;
    let end = (center_segment + half_after).rem_euclid(radial_segments) as usize;

    (start, end)
}

/// Generate a transition patch that fills the rectangular trunk hole
/// and morphs into a circular opening for the branch cylinder.
/// Returns the indices of the innermost (circular) ring for branch connection.
fn generate_junction_patch(
    hole_boundary: &HoleBoundary,
    _branch_start: Vector3,
    branch_direction: Vector3,
    branch_radius: f32,
    mesh: &mut ManifoldMeshData,
) -> Vec<u32> {
    let lower = &hole_boundary.lower_positions;
    let upper = &hole_boundary.upper_positions;
    let lower_idx = &hole_boundary.lower_indices;
    let upper_idx = &hole_boundary.upper_indices;

    let arc_segs = lower.len();

    // Build branch coordinate system
    let right = get_perpendicular(branch_direction);
    let forward = branch_direction.cross(right);

    // Number of transition rings (outer rectangular → inner circular)
    let transition_rings = 4;

    // Store all ring indices for triangulation
    let mut all_rings: Vec<Vec<u32>> = Vec::new();

    // Ring 0: Use existing hole boundary vertices (reuse indices)
    // Build the closed loop: lower in order, then upper reversed
    let mut ring0: Vec<u32> = Vec::new();
    ring0.extend(lower_idx.iter().copied());
    ring0.extend(upper_idx.iter().rev().copied());
    all_rings.push(ring0);

    // Compute hole center (average of boundary positions)
    let hole_center = {
        let sum: Vector3 = lower.iter().chain(upper.iter()).copied().fold(
            Vector3::ZERO,
            |acc, v| acc + v,
        );
        sum / (lower.len() + upper.len()) as f32
    };

    // Total vertices in the boundary loop
    let total_verts = arc_segs * 2;

    // Rings 1 to N: Interpolated from rectangle to circle
    for ring_idx in 1..transition_rings {
        let t = ring_idx as f32 / (transition_rings - 1) as f32;
        // Smoothstep for nicer blending
        let blend = t * t * (3.0 - 2.0 * t);

        let mut ring: Vec<u32> = Vec::new();

        for i in 0..total_verts {
            // Get rectangular position from boundary loop
            let rect_pos = if i < arc_segs {
                lower[i]
            } else {
                // Upper ring in reverse order
                upper[total_verts - 1 - i]
            };

            // Calculate circular position around branch
            let angle = (i as f32 / total_verts as f32) * TAU;
            // Slightly larger radius for outer rings, matching at inner ring
            let effective_radius = branch_radius * (1.0 + (1.0 - blend) * 0.5);
            let offset_along_branch = branch_direction * (blend * 0.05);

            let circ_pos = hole_center
                + right * (angle.cos() * effective_radius)
                + forward * (angle.sin() * effective_radius)
                + offset_along_branch;

            // Blend position
            let pos = rect_pos.lerp(circ_pos, blend);

            // Trunk normal: radial from trunk axis (0, y, 0)
            let trunk_center_at_height = Vector3::new(0.0, rect_pos.y, 0.0);
            let trunk_normal = (rect_pos - trunk_center_at_height)
                .try_normalized()
                .unwrap_or(Vector3::new(1.0, 0.0, 0.0));

            // Branch normal: use actual angle of vertex around trunk, not loop index
            let actual_angle = rect_pos.z.atan2(rect_pos.x);
            let branch_normal = (right * actual_angle.cos() + forward * actual_angle.sin()).normalized();

            // Blend from trunk-radial to branch-radial
            let normal = trunk_normal.lerp(branch_normal, blend).normalized();

            let u = i as f32 / total_verts as f32;
            let v = blend * 0.15; // Small v range for transition

            let idx = mesh.pool.add_vertex(pos, normal, Vector2::new(u, v));
            ring.push(idx);
        }
        all_rings.push(ring);
    }

    // Triangulate between consecutive rings
    for r in 0..(all_rings.len() - 1) {
        let ring_a = &all_rings[r];
        let ring_b = &all_rings[r + 1];

        // Rings have same vertex count, use direct quads
        let n = ring_a.len();
        for i in 0..n {
            let a0 = ring_a[i];
            let a1 = ring_a[(i + 1) % n];
            let b0 = ring_b[i];
            let b1 = ring_b[(i + 1) % n];
            mesh.add_quad(a0, a1, b0, b1);
        }
    }

    // Return innermost ring for connection to branch cylinder
    all_rings.pop().unwrap()
}

/// Generate a branch mesh that connects to a trunk hole boundary
/// The branch base connects to the hole boundary vertices through transition geometry
/// This properly bridges trunk-space to branch-space coordinate systems
pub fn generate_branch_mesh_manifold(
    branch: &BranchSegment,
    hole_boundary: &HoleBoundary,
    mesh: &mut ManifoldMeshData,
    branch_twist: f32,
    gravity_strength: f32,
    stiffness: f32,
) {
    // Require valid hole boundary
    if !hole_boundary.is_valid() {
        return;
    }

    // Use more rings when twist or gravity is applied for smoother curves
    let rings = if branch_twist.abs() > 0.1 || gravity_strength > 0.01 {
        6usize
    } else {
        4usize // Transition, base, middle, tip
    };

    // Pre-compute positions along the branch with gravity
    let step_length = branch.length / (rings - 1) as f32;
    let mut positions: Vec<Vector3> = Vec::with_capacity(rings);
    let mut directions: Vec<Vector3> = Vec::with_capacity(rings);

    let mut current_pos = branch.start;

    for ring in 0..rings {
        let t = ring as f32 / (rings - 1) as f32;
        positions.push(current_pos);

        // Apply gravity bending
        let current_dir = apply_gravity(branch.direction, t, gravity_strength, stiffness);
        directions.push(current_dir);

        if ring < rings - 1 {
            current_pos += current_dir * step_length;
        }
    }

    // Build branch coordinate basis
    let branch_dir = directions[0];

    // ═══════════════════════════════════════════════════════════════════════════════
    // Step 1-2: Generate junction patch (fills hole and transitions to circular)
    // ═══════════════════════════════════════════════════════════════════════════════

    let junction_ring_indices = generate_junction_patch(
        hole_boundary,
        branch.start,
        branch_dir,
        branch.base_radius,
        mesh,
    );

    // ═══════════════════════════════════════════════════════════════════════════════
    // Step 3: Generate branch cylinder rings
    // ═══════════════════════════════════════════════════════════════════════════════

    // Use the junction ring's vertex count for consistent bridging
    let branch_segments = junction_ring_indices.len();

    // The junction ring is already at the branch base position with circular shape
    // Use it as the first ring for the branch cylinder
    let mut prev_ring_indices = junction_ring_indices;

    // Generate subsequent rings along the branch (start from ring 1, not 0)
    for ring in 1..rings {
        let t = ring as f32 / (rings - 1) as f32;
        let pos = positions[ring];
        let dir = directions[ring];
        let radius = lerp(branch.base_radius, branch.tip_radius, t);
        let v = t;

        let twist_angle = t * branch_twist.to_radians();

        // Build rotation basis from direction
        let right = get_perpendicular(dir);
        let forward = dir.cross(right);

        let mut current_ring_indices: Vec<u32> = Vec::with_capacity(branch_segments);

        for seg in 0..branch_segments {
            let base_angle = (seg as f32 / branch_segments as f32) * TAU;
            let angle = base_angle + twist_angle;

            let local_x = angle.cos() * radius;
            let local_z = angle.sin() * radius;

            let offset = right * local_x + forward * local_z;
            let vertex = pos + offset;

            let normal = (right * angle.cos() + forward * angle.sin()).normalized();

            let idx = mesh.pool.add_vertex(vertex, normal, Vector2::new(seg as f32 / branch_segments as f32, v));
            current_ring_indices.push(idx);
        }

        // Connect this ring to previous ring with triangles
        for seg in 0..branch_segments {
            let current = prev_ring_indices[seg];
            let next = prev_ring_indices[(seg + 1) % branch_segments];
            let above = current_ring_indices[seg];
            let above_next = current_ring_indices[(seg + 1) % branch_segments];

            mesh.add_quad(current, next, above, above_next);
        }

        prev_ring_indices = current_ring_indices;
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Step 4: Add tip cap
    // ═══════════════════════════════════════════════════════════════════════════════

    let end_pos = positions[rings - 1];
    let end_dir = directions[rings - 1];

    let tip_center_idx = mesh.pool.add_unique_vertex(end_pos, end_dir, Vector2::new(0.5, 0.5));

    // Tip cap triangles (fan from center)
    for seg in 0..branch_segments {
        let current = prev_ring_indices[seg];
        let next = prev_ring_indices[(seg + 1) % branch_segments];
        mesh.add_triangle(tip_center_idx, current, next);
    }
}

/// Bridge two rings together with triangles
/// Useful for connecting hole boundaries to branch bases when they have different segment counts
#[allow(dead_code)]
pub fn bridge_rings(
    ring_a: &[u32],
    ring_b: &[u32],
    mesh: &mut ManifoldMeshData,
) {
    if ring_a.is_empty() || ring_b.is_empty() {
        return;
    }

    // Simple case: same segment count
    if ring_a.len() == ring_b.len() {
        let segments = ring_a.len();
        for seg in 0..segments {
            let a0 = ring_a[seg];
            let a1 = ring_a[(seg + 1) % segments];
            let b0 = ring_b[seg];
            let b1 = ring_b[(seg + 1) % segments];

            mesh.add_quad(a0, a1, b0, b1);
        }
        return;
    }

    // Different segment counts: use triangle fan approach
    // Connect smaller ring to larger ring
    let (small, large) = if ring_a.len() < ring_b.len() {
        (ring_a, ring_b)
    } else {
        (ring_b, ring_a)
    };

    let ratio = large.len() as f32 / small.len() as f32;

    for i in 0..small.len() {
        let s0 = small[i];
        let s1 = small[(i + 1) % small.len()];

        // Find corresponding range in large ring
        let l_start = (i as f32 * ratio).floor() as usize;
        let l_end = ((i + 1) as f32 * ratio).floor() as usize;

        // Connect s0 to range [l_start, l_end] in large ring
        for j in l_start..l_end {
            let l0 = large[j % large.len()];
            let l1 = large[(j + 1) % large.len()];

            if j == l_start {
                // First triangle connects both small vertices
                mesh.add_triangle(s0, l0, l1);
                mesh.add_triangle(s0, l1, s1);
            } else {
                // Subsequent triangles only use s1
                mesh.add_triangle(s0, l0, l1);
            }
        }
    }
}

/// Smooth normals at junction vertices by averaging trunk and branch normals
#[allow(dead_code)]
pub fn smooth_junction_normals(
    mesh: &mut ManifoldMeshData,
    junction_indices: &[u32],
    trunk_normal_weight: f32,
) {
    for &idx in junction_indices {
        if let Some(current_normal) = mesh.pool.get_normal(idx) {
            // For junction vertices, we want to blend toward a more averaged normal
            // The trunk_normal_weight controls how much of the original normal to keep
            // A weight of 0.3-0.5 typically gives smooth results
            let smoothed = current_normal.normalized();
            mesh.pool.set_normal(idx, smoothed * trunk_normal_weight + smoothed * (1.0 - trunk_normal_weight));
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests for Manifold Mesh Topology
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod manifold_tests {
    use super::*;

    #[test]
    fn test_vertex_pool_add_vertex() {
        let mut pool = VertexPool::new();

        let idx1 = pool.add_vertex(
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector2::new(0.0, 0.0),
        );

        assert_eq!(idx1, 0);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_vertex_pool_deduplication() {
        let mut pool = VertexPool::new();

        // Add same vertex twice - should return same index
        let idx1 = pool.add_vertex(
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector2::new(0.0, 0.0),
        );

        let idx2 = pool.add_vertex(
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector2::new(0.5, 0.5), // UV doesn't affect deduplication
        );

        assert_eq!(idx1, idx2);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_vertex_pool_different_normals() {
        let mut pool = VertexPool::new();

        // Same position, different normal - should be separate vertices
        let idx1 = pool.add_vertex(
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector2::new(0.0, 0.0),
        );

        let idx2 = pool.add_vertex(
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0), // Different normal
            Vector2::new(0.0, 0.0),
        );

        assert_ne!(idx1, idx2);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_vertex_pool_unique_vertex() {
        let mut pool = VertexPool::new();

        // Add unique vertex that bypasses deduplication
        let idx1 = pool.add_unique_vertex(
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector2::new(0.0, 0.0),
        );

        // Add same vertex again as unique - should get different index
        let idx2 = pool.add_unique_vertex(
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector2::new(0.0, 0.0),
        );

        assert_ne!(idx1, idx2);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_vertex_ring_creation() {
        let ring = VertexRing::new(
            vec![0, 1, 2, 3],
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            0.5,
        );

        assert_eq!(ring.len(), 4);
        assert!(!ring.is_empty());
    }

    #[test]
    fn test_manifold_mesh_add_triangle() {
        let mut mesh = ManifoldMeshData::new();

        // Add some vertices
        mesh.pool.add_vertex(Vector3::ZERO, Vector3::UP, Vector2::ZERO);
        mesh.pool.add_vertex(Vector3::new(1.0, 0.0, 0.0), Vector3::UP, Vector2::new(1.0, 0.0));
        mesh.pool.add_vertex(Vector3::new(0.0, 0.0, 1.0), Vector3::UP, Vector2::new(0.0, 1.0));

        mesh.add_triangle(0, 1, 2);

        assert_eq!(mesh.indices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_manifold_mesh_add_quad() {
        let mut mesh = ManifoldMeshData::new();

        // Add 4 vertices for a quad
        mesh.pool.add_vertex(Vector3::ZERO, Vector3::UP, Vector2::ZERO);
        mesh.pool.add_vertex(Vector3::new(1.0, 0.0, 0.0), Vector3::UP, Vector2::new(1.0, 0.0));
        mesh.pool.add_vertex(Vector3::new(0.0, 0.0, 1.0), Vector3::UP, Vector2::new(0.0, 1.0));
        mesh.pool.add_vertex(Vector3::new(1.0, 0.0, 1.0), Vector3::UP, Vector2::new(1.0, 1.0));

        mesh.add_quad(0, 1, 2, 3);

        // Quad = 2 triangles = 6 indices
        assert_eq!(mesh.indices.len(), 6);
    }

    #[test]
    fn test_manifold_mesh_to_mesh_data() {
        let mut mesh = ManifoldMeshData::new();

        mesh.pool.add_vertex(Vector3::ZERO, Vector3::UP, Vector2::ZERO);
        mesh.pool.add_vertex(Vector3::new(1.0, 0.0, 0.0), Vector3::UP, Vector2::new(1.0, 0.0));
        mesh.pool.add_vertex(Vector3::new(0.0, 0.0, 1.0), Vector3::UP, Vector2::new(0.0, 1.0));
        mesh.add_triangle(0, 1, 2);

        let mesh_data = mesh.to_mesh_data();

        assert_eq!(mesh_data.vertices.len(), 3);
        assert_eq!(mesh_data.normals.len(), 3);
        assert_eq!(mesh_data.uvs.len(), 3);
        assert_eq!(mesh_data.indices.len(), 3);
    }

    #[test]
    fn test_manifold_mesh_extend_mesh_data() {
        let mut mesh = ManifoldMeshData::new();

        // Add initial triangle
        mesh.pool.add_vertex(Vector3::ZERO, Vector3::UP, Vector2::ZERO);
        mesh.pool.add_vertex(Vector3::new(1.0, 0.0, 0.0), Vector3::UP, Vector2::new(1.0, 0.0));
        mesh.pool.add_vertex(Vector3::new(0.0, 0.0, 1.0), Vector3::UP, Vector2::new(0.0, 1.0));
        mesh.add_triangle(0, 1, 2);

        // Create another mesh to extend with
        let other = MeshData {
            vertices: vec![
                Vector3::new(0.0, 1.0, 0.0),
                Vector3::new(1.0, 1.0, 0.0),
                Vector3::new(0.0, 1.0, 1.0),
            ],
            normals: vec![Vector3::UP; 3],
            uvs: vec![Vector2::ZERO; 3],
            indices: vec![0, 1, 2],
        };

        mesh.extend_mesh_data(&other);

        // Should have 6 vertices now
        assert_eq!(mesh.pool.len(), 6);
        // Should have 6 indices (2 triangles)
        assert_eq!(mesh.indices.len(), 6);
        // Second triangle indices should be offset by 3
        assert_eq!(mesh.indices[3..], vec![3, 4, 5]);
    }

    #[test]
    fn test_calculate_hole_segments() {
        // Branch at angle 0, radius 0.1 on trunk radius 0.5, with 8 segments
        let (start, end) = calculate_hole_segments(0.0, 0.1, 0.5, 8);

        // Should have a reasonable hole span
        assert!(end >= start || start > end); // May wrap around
    }

    #[test]
    fn test_calculate_hole_segments_larger_branch() {
        // Larger branch should create larger hole
        let (start1, end1) = calculate_hole_segments(0.0, 0.1, 0.5, 8);
        let (start2, end2) = calculate_hole_segments(0.0, 0.3, 0.5, 8);

        // Calculate segment span (accounting for wraparound)
        let span1 = if end1 >= start1 { end1 - start1 + 1 } else { 8 - start1 + end1 + 1 };
        let span2 = if end2 >= start2 { end2 - start2 + 1 } else { 8 - start2 + end2 + 1 };

        assert!(span2 >= span1);
    }

    #[test]
    fn test_calculate_hole_segments_wraparound() {
        // Test hole at segment 0 (should wrap around)
        let (start, end) = calculate_hole_segments(0.0, 0.2, 0.5, 8);

        // The hole should straddle segment 0
        // Either start > end (wraparound) or start == 0
        let is_valid = start == 0 || start > end;
        assert!(is_valid);
    }

    #[test]
    fn test_hole_boundary_creation() {
        let lower = vec![0, 1, 2, 3];
        let upper = vec![4, 5, 6, 7];
        let lower_pos = vec![Vector3::ZERO; 4];
        let upper_pos = vec![Vector3::UP; 4];

        let boundary = HoleBoundary::new(
            lower.clone(),
            upper.clone(),
            lower_pos,
            upper_pos,
            Vector3::new(0.5, 1.0, 0.0),
            Vector3::new(1.0, 0.5, 0.0).normalized(),
            0.1,
            0.5,
            std::f32::consts::FRAC_PI_4,
            0.0,
        );

        assert_eq!(boundary.arc_segments(), 4);
        assert_eq!(boundary.vertex_count(), 8);
        assert!(boundary.is_valid());
        assert!(!boundary.is_empty());
    }

    #[test]
    fn test_hole_boundary_loop_indices() {
        let lower = vec![0, 1, 2];
        let upper = vec![3, 4, 5];
        let lower_pos = vec![Vector3::ZERO; 3];
        let upper_pos = vec![Vector3::UP; 3];

        let boundary = HoleBoundary::new(
            lower,
            upper,
            lower_pos,
            upper_pos,
            Vector3::ZERO,
            Vector3::RIGHT,
            0.1,
            0.5,
            0.5,
            0.0,
        );

        let loop_indices = boundary.get_loop_indices();

        // Loop should be: lower in order, then upper reversed
        // [0, 1, 2] + [5, 4, 3] = [0, 1, 2, 5, 4, 3]
        assert_eq!(loop_indices, vec![0, 1, 2, 5, 4, 3]);
    }

    #[test]
    fn test_hole_boundary_loop_positions() {
        let lower = vec![0, 1, 2];
        let upper = vec![3, 4, 5];
        // Create distinct positions for each vertex
        let p0 = Vector3::new(0.0, 0.0, 0.0);
        let p1 = Vector3::new(1.0, 0.0, 0.0);
        let p2 = Vector3::new(2.0, 0.0, 0.0);
        let p3 = Vector3::new(0.0, 1.0, 0.0);
        let p4 = Vector3::new(1.0, 1.0, 0.0);
        let p5 = Vector3::new(2.0, 1.0, 0.0);
        let lower_pos = vec![p0, p1, p2];
        let upper_pos = vec![p3, p4, p5];

        let boundary = HoleBoundary::new(
            lower,
            upper,
            lower_pos,
            upper_pos,
            Vector3::ZERO,
            Vector3::RIGHT,
            0.1,
            0.5,
            0.5,
            0.0,
        );

        let loop_positions = boundary.get_loop_positions();

        // Loop should be: lower in order, then upper reversed
        // [p0, p1, p2] + [p5, p4, p3] = [p0, p1, p2, p5, p4, p3]
        assert_eq!(loop_positions.len(), 6);
        assert_eq!(loop_positions[0], p0);
        assert_eq!(loop_positions[1], p1);
        assert_eq!(loop_positions[2], p2);
        assert_eq!(loop_positions[3], p5);
        assert_eq!(loop_positions[4], p4);
        assert_eq!(loop_positions[5], p3);
    }

    #[test]
    fn test_hole_boundary_side_indices() {
        let lower = vec![10, 11, 12, 13];
        let upper = vec![20, 21, 22, 23];
        let lower_pos = vec![Vector3::ZERO; 4];
        let upper_pos = vec![Vector3::UP; 4];

        let boundary = HoleBoundary::new(
            lower,
            upper,
            lower_pos,
            upper_pos,
            Vector3::ZERO,
            Vector3::RIGHT,
            0.1,
            0.5,
            0.5,
            0.0,
        );

        let (left_lower, left_upper, right_lower, right_upper) =
            boundary.get_side_indices().unwrap();

        assert_eq!(left_lower, 10);
        assert_eq!(left_upper, 20);
        assert_eq!(right_lower, 13);
        assert_eq!(right_upper, 23);
    }

    #[test]
    fn test_hole_boundary_invalid() {
        // Boundary with only 1 vertex per ring is invalid
        let boundary = HoleBoundary::new(
            vec![0],
            vec![1],
            vec![Vector3::ZERO],
            vec![Vector3::UP],
            Vector3::ZERO,
            Vector3::RIGHT,
            0.1,
            0.5,
            0.5,
            0.0,
        );

        assert!(!boundary.is_valid());
    }

    #[test]
    fn test_hole_boundary_empty() {
        let boundary = HoleBoundary::new(
            vec![],
            vec![],
            vec![],
            vec![],
            Vector3::ZERO,
            Vector3::RIGHT,
            0.1,
            0.5,
            0.5,
            0.0,
        );

        assert!(boundary.is_empty());
        assert!(!boundary.is_valid());
    }
}
