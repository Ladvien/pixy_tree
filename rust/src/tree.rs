use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, Engine, MeshInstance3D};
use godot::prelude::*;

use crate::branch::{
    generate_branch_mesh, generate_branch_origins, generate_sub_branches, BranchConfig,
    BranchSegment, MeshData, SeededRng,
};
use crate::crown_shape::CrownShape;
use crate::foliage::{
    collect_leaf_points, generate_foliage_mesh, BranchInfo, FoliageConfig, FoliagePlacement,
    FoliagePresetValues, LeafOrientation, LeafStyle,
};
use crate::tree_preset::{TreePreset, TreePresetValues};

#[derive(GodotClass)]
#[class(base=Node3D, init, tool)]
pub struct PixyTree {
    base: Base<Node3D>,

    // ═══════════════════════════════════════════
    // Preset
    // ═══════════════════════════════════════════
    #[export]
    #[var(get = get_preset, set = set_preset)]
    #[init(val = TreePreset::Custom)]
    preset: TreePreset,

    // ═══════════════════════════════════════════
    // Trunk Settings
    // ═══════════════════════════════════════════
    #[export(range = (0.1, 50.0, 0.1))]
    #[init(val = 5.0)]
    trunk_height: f32,

    #[export(range = (0.05, 5.0, 0.05))]
    #[init(val = 0.5)]
    trunk_radius: f32,

    #[export(range = (3.0, 32.0, 1.0))]
    #[init(val = 8)]
    radial_segments: i32,

    #[export(range = (1.0, 16.0, 1.0))]
    #[init(val = 4)]
    height_segments: i32,

    // ═══════════════════════════════════════════
    // Branch Settings
    // ═══════════════════════════════════════════
    /// Where branches start on trunk (0-1 ratio of height)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.3)]
    branch_start: f32,

    /// Where branches end on trunk (0-1 ratio of height)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.9)]
    branch_end: f32,

    /// Branches per unit length
    #[export(range = (0.1, 5.0, 0.1))]
    #[init(val = 1.0)]
    branch_density: f32,

    /// Branch length relative to trunk height
    #[export(range = (0.1, 1.0, 0.05))]
    #[init(val = 0.4)]
    branch_length: f32,

    /// Angle from trunk (degrees, 0=up, 90=horizontal)
    #[export(range = (0.0, 90.0, 1.0))]
    #[init(val = 45.0)]
    branch_angle: f32,

    /// Branch radius relative to trunk radius at attachment point
    #[export(range = (0.1, 0.8, 0.05))]
    #[init(val = 0.3)]
    branch_radius_ratio: f32,

    /// Taper from base to tip (0=none, 1=point)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.7)]
    branch_taper: f32,

    /// Spiral angle between branches (137.5° = golden angle)
    #[export(range = (0.0, 360.0, 0.5))]
    #[init(val = 137.5)]
    phyllotaxis_angle: f32,

    /// Direction randomness
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.2)]
    branch_randomness: f32,

    /// Upward growth tendency
    #[export(range = (-1.0, 1.0, 0.05))]
    #[init(val = 0.1)]
    up_attraction: f32,

    /// Sub-branch recursion levels (0=none)
    #[export(range = (0.0, 3.0, 1.0))]
    #[init(val = 1)]
    branch_recursion: i32,

    /// Sub-branches per branch
    #[export(range = (0.0, 5.0, 1.0))]
    #[init(val = 2)]
    sub_branch_count: i32,

    /// Length multiplier per recursion level
    #[export(range = (0.3, 0.8, 0.05))]
    #[init(val = 0.5)]
    sub_branch_scale: f32,

    // ═══════════════════════════════════════════
    // Crown Shape
    // ═══════════════════════════════════════════
    /// Crown shape envelope that modulates branch length based on height
    #[export]
    #[init(val = CrownShape::Cylindrical)]
    crown_shape: CrownShape,

    /// How much the crown shape affects branch length (0=uniform, 1=fully shaped)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 1.0)]
    crown_influence: f32,

    // ═══════════════════════════════════════════
    // Foliage Settings
    // ═══════════════════════════════════════════
    /// Enable foliage generation
    #[export]
    #[init(val = true)]
    foliage_enabled: bool,

    /// Leaf geometry style
    #[export]
    #[init(val = LeafStyle::CrossedPlanes)]
    leaf_style: LeafStyle,

    /// How foliage is placed on branches
    #[export]
    #[init(val = FoliagePlacement::TerminalBranches)]
    foliage_placement: FoliagePlacement,

    /// Leaf orientation mode
    #[export]
    #[init(val = LeafOrientation::RadialOutward)]
    leaf_orientation: LeafOrientation,

    /// Foliage density (leaves per unit)
    #[export(range = (0.5, 10.0, 0.5))]
    #[init(val = 3.0)]
    foliage_density: f32,

    /// Leaves per cluster (for TipClusters placement)
    #[export(range = (1.0, 12.0, 1.0))]
    #[init(val = 4)]
    cluster_size: i32,

    /// Base leaf size
    #[export(range = (0.05, 2.0, 0.05))]
    #[init(val = 0.3)]
    leaf_size: f32,

    /// Random variation in leaf size (0-0.5)
    #[export(range = (0.0, 0.5, 0.05))]
    #[init(val = 0.15)]
    leaf_size_variation: f32,

    /// Branch radius threshold for foliage (skip thicker branches)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.15)]
    foliage_radius_threshold: f32,

    /// Density falloff from bottom to top of crown
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.3)]
    foliage_height_falloff: f32,

    /// Downward droop amount for leaves
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.2)]
    leaf_droop: f32,

    /// Random rotation variation for leaves
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.5)]
    leaf_rotation_variation: f32,

    /// Use crown shape to modulate foliage density
    #[export]
    #[init(val = true)]
    use_crown_foliage_density: bool,

    /// Create foliage as separate mesh (for different materials)
    #[export]
    #[init(val = true)]
    separate_foliage_mesh: bool,

    // ═══════════════════════════════════════════
    // Generation
    // ═══════════════════════════════════════════
    #[export]
    #[init(val = 42)]
    seed: i32,

    // ═══════════════════════════════════════════
    // Auto-Regeneration
    // ═══════════════════════════════════════════
    /// Enable auto-regeneration when properties change in editor
    #[export]
    #[init(val = true)]
    auto_regenerate: bool,

    // Internal state (not exported)
    #[init(val = None)]
    mesh_instance: Option<Gd<MeshInstance3D>>,

    #[init(val = None)]
    foliage_mesh_instance: Option<Gd<MeshInstance3D>>,

    /// Hash of properties for change detection
    #[init(val = 0)]
    last_property_hash: u64,

    /// Debounce timer (seconds remaining)
    #[init(val = 0.0)]
    debounce_remaining: f64,
}

#[godot_api]
impl INode3D for PixyTree {
    fn ready(&mut self) {
        // Could auto-generate here if desired
    }

    fn process(&mut self, delta: f64) {
        if !self.auto_regenerate {
            return;
        }

        // Only run in editor
        if !Engine::singleton().is_editor_hint() {
            return;
        }

        let current_hash = self.compute_property_hash();

        if current_hash != self.last_property_hash {
            // Property changed - start/restart debounce
            self.debounce_remaining = 0.3; // 300ms debounce
            self.last_property_hash = current_hash;
        }

        if self.debounce_remaining > 0.0 {
            self.debounce_remaining -= delta;
            if self.debounce_remaining <= 0.0 {
                self.debounce_remaining = 0.0;
                self.generate();
            }
        }
    }
}

#[godot_api]
impl PixyTree {
    #[signal]
    fn tree_generated(height: f32, radius: f32);

    #[func]
    fn get_preset(&self) -> TreePreset {
        self.preset
    }

    #[func]
    fn set_preset(&mut self, value: TreePreset) {
        self.preset = value;
        if let Some(values) = value.get_values() {
            self.apply_preset_values(&values);
        }
    }

    fn apply_preset_values(&mut self, values: &TreePresetValues) {
        // Trunk
        self.trunk_height = values.trunk_height;
        self.trunk_radius = values.trunk_radius;
        self.radial_segments = values.radial_segments;
        self.height_segments = values.height_segments;

        // Branch
        self.branch_start = values.branch_start;
        self.branch_end = values.branch_end;
        self.branch_density = values.branch_density;
        self.branch_length = values.branch_length;
        self.branch_angle = values.branch_angle;
        self.branch_radius_ratio = values.branch_radius_ratio;
        self.branch_taper = values.branch_taper;
        self.phyllotaxis_angle = values.phyllotaxis_angle;
        self.branch_randomness = values.branch_randomness;
        self.up_attraction = values.up_attraction;
        self.branch_recursion = values.branch_recursion;
        self.sub_branch_count = values.sub_branch_count;
        self.sub_branch_scale = values.sub_branch_scale;

        // Crown
        self.crown_shape = values.crown_shape;
        self.crown_influence = values.crown_influence;

        // Foliage
        if let Some(foliage) = &values.foliage {
            self.apply_foliage_preset_values(foliage);
        }
    }

    /// Compute a hash of all generation-relevant properties for change detection
    fn compute_property_hash(&self) -> u64 {
        let mut hash = 0u64;
        hash = hash.wrapping_add((self.preset as u64).wrapping_mul(29));
        hash = hash.wrapping_add((self.trunk_height.to_bits() as u64).wrapping_mul(31));
        hash = hash.wrapping_add((self.trunk_radius.to_bits() as u64).wrapping_mul(37));
        hash = hash.wrapping_add((self.radial_segments as u64).wrapping_mul(41));
        hash = hash.wrapping_add((self.height_segments as u64).wrapping_mul(43));
        hash = hash.wrapping_add((self.branch_start.to_bits() as u64).wrapping_mul(47));
        hash = hash.wrapping_add((self.branch_end.to_bits() as u64).wrapping_mul(53));
        hash = hash.wrapping_add((self.branch_density.to_bits() as u64).wrapping_mul(59));
        hash = hash.wrapping_add((self.branch_length.to_bits() as u64).wrapping_mul(61));
        hash = hash.wrapping_add((self.branch_angle.to_bits() as u64).wrapping_mul(67));
        hash = hash.wrapping_add((self.branch_radius_ratio.to_bits() as u64).wrapping_mul(71));
        hash = hash.wrapping_add((self.branch_taper.to_bits() as u64).wrapping_mul(73));
        hash = hash.wrapping_add((self.phyllotaxis_angle.to_bits() as u64).wrapping_mul(79));
        hash = hash.wrapping_add((self.branch_randomness.to_bits() as u64).wrapping_mul(83));
        hash = hash.wrapping_add((self.up_attraction.to_bits() as u64).wrapping_mul(89));
        hash = hash.wrapping_add((self.branch_recursion as u64).wrapping_mul(97));
        hash = hash.wrapping_add((self.sub_branch_count as u64).wrapping_mul(101));
        hash = hash.wrapping_add((self.sub_branch_scale.to_bits() as u64).wrapping_mul(103));
        hash = hash.wrapping_add((self.crown_shape as u64).wrapping_mul(109));
        hash = hash.wrapping_add((self.crown_influence.to_bits() as u64).wrapping_mul(113));
        hash = hash.wrapping_add((self.seed as u64).wrapping_mul(127));
        // Foliage parameters
        hash = hash.wrapping_add((self.foliage_enabled as u64).wrapping_mul(131));
        hash = hash.wrapping_add((self.leaf_style as u64).wrapping_mul(137));
        hash = hash.wrapping_add((self.foliage_placement as u64).wrapping_mul(139));
        hash = hash.wrapping_add((self.leaf_orientation as u64).wrapping_mul(149));
        hash = hash.wrapping_add((self.foliage_density.to_bits() as u64).wrapping_mul(151));
        hash = hash.wrapping_add((self.cluster_size as u64).wrapping_mul(157));
        hash = hash.wrapping_add((self.leaf_size.to_bits() as u64).wrapping_mul(163));
        hash = hash.wrapping_add((self.leaf_size_variation.to_bits() as u64).wrapping_mul(167));
        hash =
            hash.wrapping_add((self.foliage_radius_threshold.to_bits() as u64).wrapping_mul(173));
        hash = hash.wrapping_add((self.foliage_height_falloff.to_bits() as u64).wrapping_mul(179));
        hash = hash.wrapping_add((self.leaf_droop.to_bits() as u64).wrapping_mul(181));
        hash = hash.wrapping_add((self.leaf_rotation_variation.to_bits() as u64).wrapping_mul(191));
        hash = hash.wrapping_add((self.use_crown_foliage_density as u64).wrapping_mul(193));
        hash = hash.wrapping_add((self.separate_foliage_mesh as u64).wrapping_mul(197));
        hash
    }

    #[func]
    pub fn generate(&mut self) {
        self.clear();

        // 1. Generate trunk mesh data
        let mut mesh_data = self.create_trunk_mesh_data();

        // 2. Generate branches
        let config = self.create_branch_config();
        let mut rng = SeededRng::new(self.seed);

        let primary_branches = generate_branch_origins(&config, &mut rng);
        let branch_segments = (self.radial_segments / 2).max(4);

        // Collect all branches for foliage generation
        let mut all_branches: Vec<BranchSegment> = Vec::new();

        for branch in &primary_branches {
            // Add primary branch mesh
            let branch_mesh = generate_branch_mesh(branch, branch_segments);
            mesh_data.extend(&branch_mesh);

            all_branches.push(branch.clone());

            // Add sub-branches recursively
            let sub_branches = generate_sub_branches(branch, &config, &mut rng, 0);
            for sub in &sub_branches {
                let sub_mesh = generate_branch_mesh(sub, 4); // fewer segments for sub-branches
                mesh_data.extend(&sub_mesh);
                all_branches.push(sub.clone());
            }
        }

        // 3. Build final trunk/branch mesh
        let mesh = self.build_array_mesh(
            mesh_data.vertices,
            mesh_data.normals,
            mesh_data.uvs,
            mesh_data.indices,
        );
        self.apply_mesh(mesh);

        // 4. Generate foliage
        if self.foliage_enabled {
            let foliage_config = self.create_foliage_config();
            let branch_infos = self.branches_to_branch_infos(&all_branches);
            let leaves = collect_leaf_points(&branch_infos, &foliage_config, &mut rng);

            if !leaves.is_empty() {
                let foliage_mesh_data = generate_foliage_mesh(&leaves, self.leaf_style);

                if self.separate_foliage_mesh {
                    // Create separate mesh instance for foliage
                    let foliage_mesh = self.build_array_mesh(
                        foliage_mesh_data.vertices,
                        foliage_mesh_data.normals,
                        foliage_mesh_data.uvs,
                        foliage_mesh_data.indices,
                    );
                    self.apply_foliage_mesh(foliage_mesh);
                } else {
                    // Would need to rebuild trunk mesh with foliage combined
                    // For now, just create separate mesh anyway
                    let foliage_mesh = self.build_array_mesh(
                        foliage_mesh_data.vertices,
                        foliage_mesh_data.normals,
                        foliage_mesh_data.uvs,
                        foliage_mesh_data.indices,
                    );
                    self.apply_foliage_mesh(foliage_mesh);
                }
            }
        }

        // 5. Emit signal with tree dimensions for camera framing
        let height = self.trunk_height;
        let radius = self.trunk_radius;
        self.base_mut().emit_signal(
            "tree_generated",
            &[height.to_variant(), radius.to_variant()],
        );
    }

    fn create_branch_config(&self) -> BranchConfig {
        BranchConfig {
            trunk_height: self.trunk_height,
            trunk_radius: self.trunk_radius,
            branch_start: self.branch_start,
            branch_end: self.branch_end,
            branch_density: self.branch_density,
            branch_length: self.branch_length,
            branch_angle: self.branch_angle,
            branch_radius_ratio: self.branch_radius_ratio,
            branch_taper: self.branch_taper,
            phyllotaxis_angle: self.phyllotaxis_angle,
            branch_randomness: self.branch_randomness,
            up_attraction: self.up_attraction,
            branch_recursion: self.branch_recursion,
            sub_branch_count: self.sub_branch_count,
            sub_branch_scale: self.sub_branch_scale,
            radial_segments: self.radial_segments,
            crown_shape: self.crown_shape,
            crown_influence: self.crown_influence,
        }
    }

    fn create_foliage_config(&self) -> FoliageConfig {
        FoliageConfig {
            enabled: self.foliage_enabled,
            leaf_style: self.leaf_style,
            placement: self.foliage_placement,
            orientation: self.leaf_orientation,
            density: self.foliage_density,
            cluster_size: self.cluster_size,
            leaf_size: self.leaf_size,
            leaf_size_variation: self.leaf_size_variation,
            radius_threshold: self.foliage_radius_threshold,
            height_falloff: self.foliage_height_falloff,
            leaf_droop: self.leaf_droop,
            rotation_variation: self.leaf_rotation_variation,
            use_crown_density: self.use_crown_foliage_density,
            trunk_height: self.trunk_height,
            branch_start: self.branch_start,
            branch_end: self.branch_end,
        }
    }

    fn branches_to_branch_infos(&self, branches: &[BranchSegment]) -> Vec<BranchInfo> {
        branches
            .iter()
            .map(|b| {
                let end = b.start + b.direction * b.length;
                BranchInfo {
                    start: b.start,
                    end,
                    direction: b.direction,
                    length: b.length,
                    base_radius: b.base_radius,
                    tip_radius: b.tip_radius,
                    is_terminal: b.is_terminal,
                    height_ratio: b.height_ratio,
                }
            })
            .collect()
    }

    #[func]
    pub fn clear(&mut self) {
        if let Some(ref mut instance) = self.mesh_instance {
            if instance.is_instance_valid() {
                instance.queue_free();
            }
        }
        self.mesh_instance = None;

        if let Some(ref mut instance) = self.foliage_mesh_instance {
            if instance.is_instance_valid() {
                instance.queue_free();
            }
        }
        self.foliage_mesh_instance = None;
    }

    fn create_trunk_mesh_data(&self) -> MeshData {
        let mut mesh = MeshData::new();

        let segments = self.radial_segments.max(3) as usize;
        let rings = self.height_segments.max(1) as usize + 1;

        // Generate rings of vertices for the cylinder sides
        for ring in 0..rings {
            let y = (ring as f32 / (rings - 1) as f32) * self.trunk_height;
            let v = ring as f32 / (rings - 1) as f32;

            for seg in 0..=segments {
                let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;
                let x = angle.cos() * self.trunk_radius;
                let z = angle.sin() * self.trunk_radius;

                mesh.vertices.push(Vector3::new(x, y, z));
                mesh.normals
                    .push(Vector3::new(angle.cos(), 0.0, angle.sin()));
                mesh.uvs.push(Vector2::new(seg as f32 / segments as f32, v));
            }
        }

        // Generate indices for cylinder sides (connect rings with triangles)
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

        // Add bottom cap
        let bottom_center_idx = mesh.vertices.len() as i32;
        mesh.vertices.push(Vector3::new(0.0, 0.0, 0.0));
        mesh.normals.push(Vector3::new(0.0, -1.0, 0.0));
        mesh.uvs.push(Vector2::new(0.5, 0.5));

        for seg in 0..=segments {
            let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;
            let x = angle.cos() * self.trunk_radius;
            let z = angle.sin() * self.trunk_radius;

            mesh.vertices.push(Vector3::new(x, 0.0, z));
            mesh.normals.push(Vector3::new(0.0, -1.0, 0.0));
            mesh.uvs.push(Vector2::new(
                0.5 + angle.cos() * 0.5,
                0.5 + angle.sin() * 0.5,
            ));
        }

        // Bottom cap triangles (clockwise for bottom-facing)
        let bottom_ring_start = bottom_center_idx + 1;
        for seg in 0..segments {
            let current = bottom_ring_start + seg as i32;
            let next = bottom_ring_start + (seg + 1) as i32;
            mesh.indices
                .extend_from_slice(&[bottom_center_idx, next, current]);
        }

        // Add top cap
        let top_center_idx = mesh.vertices.len() as i32;
        mesh.vertices
            .push(Vector3::new(0.0, self.trunk_height, 0.0));
        mesh.normals.push(Vector3::new(0.0, 1.0, 0.0));
        mesh.uvs.push(Vector2::new(0.5, 0.5));

        for seg in 0..=segments {
            let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;
            let x = angle.cos() * self.trunk_radius;
            let z = angle.sin() * self.trunk_radius;

            mesh.vertices.push(Vector3::new(x, self.trunk_height, z));
            mesh.normals.push(Vector3::new(0.0, 1.0, 0.0));
            mesh.uvs.push(Vector2::new(
                0.5 + angle.cos() * 0.5,
                0.5 + angle.sin() * 0.5,
            ));
        }

        // Top cap triangles (counter-clockwise for top-facing)
        let top_ring_start = top_center_idx + 1;
        for seg in 0..segments {
            let current = top_ring_start + seg as i32;
            let next = top_ring_start + (seg + 1) as i32;
            mesh.indices
                .extend_from_slice(&[top_center_idx, current, next]);
        }

        mesh
    }

    fn build_array_mesh(
        &self,
        vertices: Vec<Vector3>,
        normals: Vec<Vector3>,
        uvs: Vec<Vector2>,
        indices: Vec<i32>,
    ) -> Gd<ArrayMesh> {
        let mut mesh = ArrayMesh::new_gd();

        // Create the surface arrays
        let mut arrays = Array::<Variant>::new();
        arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());

        // Convert Rust vectors to Godot packed arrays
        let vertex_array = PackedVector3Array::from(vertices.as_slice());
        let normal_array = PackedVector3Array::from(normals.as_slice());
        let uv_array = PackedVector2Array::from(uvs.as_slice());
        let index_array = PackedInt32Array::from(indices.as_slice());

        arrays.set(ArrayType::VERTEX.ord() as usize, &vertex_array.to_variant());
        arrays.set(ArrayType::NORMAL.ord() as usize, &normal_array.to_variant());
        arrays.set(ArrayType::TEX_UV.ord() as usize, &uv_array.to_variant());
        arrays.set(ArrayType::INDEX.ord() as usize, &index_array.to_variant());

        mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);

        mesh
    }

    fn apply_mesh(&mut self, mesh: Gd<ArrayMesh>) {
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_mesh(&mesh);
        instance.set_name("TrunkMesh");

        self.base_mut().add_child(&instance);
        self.mesh_instance = Some(instance);
    }

    fn apply_foliage_mesh(&mut self, mesh: Gd<ArrayMesh>) {
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_mesh(&mesh);
        instance.set_name("FoliageMesh");

        self.base_mut().add_child(&instance);
        self.foliage_mesh_instance = Some(instance);
    }

    fn apply_foliage_preset_values(&mut self, values: &FoliagePresetValues) {
        self.foliage_enabled = values.enabled;
        self.leaf_style = values.leaf_style;
        self.foliage_placement = values.placement;
        self.leaf_orientation = values.orientation;
        self.foliage_density = values.density;
        self.cluster_size = values.cluster_size;
        self.leaf_size = values.leaf_size;
        self.leaf_size_variation = values.leaf_size_variation;
        self.foliage_radius_threshold = values.radius_threshold;
        self.foliage_height_falloff = values.height_falloff;
        self.leaf_droop = values.leaf_droop;
        self.leaf_rotation_variation = values.rotation_variation;
        self.use_crown_foliage_density = values.use_crown_density;
        self.separate_foliage_mesh = values.separate_mesh;
    }
}
