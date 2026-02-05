use godot::prelude::*;
use std::f32::consts::{PI, TAU};

use crate::branch::{MeshData, SeededRng};

/// Leaf geometry style
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum LeafStyle {
    #[default]
    CrossedPlanes = 0, // 2 quads at 90° (classic pixel art)
    SingleQuad = 1,    // 1 quad (needs billboard shader)
    ClusterSphere = 2, // Octahedron approximation
    StarBurst = 3,     // 3 quads at 60°
    NeedleCluster = 4, // 6 thin radiating quads (pine)
    Icosphere = 5,     // Subdivided icosahedron (low-poly rounded blob)
}

/// Foliage placement mode
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum FoliagePlacement {
    #[default]
    TerminalBranches = 0, // Only at branch tips
    AllBranches = 1, // Distributed along all branches
    TipClusters = 2, // Sphere clusters at endpoints
}

/// Leaf orientation mode
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum LeafOrientation {
    #[default]
    RadialOutward = 0, // Away from trunk
    FollowBranch = 1,     // Along branch direction
    RandomUpward = 2,     // Random with upward bias
    HorizontalSpread = 3, // Flat with random rotation
}

/// Configuration for foliage generation (extracted from PixyTree exports)
#[derive(Clone, Debug)]
pub struct FoliageConfig {
    pub enabled: bool,
    #[allow(dead_code)] // Used for future material assignment
    pub leaf_style: LeafStyle,
    pub placement: FoliagePlacement,
    pub orientation: LeafOrientation,
    pub density: f32,
    pub cluster_size: i32,
    pub leaf_size: f32,
    pub leaf_size_variation: f32,
    pub radius_threshold: f32,
    pub height_falloff: f32,
    pub leaf_droop: f32,
    pub rotation_variation: f32,
    pub use_crown_density: bool,
    #[allow(dead_code)] // Reserved for future crown-aware placement
    pub trunk_height: f32,
    #[allow(dead_code)] // Reserved for future crown-aware placement
    pub branch_start: f32,
    #[allow(dead_code)] // Reserved for future crown-aware placement
    pub branch_end: f32,
}

/// A single leaf placement point with orientation
#[derive(Clone, Debug)]
pub struct LeafPoint {
    pub position: Vector3,
    pub direction: Vector3, // Primary orientation
    pub size: f32,
    pub rotation: f32, // Random rotation around direction
}

/// Extended branch info for foliage placement
#[derive(Clone, Debug)]
pub struct BranchInfo {
    pub start: Vector3,
    pub end: Vector3,
    pub direction: Vector3,
    pub length: f32,
    #[allow(dead_code)] // Reserved for future density calculations
    pub base_radius: f32,
    pub tip_radius: f32,
    pub is_terminal: bool,
    pub height_ratio: f32, // 0.0 at branch_start, 1.0 at branch_end
}

/// Collect leaf placement points from branches
pub fn collect_leaf_points(
    branches: &[BranchInfo],
    config: &FoliageConfig,
    rng: &mut SeededRng,
) -> Vec<LeafPoint> {
    let mut points = Vec::new();

    if !config.enabled {
        return points;
    }

    for branch in branches {
        // Check radius threshold (skip thick branches for some placements)
        let normalized_radius = branch.tip_radius / 0.5; // Normalize against default trunk radius
        if normalized_radius > config.radius_threshold
            && config.placement != FoliagePlacement::AllBranches
        {
            continue;
        }

        // Filter by placement mode
        match config.placement {
            FoliagePlacement::TerminalBranches => {
                if !branch.is_terminal {
                    continue;
                }
                // Place leaves at branch tip
                add_terminal_leaves(&mut points, branch, config, rng);
            }
            FoliagePlacement::AllBranches => {
                // Distribute leaves along the branch
                add_distributed_leaves(&mut points, branch, config, rng);
            }
            FoliagePlacement::TipClusters => {
                if !branch.is_terminal {
                    continue;
                }
                // Place a cluster sphere at the tip
                add_cluster_leaves(&mut points, branch, config, rng);
            }
        }
    }

    points
}

/// Add leaves at terminal branch tips
fn add_terminal_leaves(
    points: &mut Vec<LeafPoint>,
    branch: &BranchInfo,
    config: &FoliageConfig,
    rng: &mut SeededRng,
) {
    // Calculate density modifier from height
    let height_mod = calculate_height_modifier(branch.height_ratio, config);
    let count = ((config.density * height_mod) as i32).max(1);

    for _ in 0..count {
        let size = config.leaf_size
            * (1.0 + rng.range(-config.leaf_size_variation, config.leaf_size_variation));
        let direction = calculate_leaf_direction(branch, config, rng);

        // Small offset from exact tip
        let offset = rng.range(0.0, branch.length * 0.2);
        let position = branch.end - branch.direction * offset;

        points.push(LeafPoint {
            position,
            direction,
            size,
            rotation: rng.range(0.0, TAU) * config.rotation_variation,
        });
    }
}

/// Distribute leaves along branch length
fn add_distributed_leaves(
    points: &mut Vec<LeafPoint>,
    branch: &BranchInfo,
    config: &FoliageConfig,
    rng: &mut SeededRng,
) {
    // Calculate density modifier from height
    let height_mod = calculate_height_modifier(branch.height_ratio, config);
    let base_count = (config.density * branch.length * height_mod) as i32;
    let count = base_count.max(1);

    for i in 0..count {
        // Distribute along branch (bias toward tip)
        let t = (i as f32 + rng.range(0.0, 1.0)) / count as f32;
        let biased_t = t * t; // Quadratic bias toward tip
        let position = lerp_vec3(branch.start, branch.end, 0.3 + biased_t * 0.7);

        let size = config.leaf_size
            * (1.0 + rng.range(-config.leaf_size_variation, config.leaf_size_variation));
        let direction = calculate_leaf_direction(branch, config, rng);

        points.push(LeafPoint {
            position,
            direction,
            size,
            rotation: rng.range(0.0, TAU) * config.rotation_variation,
        });
    }
}

/// Add a spherical cluster of leaves at branch tip
fn add_cluster_leaves(
    points: &mut Vec<LeafPoint>,
    branch: &BranchInfo,
    config: &FoliageConfig,
    rng: &mut SeededRng,
) {
    let height_mod = calculate_height_modifier(branch.height_ratio, config);
    let count = ((config.cluster_size as f32 * height_mod) as i32).max(1);
    let cluster_radius = config.leaf_size * 1.5;

    for _ in 0..count {
        // Random point on sphere
        let theta = rng.range(0.0, TAU);
        let phi = rng.range(0.0, PI);
        let r = cluster_radius * rng.range(0.5, 1.0);

        let offset = Vector3::new(
            r * phi.sin() * theta.cos(),
            r * phi.sin() * theta.sin(),
            r * phi.cos(),
        );

        let position = branch.end + offset;
        let direction = offset.normalized();
        let size = config.leaf_size
            * (1.0 + rng.range(-config.leaf_size_variation, config.leaf_size_variation));

        points.push(LeafPoint {
            position,
            direction,
            size,
            rotation: rng.range(0.0, TAU) * config.rotation_variation,
        });
    }
}

/// Calculate density modifier based on height within crown
fn calculate_height_modifier(height_ratio: f32, config: &FoliageConfig) -> f32 {
    if !config.use_crown_density {
        return 1.0;
    }
    // More foliage at top, less at bottom
    let base = 1.0 - config.height_falloff;
    base + height_ratio * config.height_falloff
}

/// Calculate leaf direction based on orientation mode
fn calculate_leaf_direction(
    branch: &BranchInfo,
    config: &FoliageConfig,
    rng: &mut SeededRng,
) -> Vector3 {
    let base_dir = match config.orientation {
        LeafOrientation::RadialOutward => {
            // Away from trunk (horizontal outward)
            let horizontal = Vector3::new(branch.end.x, 0.0, branch.end.z);
            if horizontal.length_squared() > 0.001 {
                horizontal.normalized()
            } else {
                Vector3::new(1.0, 0.0, 0.0)
            }
        }
        LeafOrientation::FollowBranch => branch.direction,
        LeafOrientation::RandomUpward => {
            // Random with upward bias
            let random = Vector3::new(
                rng.range(-1.0, 1.0),
                rng.range(0.2, 1.0), // Upward bias
                rng.range(-1.0, 1.0),
            );
            random.normalized()
        }
        LeafOrientation::HorizontalSpread => {
            // Flat horizontal with random rotation
            let angle = rng.range(0.0, TAU);
            Vector3::new(angle.cos(), 0.0, angle.sin())
        }
    };

    // Apply droop
    let drooped = Vector3::new(base_dir.x, base_dir.y - config.leaf_droop, base_dir.z);

    drooped.normalized()
}

/// Generate foliage mesh from leaf points
pub fn generate_foliage_mesh(points: &[LeafPoint], style: LeafStyle) -> MeshData {
    let mut mesh = MeshData::new();

    for point in points {
        let leaf_mesh = match style {
            LeafStyle::CrossedPlanes => generate_crossed_planes(point),
            LeafStyle::SingleQuad => generate_single_quad(point),
            LeafStyle::ClusterSphere => generate_cluster_sphere(point),
            LeafStyle::StarBurst => generate_star_burst(point),
            LeafStyle::NeedleCluster => generate_needle_cluster(point),
            LeafStyle::Icosphere => generate_icosphere(point),
        };
        mesh.extend(&leaf_mesh);
    }

    mesh
}

/// Generate crossed planes leaf (2 quads at 90 degrees)
fn generate_crossed_planes(point: &LeafPoint) -> MeshData {
    let mut mesh = MeshData::new();
    let half_size = point.size * 0.5;

    // Build rotation basis from direction
    let (right, up, forward) = build_basis(point.direction, point.rotation);

    // First plane (right-up)
    add_double_sided_quad(
        &mut mesh,
        point.position,
        right * half_size,
        up * half_size,
        forward,
    );

    // Second plane (forward-up, rotated 90 degrees)
    add_double_sided_quad(
        &mut mesh,
        point.position,
        forward * half_size,
        up * half_size,
        right,
    );

    mesh
}

/// Generate single quad leaf (for billboard shader)
fn generate_single_quad(point: &LeafPoint) -> MeshData {
    let mut mesh = MeshData::new();
    let half_size = point.size * 0.5;

    let (right, up, forward) = build_basis(point.direction, point.rotation);

    add_double_sided_quad(
        &mut mesh,
        point.position,
        right * half_size,
        up * half_size,
        forward,
    );

    mesh
}

/// Generate octahedron-like cluster sphere
fn generate_cluster_sphere(point: &LeafPoint) -> MeshData {
    let mut mesh = MeshData::new();
    let size = point.size * 0.4;

    // Simple octahedron vertices
    let top = point.position + Vector3::UP * size;
    let bottom = point.position - Vector3::UP * size;
    let front = point.position + Vector3::FORWARD * size;
    let back = point.position + Vector3::BACK * size;
    let left = point.position + Vector3::LEFT * size;
    let right = point.position + Vector3::RIGHT * size;

    // Top 4 faces
    add_triangle(&mut mesh, top, front, right);
    add_triangle(&mut mesh, top, right, back);
    add_triangle(&mut mesh, top, back, left);
    add_triangle(&mut mesh, top, left, front);

    // Bottom 4 faces
    add_triangle(&mut mesh, bottom, right, front);
    add_triangle(&mut mesh, bottom, back, right);
    add_triangle(&mut mesh, bottom, left, back);
    add_triangle(&mut mesh, bottom, front, left);

    mesh
}

/// Generate subdivided icosahedron (low-poly rounded blob)
fn generate_icosphere(point: &LeafPoint) -> MeshData {
    let mut mesh = MeshData::new();
    let radius = point.size;

    // Golden ratio for icosahedron vertex placement
    let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let inv_len = 1.0 / (1.0 + phi * phi).sqrt(); // normalize to unit sphere

    // 12 icosahedron vertices (normalized to unit sphere, will scale by radius later)
    let verts: [Vector3; 12] = [
        Vector3::new(-1.0, phi, 0.0) * inv_len,
        Vector3::new(1.0, phi, 0.0) * inv_len,
        Vector3::new(-1.0, -phi, 0.0) * inv_len,
        Vector3::new(1.0, -phi, 0.0) * inv_len,
        Vector3::new(0.0, -1.0, phi) * inv_len,
        Vector3::new(0.0, 1.0, phi) * inv_len,
        Vector3::new(0.0, -1.0, -phi) * inv_len,
        Vector3::new(0.0, 1.0, -phi) * inv_len,
        Vector3::new(phi, 0.0, -1.0) * inv_len,
        Vector3::new(phi, 0.0, 1.0) * inv_len,
        Vector3::new(-phi, 0.0, -1.0) * inv_len,
        Vector3::new(-phi, 0.0, 1.0) * inv_len,
    ];

    // 20 triangular faces of the icosahedron (vertex indices)
    let faces: [[usize; 3]; 20] = [
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    // Subdivide once: split each triangle into 4 by adding midpoints on edges
    // This produces 80 faces from the original 20
    for face in &faces {
        let v0 = verts[face[0]];
        let v1 = verts[face[1]];
        let v2 = verts[face[2]];

        // Midpoints projected onto unit sphere
        let m01 = midpoint_on_sphere(v0, v1);
        let m12 = midpoint_on_sphere(v1, v2);
        let m20 = midpoint_on_sphere(v2, v0);

        // 4 sub-triangles, scaled by radius and offset by position
        let tris = [
            (v0, m01, m20),
            (m01, v1, m12),
            (m20, m12, v2),
            (m01, m12, m20),
        ];

        for (a, b, c) in &tris {
            add_triangle(
                &mut mesh,
                point.position + *a * radius,
                point.position + *b * radius,
                point.position + *c * radius,
            );
        }
    }

    mesh
}

/// Compute midpoint of two vectors and project back onto the unit sphere
fn midpoint_on_sphere(a: Vector3, b: Vector3) -> Vector3 {
    let mid = Vector3::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5, (a.z + b.z) * 0.5);
    mid.normalized()
}

/// Generate 3 quads at 60 degree angles (star burst)
fn generate_star_burst(point: &LeafPoint) -> MeshData {
    let mut mesh = MeshData::new();
    let half_size = point.size * 0.5;

    let (_, up, _) = build_basis(point.direction, point.rotation);

    // 3 planes at 60 degree intervals around up axis
    for i in 0..3 {
        let angle = (i as f32 / 3.0) * TAU + point.rotation;
        let right = Vector3::new(angle.cos(), 0.0, angle.sin());
        let forward = up.cross(right).normalized();

        add_double_sided_quad(
            &mut mesh,
            point.position,
            right * half_size,
            up * half_size,
            forward,
        );
    }

    mesh
}

/// Generate needle cluster (6 thin radiating quads)
fn generate_needle_cluster(point: &LeafPoint) -> MeshData {
    let mut mesh = MeshData::new();
    let length = point.size;
    let width = point.size * 0.1; // Thin needles

    let (_, _, forward) = build_basis(point.direction, point.rotation);

    // 6 needles radiating out
    for i in 0..6 {
        let angle = (i as f32 / 6.0) * TAU + point.rotation;
        let needle_dir = Vector3::new(angle.cos(), 0.3, angle.sin()).normalized();
        let needle_right = forward.cross(needle_dir).normalized();

        // Needle quad along needle_dir
        add_double_sided_quad(
            &mut mesh,
            point.position + needle_dir * length * 0.5,
            needle_right * width,
            needle_dir * length * 0.5,
            Vector3::UP,
        );
    }

    mesh
}

/// Build orthonormal basis from direction and rotation
fn build_basis(direction: Vector3, rotation: f32) -> (Vector3, Vector3, Vector3) {
    // Default up vector, handle near-vertical directions
    let world_up = if direction.y.abs() > 0.9 {
        Vector3::FORWARD
    } else {
        Vector3::UP
    };

    let right = direction.cross(world_up).normalized();
    let up = right.cross(direction).normalized();

    // Apply rotation around direction
    let cos_r = rotation.cos();
    let sin_r = rotation.sin();
    let rotated_right = right * cos_r + up * sin_r;
    let rotated_up = up * cos_r - right * sin_r;

    (rotated_right, rotated_up, direction)
}

/// Add a double-sided quad to mesh data
fn add_double_sided_quad(
    mesh: &mut MeshData,
    center: Vector3,
    half_right: Vector3,
    half_up: Vector3,
    normal: Vector3,
) {
    let base_idx = mesh.vertices.len() as i32;

    // 4 corners
    let v0 = center - half_right - half_up;
    let v1 = center + half_right - half_up;
    let v2 = center + half_right + half_up;
    let v3 = center - half_right + half_up;

    // Front face
    mesh.vertices.extend_from_slice(&[v0, v1, v2, v3]);
    mesh.normals
        .extend_from_slice(&[normal, normal, normal, normal]);
    mesh.uvs.extend_from_slice(&[
        Vector2::new(0.0, 0.0),
        Vector2::new(1.0, 0.0),
        Vector2::new(1.0, 1.0),
        Vector2::new(0.0, 1.0),
    ]);
    mesh.indices.extend_from_slice(&[
        base_idx,
        base_idx + 1,
        base_idx + 2,
        base_idx,
        base_idx + 2,
        base_idx + 3,
    ]);

    // Back face (reversed winding, flipped normal)
    let back_idx = mesh.vertices.len() as i32;
    let back_normal = -normal;
    mesh.vertices.extend_from_slice(&[v0, v1, v2, v3]);
    mesh.normals
        .extend_from_slice(&[back_normal, back_normal, back_normal, back_normal]);
    mesh.uvs.extend_from_slice(&[
        Vector2::new(0.0, 0.0),
        Vector2::new(1.0, 0.0),
        Vector2::new(1.0, 1.0),
        Vector2::new(0.0, 1.0),
    ]);
    mesh.indices.extend_from_slice(&[
        back_idx,
        back_idx + 2,
        back_idx + 1,
        back_idx,
        back_idx + 3,
        back_idx + 2,
    ]);
}

/// Add a single triangle to mesh data
fn add_triangle(mesh: &mut MeshData, v0: Vector3, v1: Vector3, v2: Vector3) {
    let base_idx = mesh.vertices.len() as i32;

    // Calculate face normal
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let normal = edge1.cross(edge2).normalized();

    mesh.vertices.extend_from_slice(&[v0, v1, v2]);
    mesh.normals.extend_from_slice(&[normal, normal, normal]);
    mesh.uvs.extend_from_slice(&[
        Vector2::new(0.0, 0.0),
        Vector2::new(1.0, 0.0),
        Vector2::new(0.5, 1.0),
    ]);
    mesh.indices
        .extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);
}

/// Linear interpolation for Vector3
fn lerp_vec3(a: Vector3, b: Vector3, t: f32) -> Vector3 {
    Vector3::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
    )
}

/// Foliage preset values for different tree types
#[derive(Clone, Debug)]
pub struct FoliagePresetValues {
    pub enabled: bool,
    pub leaf_style: LeafStyle,
    pub placement: FoliagePlacement,
    pub orientation: LeafOrientation,
    pub density: f32,
    pub cluster_size: i32,
    pub leaf_size: f32,
    pub leaf_size_variation: f32,
    pub radius_threshold: f32,
    pub height_falloff: f32,
    pub leaf_droop: f32,
    pub rotation_variation: f32,
    pub use_crown_density: bool,
    pub separate_mesh: bool,
    pub foliage_color: Color,
}

impl Default for FoliagePresetValues {
    fn default() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::RadialOutward,
            density: 3.0,
            cluster_size: 4,
            leaf_size: 0.3,
            leaf_size_variation: 0.15,
            radius_threshold: 0.15,
            height_falloff: 0.3,
            leaf_droop: 0.2,
            rotation_variation: 0.5,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.133, 0.545, 0.133), // #228B22 Forest Green
        }
    }
}

impl FoliagePresetValues {
    /// Oak: Dense, radial outward, crossed planes
    pub fn oak() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RadialOutward,
            density: 4.0,
            cluster_size: 5,
            leaf_size: 0.35,
            leaf_size_variation: 0.2,
            radius_threshold: 0.2,
            height_falloff: 0.25,
            leaf_droop: 0.15,
            rotation_variation: 0.6,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.176, 0.314, 0.086), // #2D5016 Dark Green
        }
    }

    /// Pine: Needle clusters, follow branch, high density
    pub fn pine() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::NeedleCluster,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 5.0,
            cluster_size: 6,
            leaf_size: 0.25,
            leaf_size_variation: 0.1,
            radius_threshold: 0.25,
            height_falloff: 0.2,
            leaf_droop: 0.05,
            rotation_variation: 0.3,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.106, 0.302, 0.243), // #1B4D3E Pine Green
        }
    }

    /// Willow: Single quad (billboard), high droop, random
    pub fn willow() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::SingleQuad,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RandomUpward,
            density: 6.0,
            cluster_size: 4,
            leaf_size: 0.2,
            leaf_size_variation: 0.25,
            radius_threshold: 0.15,
            height_falloff: 0.4,
            leaf_droop: 0.6,
            rotation_variation: 0.8,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.565, 0.690, 0.376), // #90B060 Yellow-Green
        }
    }

    /// Birch: Crossed planes, terminal, small, upward bias
    pub fn birch() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::RandomUpward,
            density: 3.5,
            cluster_size: 4,
            leaf_size: 0.2,
            leaf_size_variation: 0.15,
            radius_threshold: 0.12,
            height_falloff: 0.35,
            leaf_droop: 0.1,
            rotation_variation: 0.5,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.486, 0.804, 0.486), // #7CCD7C Light Green
        }
    }

    /// Palm: Single quad, terminal, large leaves, low count
    pub fn palm() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::SingleQuad,
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 1.5,
            cluster_size: 2,
            leaf_size: 0.8,
            leaf_size_variation: 0.2,
            radius_threshold: 0.3,
            height_falloff: 0.1,
            leaf_droop: 0.4,
            rotation_variation: 0.2,
            use_crown_density: false,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.133, 0.545, 0.133), // #228B22 Forest Green
        }
    }

    /// Cypress: Cluster sphere, tip clusters, small, tight
    pub fn cypress() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::ClusterSphere,
            placement: FoliagePlacement::TipClusters,
            orientation: LeafOrientation::RadialOutward,
            density: 4.0,
            cluster_size: 8,
            leaf_size: 0.15,
            leaf_size_variation: 0.1,
            radius_threshold: 0.18,
            height_falloff: 0.15,
            leaf_droop: 0.05,
            rotation_variation: 0.4,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.208, 0.369, 0.231), // #355E3B Hunter Green
        }
    }

    /// Bonsai: Star burst, tip clusters, artistic, horizontal spread
    pub fn bonsai() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::StarBurst,
            placement: FoliagePlacement::TipClusters,
            orientation: LeafOrientation::HorizontalSpread,
            density: 2.5,
            cluster_size: 6,
            leaf_size: 0.25,
            leaf_size_variation: 0.2,
            radius_threshold: 0.2,
            height_falloff: 0.2,
            leaf_droop: 0.15,
            rotation_variation: 0.7,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.208, 0.369, 0.231), // #355E3B Hunter Green
        }
    }

    /// Maple: Dense crossed planes, radial outward, medium leaves
    pub fn maple() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RadialOutward,
            density: 4.5,
            cluster_size: 5,
            leaf_size: 0.3,
            leaf_size_variation: 0.2,
            radius_threshold: 0.18,
            height_falloff: 0.25,
            leaf_droop: 0.12,
            rotation_variation: 0.6,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.133, 0.322, 0.133), // Forest green
        }
    }

    /// Spruce: Needle clusters, follow branch, high density
    pub fn spruce() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::NeedleCluster,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 5.5,
            cluster_size: 7,
            leaf_size: 0.22,
            leaf_size_variation: 0.08,
            radius_threshold: 0.22,
            height_falloff: 0.18,
            leaf_droop: 0.08,
            rotation_variation: 0.25,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.086, 0.278, 0.212), // Blue-green
        }
    }

    /// Poplar: Crossed planes, terminal, upward bias, small dense
    pub fn poplar() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::RandomUpward,
            density: 3.5,
            cluster_size: 4,
            leaf_size: 0.22,
            leaf_size_variation: 0.12,
            radius_threshold: 0.15,
            height_falloff: 0.3,
            leaf_droop: 0.05,
            rotation_variation: 0.45,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.404, 0.545, 0.306), // Yellow-green
        }
    }

    /// Baobab: Cluster sphere, sparse, large leaves
    pub fn baobab() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::ClusterSphere,
            placement: FoliagePlacement::TipClusters,
            orientation: LeafOrientation::RadialOutward,
            density: 2.0,
            cluster_size: 5,
            leaf_size: 0.4,
            leaf_size_variation: 0.25,
            radius_threshold: 0.25,
            height_falloff: 0.15,
            leaf_droop: 0.2,
            rotation_variation: 0.5,
            use_crown_density: false,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.306, 0.463, 0.235), // Olive green
        }
    }

    /// Dragon Tree: Star burst, tip clusters at fork points
    pub fn dragon_tree() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::StarBurst,
            placement: FoliagePlacement::TipClusters,
            orientation: LeafOrientation::RadialOutward,
            density: 3.0,
            cluster_size: 8,
            leaf_size: 0.35,
            leaf_size_variation: 0.15,
            radius_threshold: 0.2,
            height_falloff: 0.1,
            leaf_droop: 0.1,
            rotation_variation: 0.4,
            use_crown_density: false,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.286, 0.443, 0.255), // Blue-green
        }
    }

    /// Japanese Maple: Crossed planes, all branches, layered, red foliage
    pub fn japanese_maple() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::HorizontalSpread,
            density: 4.0,
            cluster_size: 4,
            leaf_size: 0.18,
            leaf_size_variation: 0.15,
            radius_threshold: 0.12,
            height_falloff: 0.35,
            leaf_droop: 0.18,
            rotation_variation: 0.65,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.698, 0.133, 0.133), // Red foliage
        }
    }

    /// Redwood: Needle clusters, sparse, small
    pub fn redwood() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::NeedleCluster,
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 3.0,
            cluster_size: 5,
            leaf_size: 0.18,
            leaf_size_variation: 0.1,
            radius_threshold: 0.15,
            height_falloff: 0.25,
            leaf_droop: 0.1,
            rotation_variation: 0.3,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.12, 0.32, 0.21), // Dark green
        }
    }

    /// Elm: Dense crossed planes, vase-shaped distribution
    pub fn elm() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RadialOutward,
            density: 4.0,
            cluster_size: 5,
            leaf_size: 0.28,
            leaf_size_variation: 0.18,
            radius_threshold: 0.18,
            height_falloff: 0.25,
            leaf_droop: 0.12,
            rotation_variation: 0.55,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.2, 0.4, 0.15), // Medium green
        }
    }

    /// Fir: Dense needle clusters, follow branch direction
    pub fn fir() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::NeedleCluster,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 5.5,
            cluster_size: 7,
            leaf_size: 0.2,
            leaf_size_variation: 0.08,
            radius_threshold: 0.2,
            height_falloff: 0.15,
            leaf_droop: 0.05,
            rotation_variation: 0.25,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.1, 0.28, 0.2), // Blue-green
        }
    }

    /// Cedar: Flat spray clusters, horizontal spread
    pub fn cedar() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::ClusterSphere,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::HorizontalSpread,
            density: 4.0,
            cluster_size: 6,
            leaf_size: 0.22,
            leaf_size_variation: 0.12,
            radius_threshold: 0.2,
            height_falloff: 0.2,
            leaf_droop: 0.08,
            rotation_variation: 0.4,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.15, 0.35, 0.22), // Dark green
        }
    }

    /// Joshua Tree: Star burst clusters at branch tips
    pub fn joshua_tree() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::StarBurst,
            placement: FoliagePlacement::TipClusters,
            orientation: LeafOrientation::RadialOutward,
            density: 2.5,
            cluster_size: 10,
            leaf_size: 0.35,
            leaf_size_variation: 0.2,
            radius_threshold: 0.25,
            height_falloff: 0.1,
            leaf_droop: 0.05,
            rotation_variation: 0.5,
            use_crown_density: false,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.35, 0.45, 0.25), // Yellow-green
        }
    }

    /// Olive: Small silver-green leaves, sparse
    pub fn olive() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RandomUpward,
            density: 3.5,
            cluster_size: 4,
            leaf_size: 0.15,
            leaf_size_variation: 0.15,
            radius_threshold: 0.15,
            height_falloff: 0.2,
            leaf_droop: 0.1,
            rotation_variation: 0.6,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.4, 0.5, 0.35), // Silver-green
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 2: New Species Foliage Presets
    // ═══════════════════════════════════════════════════════════════

    /// Cherry Blossom: Pink-white flower clusters
    pub fn cherry_blossom() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RandomUpward,
            density: 5.0, // Dense blossoms
            cluster_size: 5,
            leaf_size: 0.22,
            leaf_size_variation: 0.18,
            radius_threshold: 0.15,
            height_falloff: 0.25,
            leaf_droop: 0.12,
            rotation_variation: 0.7,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(1.0, 0.85, 0.88), // Pink-white
        }
    }

    /// Acacia: Sparse, feathery foliage
    pub fn acacia() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::TipClusters,
            orientation: LeafOrientation::HorizontalSpread,
            density: 2.5, // Sparse savanna look
            cluster_size: 6,
            leaf_size: 0.28,
            leaf_size_variation: 0.2,
            radius_threshold: 0.2,
            height_falloff: 0.1,
            leaf_droop: 0.08,
            rotation_variation: 0.5,
            use_crown_density: false, // Uniform flat canopy
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.35, 0.5, 0.25), // Olive green
        }
    }

    /// Beech: Dense, rich green foliage
    pub fn beech() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RadialOutward,
            density: 5.0, // Very dense
            cluster_size: 5,
            leaf_size: 0.25,
            leaf_size_variation: 0.15,
            radius_threshold: 0.18,
            height_falloff: 0.22,
            leaf_droop: 0.1,
            rotation_variation: 0.55,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.2, 0.4, 0.15), // Rich green
        }
    }

    /// Ginkgo: Fan-shaped leaves, light yellow-green
    pub fn ginkgo() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::RandomUpward,
            density: 3.5,
            cluster_size: 4,
            leaf_size: 0.3, // Distinctive fan shape
            leaf_size_variation: 0.18,
            radius_threshold: 0.12,
            height_falloff: 0.3,
            leaf_droop: 0.05, // Leaves hold horizontal
            rotation_variation: 0.6,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.6, 0.7, 0.3), // Light yellow-green
        }
    }

    /// Weeping Cherry: Cascading pink blossoms
    pub fn weeping_cherry() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::SingleQuad, // Billboard for cascade effect
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RandomUpward,
            density: 5.5, // Dense cascade
            cluster_size: 4,
            leaf_size: 0.18,
            leaf_size_variation: 0.22,
            radius_threshold: 0.12,
            height_falloff: 0.4, // More at tips
            leaf_droop: 0.5,     // Heavy droop
            rotation_variation: 0.75,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(1.0, 0.8, 0.85), // Pink
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 3: Fantasy Foliage Presets
    // ═══════════════════════════════════════════════════════════════

    /// World Tree: Dense, ancient foliage
    pub fn world_tree() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::Icosphere, // Large rounded canopy blobs
            placement: FoliagePlacement::TipClusters,
            orientation: LeafOrientation::RadialOutward,
            density: 3.0,
            cluster_size: 10, // Large clusters
            leaf_size: 1.5,   // Massive scale foliage
            leaf_size_variation: 0.25,
            radius_threshold: 0.3,
            height_falloff: 0.2,
            leaf_droop: 0.15,
            rotation_variation: 0.5,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.15, 0.35, 0.12), // Deep forest green
        }
    }

    /// Corrupted: Sparse, dark, diseased foliage
    pub fn corrupted() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::StarBurst, // Spiky, aggressive
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::RandomUpward,
            density: 1.5, // Sparse
            cluster_size: 3,
            leaf_size: 0.25,
            leaf_size_variation: 0.3,
            radius_threshold: 0.12,
            height_falloff: 0.1,
            leaf_droop: 0.25,
            rotation_variation: 0.8,
            use_crown_density: false,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.25, 0.1, 0.3), // Dark purple
        }
    }

    /// Glowing: Dense magical foliage (emission-ready color)
    pub fn glowing() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::Icosphere, // Rounded for glow effect
            placement: FoliagePlacement::TipClusters,
            orientation: LeafOrientation::RadialOutward,
            density: 4.0, // Dense for bright effect
            cluster_size: 8,
            leaf_size: 0.35,
            leaf_size_variation: 0.2,
            radius_threshold: 0.15,
            height_falloff: 0.15,
            leaf_droop: 0.08,
            rotation_variation: 0.4,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.4, 0.9, 0.6), // Bright magical green
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 4: New Species Foliage Presets
    // ═══════════════════════════════════════════════════════════════

    /// Blue Spruce: Blue-gray needle clusters
    pub fn blue_spruce() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::NeedleCluster,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 5.5,
            cluster_size: 7,
            leaf_size: 0.22,
            leaf_size_variation: 0.08,
            radius_threshold: 0.22,
            height_falloff: 0.18,
            leaf_droop: 0.05,
            rotation_variation: 0.25,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.45, 0.55, 0.65), // Blue-gray
        }
    }

    /// Douglas Fir: Dense needle clusters, flat sprays
    pub fn douglas_fir() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::NeedleCluster,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 5.0,
            cluster_size: 6,
            leaf_size: 0.2,
            leaf_size_variation: 0.1,
            radius_threshold: 0.2,
            height_falloff: 0.15,
            leaf_droop: 0.08,
            rotation_variation: 0.3,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.12, 0.32, 0.18), // Dark green
        }
    }

    /// Ponderosa Pine: Long needle clusters
    pub fn ponderosa_pine() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::NeedleCluster,
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 4.0,
            cluster_size: 5,
            leaf_size: 0.3, // Longer needles
            leaf_size_variation: 0.12,
            radius_threshold: 0.18,
            height_falloff: 0.2,
            leaf_droop: 0.1,
            rotation_variation: 0.35,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.15, 0.35, 0.2), // Yellow-green
        }
    }

    /// Bristlecone Pine: Sparse, ancient-looking needles
    pub fn bristlecone_pine() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::NeedleCluster,
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 2.0, // Sparse
            cluster_size: 4,
            leaf_size: 0.18,
            leaf_size_variation: 0.15,
            radius_threshold: 0.15,
            height_falloff: 0.1,
            leaf_droop: 0.05,
            rotation_variation: 0.4,
            use_crown_density: false,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.2, 0.35, 0.25), // Dark green
        }
    }

    /// Ash: Compound leaves, open crown
    pub fn ash() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RadialOutward,
            density: 3.5,
            cluster_size: 4,
            leaf_size: 0.28,
            leaf_size_variation: 0.18,
            radius_threshold: 0.18,
            height_falloff: 0.25,
            leaf_droop: 0.12,
            rotation_variation: 0.55,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.2, 0.42, 0.18), // Medium green
        }
    }

    /// Linden: Dense heart-shaped leaves
    pub fn linden() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RadialOutward,
            density: 5.0, // Very dense
            cluster_size: 5,
            leaf_size: 0.25,
            leaf_size_variation: 0.15,
            radius_threshold: 0.18,
            height_falloff: 0.22,
            leaf_droop: 0.1,
            rotation_variation: 0.5,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.22, 0.45, 0.18), // Rich green
        }
    }

    /// Sycamore: Large palmate leaves
    pub fn sycamore() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RadialOutward,
            density: 3.5,
            cluster_size: 4,
            leaf_size: 0.35, // Large leaves
            leaf_size_variation: 0.2,
            radius_threshold: 0.2,
            height_falloff: 0.25,
            leaf_droop: 0.15,
            rotation_variation: 0.6,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.25, 0.45, 0.2), // Medium green
        }
    }

    /// Aspen: Small trembling leaves with high rotation variation
    pub fn aspen() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::SingleQuad, // For trembling effect
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::RandomUpward,
            density: 4.0,
            cluster_size: 5,
            leaf_size: 0.18, // Small round leaves
            leaf_size_variation: 0.12,
            radius_threshold: 0.12,
            height_falloff: 0.3,
            leaf_droop: 0.05,
            rotation_variation: 0.8, // High rotation for trembling
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.4, 0.55, 0.3), // Light green
        }
    }

    /// Royal Palm: Large arching fronds
    pub fn royal_palm() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::SingleQuad,
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 1.5,
            cluster_size: 2,
            leaf_size: 0.9, // Large fronds
            leaf_size_variation: 0.15,
            radius_threshold: 0.25,
            height_falloff: 0.1,
            leaf_droop: 0.35,
            rotation_variation: 0.15,
            use_crown_density: false,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.15, 0.5, 0.2), // Dark green
        }
    }

    /// Fan Palm: Palmate fronds
    pub fn fan_palm() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::StarBurst, // Fan shape
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::FollowBranch,
            density: 2.0,
            cluster_size: 3,
            leaf_size: 0.7, // Large fan fronds
            leaf_size_variation: 0.18,
            radius_threshold: 0.25,
            height_falloff: 0.1,
            leaf_droop: 0.25,
            rotation_variation: 0.2,
            use_crown_density: false,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.2, 0.55, 0.25), // Bright green
        }
    }

    /// Eucalyptus: Hanging sickle-shaped leaves
    pub fn eucalyptus() -> Self {
        Self {
            enabled: true,
            leaf_style: LeafStyle::SingleQuad, // Hanging
            placement: FoliagePlacement::AllBranches,
            orientation: LeafOrientation::RandomUpward,
            density: 4.0,
            cluster_size: 4,
            leaf_size: 0.22,
            leaf_size_variation: 0.18,
            radius_threshold: 0.15,
            height_falloff: 0.3,
            leaf_droop: 0.45, // Heavy droop
            rotation_variation: 0.7,
            use_crown_density: true,
            separate_mesh: true,
            foliage_color: Color::from_rgb(0.35, 0.5, 0.4), // Blue-green
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaf_style_default() {
        assert_eq!(LeafStyle::default(), LeafStyle::CrossedPlanes);
    }

    #[test]
    fn test_foliage_placement_default() {
        assert_eq!(
            FoliagePlacement::default(),
            FoliagePlacement::TerminalBranches
        );
    }

    #[test]
    fn test_leaf_orientation_default() {
        assert_eq!(LeafOrientation::default(), LeafOrientation::RadialOutward);
    }

    #[test]
    fn test_build_basis_produces_orthonormal() {
        let direction = Vector3::new(1.0, 0.5, 0.0).normalized();
        let (right, up, forward) = build_basis(direction, 0.0);

        // Check orthogonality
        assert!(right.dot(up).abs() < 0.001);
        assert!(right.dot(forward).abs() < 0.001);
        assert!(up.dot(forward).abs() < 0.001);

        // Check unit length
        assert!((right.length() - 1.0).abs() < 0.001);
        assert!((up.length() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_generate_crossed_planes_vertex_count() {
        let point = LeafPoint {
            position: Vector3::ZERO,
            direction: Vector3::UP,
            size: 1.0,
            rotation: 0.0,
        };
        let mesh = generate_crossed_planes(&point);
        // 2 planes × 2 sides × 4 verts = 16 vertices
        assert_eq!(mesh.vertices.len(), 16);
    }

    #[test]
    fn test_generate_single_quad_vertex_count() {
        let point = LeafPoint {
            position: Vector3::ZERO,
            direction: Vector3::UP,
            size: 1.0,
            rotation: 0.0,
        };
        let mesh = generate_single_quad(&point);
        // 1 plane × 2 sides × 4 verts = 8 vertices
        assert_eq!(mesh.vertices.len(), 8);
    }

    #[test]
    fn test_generate_cluster_sphere_vertex_count() {
        let point = LeafPoint {
            position: Vector3::ZERO,
            direction: Vector3::UP,
            size: 1.0,
            rotation: 0.0,
        };
        let mesh = generate_cluster_sphere(&point);
        // 8 triangles × 3 verts = 24 vertices
        assert_eq!(mesh.vertices.len(), 24);
    }

    #[test]
    fn test_generate_star_burst_vertex_count() {
        let point = LeafPoint {
            position: Vector3::ZERO,
            direction: Vector3::UP,
            size: 1.0,
            rotation: 0.0,
        };
        let mesh = generate_star_burst(&point);
        // 3 planes × 2 sides × 4 verts = 24 vertices
        assert_eq!(mesh.vertices.len(), 24);
    }

    #[test]
    fn test_generate_needle_cluster_vertex_count() {
        let point = LeafPoint {
            position: Vector3::ZERO,
            direction: Vector3::UP,
            size: 1.0,
            rotation: 0.0,
        };
        let mesh = generate_needle_cluster(&point);
        // 6 needles × 2 sides × 4 verts = 48 vertices
        assert_eq!(mesh.vertices.len(), 48);
    }

    #[test]
    fn test_foliage_preset_oak() {
        let oak = FoliagePresetValues::oak();
        assert_eq!(oak.leaf_style, LeafStyle::CrossedPlanes);
        assert_eq!(oak.placement, FoliagePlacement::AllBranches);
        assert!(oak.density > 3.0);
    }

    #[test]
    fn test_foliage_preset_pine() {
        let pine = FoliagePresetValues::pine();
        assert_eq!(pine.leaf_style, LeafStyle::NeedleCluster);
        assert_eq!(pine.orientation, LeafOrientation::FollowBranch);
    }

    #[test]
    fn test_foliage_preset_palm() {
        let palm = FoliagePresetValues::palm();
        assert_eq!(palm.placement, FoliagePlacement::TerminalBranches);
        assert!(palm.leaf_size > 0.5); // Large leaves
        assert!(palm.density < 2.0); // Low count
    }

    #[test]
    fn test_collect_leaf_points_disabled() {
        let config = FoliageConfig {
            enabled: false,
            leaf_style: LeafStyle::CrossedPlanes,
            placement: FoliagePlacement::TerminalBranches,
            orientation: LeafOrientation::RadialOutward,
            density: 3.0,
            cluster_size: 4,
            leaf_size: 0.3,
            leaf_size_variation: 0.15,
            radius_threshold: 0.15,
            height_falloff: 0.3,
            leaf_droop: 0.2,
            rotation_variation: 0.5,
            use_crown_density: true,
            trunk_height: 5.0,
            branch_start: 0.3,
            branch_end: 0.9,
        };
        let mut rng = SeededRng::new(42);
        let branches = vec![];
        let points = collect_leaf_points(&branches, &config, &mut rng);
        assert!(points.is_empty());
    }
}
