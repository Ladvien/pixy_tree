use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, Engine, MeshInstance3D, StandardMaterial3D};
use godot::prelude::*;

use crate::branch::{
    calculate_hole_segments, generate_branch_mesh_manifold, generate_branch_mesh_with_config,
    generate_branch_origins, generate_split_branches, generate_sub_branches, BranchAttachment,
    BranchConfig, BranchSegment, HoleBoundary, ManifoldMeshData, MeshData, SeededRng,
};
use crate::crown_shape::CrownShape;
use crate::foliage::{
    collect_leaf_points, generate_foliage_mesh, BranchInfo, FoliageConfig, FoliagePlacement,
    FoliagePresetValues, LeafOrientation, LeafStyle,
};
use crate::junction::generate_branch_collar;
use crate::smoothing;
use crate::tree_preset::{TreePreset, TreePresetValues};

/// How the trunk terminates at the top
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum TrunkTermination {
    #[default]
    FlatCap = 0,      // Current behavior - flat circular cap
    PointedTip = 1,   // Taper trunk to a point (no cap)
    LeaderBranch = 2, // Generate central leader extending up
}

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
    // Trunk Taper Settings
    // ═══════════════════════════════════════════
    /// Radius at top of trunk (0 = pointed, 1 = same as base)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.3)]
    trunk_taper: f32,

    /// Taper curve shape (0.5 = linear, <0.5 = aggressive early, >0.5 = gradual)
    #[export(range = (0.1, 2.0, 0.1))]
    #[init(val = 0.5)]
    trunk_taper_curve: f32,

    /// Flare at trunk base (1.0 = no flare, 1.5 = 50% wider base)
    #[export(range = (1.0, 2.0, 0.05))]
    #[init(val = 1.0)]
    trunk_flare: f32,

    /// Random trunk direction variation (0 = straight, 0.5 = very wobbly)
    #[export(range = (0.0, 0.5, 0.05))]
    #[init(val = 0.0)]
    trunk_randomness: f32,

    /// Number of visible root flares (0 = none, 3-8 typical)
    #[export(range = (0.0, 8.0, 1.0))]
    #[init(val = 0)]
    root_flare_count: i32,

    /// How far roots extend from trunk base
    #[export(range = (0.0, 2.0, 0.1))]
    #[init(val = 0.5)]
    root_flare_spread: f32,

    /// Root flare height on trunk (proportion of trunk height)
    #[export(range = (0.0, 0.5, 0.05))]
    #[init(val = 0.15)]
    root_flare_height: f32,

    // ═══════════════════════════════════════════
    // Trunk Termination Settings
    // ═══════════════════════════════════════════
    /// How the trunk terminates at the top
    #[export]
    #[init(val = TrunkTermination::FlatCap)]
    trunk_termination: TrunkTermination,

    /// Leader branch length as ratio of trunk height (0.1 = 10%)
    #[export(range = (0.05, 0.5, 0.05))]
    #[init(val = 0.15)]
    leader_length: f32,

    /// Leader taper (0 = pointed tip, 1 = same width as trunk top)
    #[export(range = (0.0, 0.5, 0.05))]
    #[init(val = 0.1)]
    leader_taper: f32,

    /// Enable sub-branches on leader
    #[export]
    #[init(val = false)]
    leader_has_branches: bool,

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

    /// Horizontal spread (0 = natural angle, 1 = flat horizontal canopy)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.0)]
    branch_flatness: f32,

    /// Angle variation with height (-1 = droop at top, +1 = reach upward at top)
    #[export(range = (-1.0, 1.0, 0.05))]
    #[init(val = 0.0)]
    branch_angle_curve: f32,

    /// Crown-based angle variation (-1 = lower horizontal, +1 = lower vertical)
    #[export(range = (-1.0, 1.0, 0.05))]
    #[init(val = 0.0)]
    crown_angle_variation: f32,

    /// Length multiplier per recursion level
    #[export(range = (0.3, 0.8, 0.05))]
    #[init(val = 0.5)]
    sub_branch_scale: f32,

    /// Random length variation (0 = uniform, 0.5 = high variation)
    #[export(range = (0.0, 0.5, 0.05))]
    #[init(val = 0.15)]
    branch_length_variation: f32,

    /// Sub-branch position bias (-1 = base, 0 = uniform, 1 = tip)
    #[export(range = (-1.0, 1.0, 0.1))]
    #[init(val = 0.0)]
    sub_branch_position_bias: f32,

    /// Leader dominance over side branches (0 = equal growth, 1 = strong leader)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.5)]
    apical_dominance: f32,

    // ═══════════════════════════════════════════
    // Twist Settings
    // ═══════════════════════════════════════════
    /// Total twist degrees over trunk height
    #[export(range = (0.0, 360.0, 1.0))]
    #[init(val = 0.0)]
    trunk_twist: f32,

    /// Twist degrees per branch length
    #[export(range = (0.0, 180.0, 1.0))]
    #[init(val = 0.0)]
    branch_twist: f32,

    // ═══════════════════════════════════════════
    // Gravity Settings
    // ═══════════════════════════════════════════
    /// Cumulative downward bend along branches
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.0)]
    gravity_strength: f32,

    /// Resistance to gravity bending (0 = flexible, 1 = stiff)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.5)]
    stiffness: f32,

    // ═══════════════════════════════════════════
    // Branch Randomness
    // ═══════════════════════════════════════════
    /// Chance for branch to terminate early (0 = none, 0.5 = half break)
    #[export(range = (0.0, 0.5, 0.05))]
    #[init(val = 0.0)]
    break_chance: f32,

    // ═══════════════════════════════════════════
    // Branch Splitting
    // ═══════════════════════════════════════════
    /// Enable mid-branch bifurcation
    #[export]
    #[init(val = false)]
    split_enabled: bool,

    /// Chance each branch splits
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.3)]
    split_probability: f32,

    /// Angle between split branches (degrees)
    #[export(range = (10.0, 60.0, 1.0))]
    #[init(val = 30.0)]
    split_angle: f32,

    /// Where along branch split occurs (0.0-1.0)
    #[export(range = (0.2, 0.8, 0.05))]
    #[init(val = 0.5)]
    split_position: f32,

    /// Minimum branch radius for splitting to occur
    #[export(range = (0.0, 0.5, 0.01))]
    #[init(val = 0.05)]
    split_radius_threshold: f32,

    // ═══════════════════════════════════════════
    // Branch Collar Settings
    // ═══════════════════════════════════════════
    /// Enable smooth collar mesh at branch connections
    #[export]
    #[init(val = true)]
    branch_collar_enabled: bool,

    /// Collar length as multiple of branch radius (1.0 = collar extends 1x branch radius)
    #[export(range = (0.5, 3.0, 0.1))]
    #[init(val = 1.5)]
    branch_collar_length: f32,

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
    // Material Settings
    // ═══════════════════════════════════════════
    /// Trunk and branch color
    #[export]
    #[init(val = Color::from_rgb(0.545, 0.271, 0.075))] // Saddle Brown
    trunk_color: Color,

    /// Foliage/leaf color
    #[export]
    #[init(val = Color::from_rgb(0.133, 0.545, 0.133))] // Forest Green
    foliage_color: Color,

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

    // ═══════════════════════════════════════════
    // Preview Controls (Debug)
    // ═══════════════════════════════════════════
    /// Limit primary branches for preview (0 = no limit)
    #[export(range = (0.0, 50.0, 1.0))]
    #[init(val = 0)]
    preview_branch_limit: i32,

    /// Override recursion depth for preview (-1 = use branch_recursion)
    #[export(range = (-1.0, 5.0, 1.0))]
    #[init(val = -1)]
    preview_recursion_depth: i32,

    // ═══════════════════════════════════════════
    // Manifold Mesh Settings
    // ═══════════════════════════════════════════
    /// Enable manifold mesh topology with improved branch collar generation.
    /// Branches use overlapping geometry for seamless visual connections.
    #[export]
    #[init(val = true)]
    manifold_topology: bool,

    /// Minimum segments per branch hole (prevents degenerate holes)
    #[export(range = (2.0, 6.0, 1.0))]
    #[init(val = 3)]
    min_hole_segments: i32,

    /// Junction normal blend sharpness (0 = smooth, 1 = sharp)
    #[export(range = (0.0, 1.0, 0.1))]
    #[init(val = 0.3)]
    junction_blend: f32,

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
            // Reset all settings to defaults before applying preset
            // This prevents settings from bleeding between presets
            self.reset_to_defaults();
            self.apply_preset_values(&values);
        }
    }

    /// Reset all generation-related properties to their default values
    /// Called before applying a preset to ensure clean state
    fn reset_to_defaults(&mut self) {
        // Trunk
        self.trunk_height = 5.0;
        self.trunk_radius = 0.5;
        self.radial_segments = 8;
        self.height_segments = 4;

        // Trunk Taper
        self.trunk_taper = 0.3;
        self.trunk_taper_curve = 0.5;
        self.trunk_flare = 1.0;
        self.trunk_randomness = 0.0;
        self.root_flare_count = 0;
        self.root_flare_spread = 0.5;
        self.root_flare_height = 0.15;

        // Trunk Termination
        self.trunk_termination = TrunkTermination::FlatCap;
        self.leader_length = 0.15;
        self.leader_taper = 0.1;
        self.leader_has_branches = false;

        // Branch
        self.branch_start = 0.3;
        self.branch_end = 0.9;
        self.branch_density = 1.0;
        self.branch_length = 0.4;
        self.branch_angle = 45.0;
        self.branch_radius_ratio = 0.3;
        self.branch_taper = 0.7;
        self.phyllotaxis_angle = 137.5;
        self.branch_randomness = 0.2;
        self.up_attraction = 0.1;
        self.branch_recursion = 1;
        self.sub_branch_count = 2;
        self.sub_branch_scale = 0.5;
        self.branch_length_variation = 0.15;
        self.sub_branch_position_bias = 0.0;
        self.apical_dominance = 0.5;
        self.branch_flatness = 0.0;
        self.branch_angle_curve = 0.0;
        self.crown_angle_variation = 0.0;

        // Twist
        self.trunk_twist = 0.0;
        self.branch_twist = 0.0;

        // Gravity
        self.gravity_strength = 0.0;
        self.stiffness = 0.5;

        // Branch Randomness
        self.break_chance = 0.0;

        // Splitting
        self.split_enabled = false;
        self.split_probability = 0.3;
        self.split_angle = 30.0;
        self.split_position = 0.5;
        self.split_radius_threshold = 0.05;

        // Branch Collar
        self.branch_collar_enabled = true;
        self.branch_collar_length = 1.5;

        // Manifold
        self.manifold_topology = false;
        self.min_hole_segments = 3;
        self.junction_blend = 0.3;

        // Crown
        self.crown_shape = CrownShape::Cylindrical;
        self.crown_influence = 1.0;

        // Foliage
        self.foliage_enabled = true;
        self.leaf_style = LeafStyle::CrossedPlanes;
        self.foliage_placement = FoliagePlacement::TerminalBranches;
        self.leaf_orientation = LeafOrientation::RadialOutward;
        self.foliage_density = 3.0;
        self.cluster_size = 4;
        self.leaf_size = 0.3;
        self.leaf_size_variation = 0.15;
        self.foliage_radius_threshold = 0.15;
        self.foliage_height_falloff = 0.3;
        self.leaf_droop = 0.2;
        self.leaf_rotation_variation = 0.5;
        self.use_crown_foliage_density = true;
        self.separate_foliage_mesh = true;

        // Materials
        self.trunk_color = Color::from_rgb(0.545, 0.271, 0.075); // Saddle Brown
        self.foliage_color = Color::from_rgb(0.133, 0.545, 0.133); // Forest Green
    }

    fn apply_preset_values(&mut self, values: &TreePresetValues) {
        // Trunk
        self.trunk_height = values.trunk_height;
        self.trunk_radius = values.trunk_radius;
        self.radial_segments = values.radial_segments;
        self.height_segments = values.height_segments;

        // Trunk Taper
        self.trunk_taper = values.trunk_taper;
        self.trunk_taper_curve = values.trunk_taper_curve;
        self.trunk_flare = values.trunk_flare;
        self.trunk_randomness = values.trunk_randomness;
        self.root_flare_count = values.root_flare_count;
        self.root_flare_spread = values.root_flare_spread;
        self.root_flare_height = values.root_flare_height;

        // Trunk Termination
        self.trunk_termination = values.trunk_termination;
        self.leader_length = values.leader_length;
        self.leader_taper = values.leader_taper;
        self.leader_has_branches = values.leader_has_branches;

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
        self.branch_length_variation = values.branch_length_variation;
        self.sub_branch_position_bias = values.sub_branch_position_bias;
        self.apical_dominance = values.apical_dominance;
        self.branch_flatness = values.branch_flatness;
        self.branch_angle_curve = values.branch_angle_curve;
        self.crown_angle_variation = values.crown_angle_variation;

        // Twist
        self.trunk_twist = values.trunk_twist;
        self.branch_twist = values.branch_twist;

        // Gravity
        self.gravity_strength = values.gravity_strength;
        self.stiffness = values.stiffness;

        // Branch Randomness
        self.break_chance = values.break_chance;

        // Splitting
        self.split_enabled = values.split_enabled;
        self.split_probability = values.split_probability;
        self.split_angle = values.split_angle;
        self.split_position = values.split_position;
        self.split_radius_threshold = values.split_radius_threshold;

        // Branch Collar
        self.branch_collar_enabled = values.branch_collar_enabled;
        self.branch_collar_length = values.branch_collar_length;

        // Crown
        self.crown_shape = values.crown_shape;
        self.crown_influence = values.crown_influence;

        // Materials
        self.trunk_color = values.trunk_color;
        self.foliage_color = values.foliage_color;

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
        hash = hash.wrapping_add((self.branch_flatness.to_bits() as u64).wrapping_mul(313));
        hash = hash.wrapping_add((self.branch_angle_curve.to_bits() as u64).wrapping_mul(317));
        hash = hash.wrapping_add((self.crown_angle_variation.to_bits() as u64).wrapping_mul(373));
        hash = hash.wrapping_add((self.branch_length_variation.to_bits() as u64).wrapping_mul(337));
        hash =
            hash.wrapping_add((self.sub_branch_position_bias.to_bits() as u64).wrapping_mul(347));
        hash = hash.wrapping_add((self.apical_dominance.to_bits() as u64).wrapping_mul(349));
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
        // Material colors
        hash = hash.wrapping_add((self.trunk_color.r.to_bits() as u64).wrapping_mul(199));
        hash = hash.wrapping_add((self.trunk_color.g.to_bits() as u64).wrapping_mul(211));
        hash = hash.wrapping_add((self.trunk_color.b.to_bits() as u64).wrapping_mul(223));
        hash = hash.wrapping_add((self.foliage_color.r.to_bits() as u64).wrapping_mul(227));
        hash = hash.wrapping_add((self.foliage_color.g.to_bits() as u64).wrapping_mul(229));
        hash = hash.wrapping_add((self.foliage_color.b.to_bits() as u64).wrapping_mul(233));
        // Twist, gravity, and splitting parameters
        hash = hash.wrapping_add((self.trunk_twist.to_bits() as u64).wrapping_mul(239));
        hash = hash.wrapping_add((self.branch_twist.to_bits() as u64).wrapping_mul(241));
        hash = hash.wrapping_add((self.gravity_strength.to_bits() as u64).wrapping_mul(251));
        hash = hash.wrapping_add((self.split_enabled as u64).wrapping_mul(257));
        hash = hash.wrapping_add((self.split_probability.to_bits() as u64).wrapping_mul(263));
        hash = hash.wrapping_add((self.split_angle.to_bits() as u64).wrapping_mul(269));
        hash = hash.wrapping_add((self.split_position.to_bits() as u64).wrapping_mul(271));
        hash = hash.wrapping_add((self.split_radius_threshold.to_bits() as u64).wrapping_mul(311));
        // Trunk taper, stiffness, and break_chance parameters
        hash = hash.wrapping_add((self.trunk_taper.to_bits() as u64).wrapping_mul(277));
        hash = hash.wrapping_add((self.trunk_taper_curve.to_bits() as u64).wrapping_mul(281));
        hash = hash.wrapping_add((self.trunk_flare.to_bits() as u64).wrapping_mul(283));
        hash = hash.wrapping_add((self.trunk_randomness.to_bits() as u64).wrapping_mul(331));
        hash = hash.wrapping_add((self.root_flare_count as u64).wrapping_mul(353));
        hash = hash.wrapping_add((self.root_flare_spread.to_bits() as u64).wrapping_mul(359));
        hash = hash.wrapping_add((self.root_flare_height.to_bits() as u64).wrapping_mul(367));
        hash = hash.wrapping_add((self.break_chance.to_bits() as u64).wrapping_mul(293));
        hash = hash.wrapping_add((self.stiffness.to_bits() as u64).wrapping_mul(307));
        // Trunk termination parameters
        hash = hash.wrapping_add((self.trunk_termination as u64).wrapping_mul(389));
        hash = hash.wrapping_add((self.leader_length.to_bits() as u64).wrapping_mul(397));
        hash = hash.wrapping_add((self.leader_taper.to_bits() as u64).wrapping_mul(401));
        hash = hash.wrapping_add((self.leader_has_branches as u64).wrapping_mul(409));
        // Branch collar parameters
        hash = hash.wrapping_add((self.branch_collar_enabled as u64).wrapping_mul(419));
        hash = hash.wrapping_add((self.branch_collar_length.to_bits() as u64).wrapping_mul(421));
        // Preview controls
        hash = hash.wrapping_add((self.preview_branch_limit as u64).wrapping_mul(379));
        hash = hash.wrapping_add((self.preview_recursion_depth as u64).wrapping_mul(383));
        // Manifold settings
        hash = hash.wrapping_add((self.manifold_topology as u64).wrapping_mul(431));
        hash = hash.wrapping_add((self.min_hole_segments as u64).wrapping_mul(433));
        hash = hash.wrapping_add((self.junction_blend.to_bits() as u64).wrapping_mul(439));
        hash
    }

    #[func]
    pub fn generate(&mut self) {
        // Use manifold topology if enabled
        if self.manifold_topology {
            self.generate_manifold();
            return;
        }

        self.clear();

        // 1. Generate trunk mesh data
        let mut mesh_data = self.create_trunk_mesh_data();

        // 2. Generate branches
        let mut config = self.create_branch_config();
        let mut rng = SeededRng::new(self.seed);

        // Apply preview recursion depth override
        if self.preview_recursion_depth >= 0 {
            config.branch_recursion = self.preview_recursion_depth;
        }

        let mut primary_branches = generate_branch_origins(&config, &mut rng);

        // Apply preview branch limit
        if self.preview_branch_limit > 0 {
            primary_branches.truncate(self.preview_branch_limit as usize);
        }

        let branch_segments = (self.radial_segments / 2).max(4);

        // Collect all branches for foliage generation
        let mut all_branches: Vec<BranchSegment> = Vec::new();

        // Generate leader branch if enabled
        if self.trunk_termination == TrunkTermination::LeaderBranch {
            let leader = self.create_leader_branch();
            let leader_mesh = generate_branch_mesh_with_config(
                &leader,
                branch_segments,
                self.branch_twist,
                self.gravity_strength,
                self.stiffness,
            );
            mesh_data.extend(&leader_mesh);
            all_branches.push(leader.clone());

            // Generate sub-branches on leader if enabled
            if self.leader_has_branches {
                let sub_branches = generate_sub_branches(&leader, &config, &mut rng, 0);
                for sub in &sub_branches {
                    let sub_mesh = generate_branch_mesh_with_config(
                        sub,
                        4,
                        self.branch_twist,
                        self.gravity_strength,
                        self.stiffness,
                    );
                    mesh_data.extend(&sub_mesh);
                    all_branches.push(sub.clone());
                }
            }
        }

        for branch in &primary_branches {
            // Generate branch collar for smooth trunk-branch junction
            if self.branch_collar_enabled {
                // Calculate trunk radius at branch height
                let t = branch.start.y / self.trunk_height;
                let taper_factor = t.powf(self.trunk_taper_curve);
                let base_r = self.trunk_radius * self.trunk_flare;
                let tip_r = self.trunk_radius * self.trunk_taper;
                let trunk_r_at_height = base_r + (tip_r - base_r) * taper_factor;

                let collar = generate_branch_collar(
                    branch.start,
                    branch.direction,
                    trunk_r_at_height,
                    branch.base_radius,
                    branch.base_radius * self.branch_collar_length,
                    self.radial_segments / 2,
                );
                mesh_data.extend(&collar);
            }

            // Check for branch splitting
            if let Some((stub, split1, split2)) = generate_split_branches(branch, &config, &mut rng)
            {
                // Generate stub mesh (from start to split point)
                let stub_mesh = generate_branch_mesh_with_config(
                    &stub,
                    branch_segments,
                    self.branch_twist,
                    self.gravity_strength,
                    self.stiffness,
                );
                mesh_data.extend(&stub_mesh);

                // Generate both split branches
                for split_branch in [&split1, &split2] {
                    let split_mesh = generate_branch_mesh_with_config(
                        split_branch,
                        branch_segments,
                        self.branch_twist,
                        self.gravity_strength,
                        self.stiffness,
                    );
                    mesh_data.extend(&split_mesh);
                    all_branches.push(split_branch.clone());

                    // Sub-branches from split branches
                    let sub_branches = generate_sub_branches(split_branch, &config, &mut rng, 0);
                    for sub in &sub_branches {
                        let sub_mesh = generate_branch_mesh_with_config(
                            sub,
                            4,
                            self.branch_twist,
                            self.gravity_strength,
                            self.stiffness,
                        );
                        mesh_data.extend(&sub_mesh);
                        all_branches.push(sub.clone());
                    }
                }
            } else {
                // No split - generate normal branch
                let branch_mesh = generate_branch_mesh_with_config(
                    branch,
                    branch_segments,
                    self.branch_twist,
                    self.gravity_strength,
                    self.stiffness,
                );
                mesh_data.extend(&branch_mesh);

                all_branches.push(branch.clone());

                // Add sub-branches recursively
                let sub_branches = generate_sub_branches(branch, &config, &mut rng, 0);
                for sub in &sub_branches {
                    // Sub-branches also get twist and gravity
                    let sub_mesh = generate_branch_mesh_with_config(
                        sub,
                        4, // fewer segments for sub-branches
                        self.branch_twist,
                        self.gravity_strength,
                        self.stiffness,
                    );
                    mesh_data.extend(&sub_mesh);
                    all_branches.push(sub.clone());
                }
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
            branch_length_variation: self.branch_length_variation,
            sub_branch_position_bias: self.sub_branch_position_bias,
            apical_dominance: self.apical_dominance,
            branch_flatness: self.branch_flatness,
            branch_angle_curve: self.branch_angle_curve,
            crown_angle_variation: self.crown_angle_variation,
            radial_segments: self.radial_segments,
            crown_shape: self.crown_shape,
            crown_influence: self.crown_influence,
            trunk_taper: self.trunk_taper,
            trunk_taper_curve: self.trunk_taper_curve,
            trunk_flare: self.trunk_flare,
            trunk_randomness: self.trunk_randomness,
            seed: self.seed,
            trunk_twist: self.trunk_twist,
            branch_twist: self.branch_twist,
            gravity_strength: self.gravity_strength,
            stiffness: self.stiffness,
            break_chance: self.break_chance,
            split_enabled: self.split_enabled,
            split_probability: self.split_probability,
            split_angle: self.split_angle,
            split_position: self.split_position,
            split_radius_threshold: self.split_radius_threshold,
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

    /// Create a leader branch that extends upward from the trunk top
    fn create_leader_branch(&self) -> BranchSegment {
        // Match trunk top radius accounting for taper
        let trunk_top_radius = self.trunk_radius * self.trunk_taper;

        // Calculate trunk top position (with wobble if trunk_randomness > 0)
        let seed_f = self.seed as f32;
        let top_wobble_x = if self.trunk_randomness > 0.0 {
            (std::f32::consts::PI + seed_f * 0.1).sin()
                * self.trunk_randomness
                * self.trunk_height
                * 0.3
        } else {
            0.0
        };
        let top_wobble_z = if self.trunk_randomness > 0.0 {
            (std::f32::consts::E + seed_f * 0.2).cos()
                * self.trunk_randomness
                * self.trunk_height
                * 0.3
        } else {
            0.0
        };

        BranchSegment {
            start: Vector3::new(top_wobble_x, self.trunk_height, top_wobble_z),
            direction: Vector3::UP,
            length: self.trunk_height * self.leader_length,
            base_radius: trunk_top_radius,
            tip_radius: trunk_top_radius * self.leader_taper,
            depth: 0,
            is_terminal: !self.leader_has_branches,
            height_ratio: 1.0,
        }
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

        // Calculate base and tip radii with taper and flare
        let base_radius = self.trunk_radius * self.trunk_flare;
        let tip_radius = self.trunk_radius * self.trunk_taper;

        // Helper for lerp
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;

        // Seed-based phase offsets for deterministic wobble
        let seed_f = self.seed as f32;
        let phase_x = seed_f * 0.1;
        let phase_z = seed_f * 0.2;

        // Root flare settings
        let root_height = self.root_flare_height * self.trunk_height;
        let has_root_flares = self.root_flare_count > 0 && self.root_flare_spread > 0.0;

        // Generate rings of vertices for the cylinder sides
        for ring in 0..rings {
            let t = ring as f32 / (rings - 1) as f32;
            let y = t * self.trunk_height;
            let v = t;

            // Calculate radius at this height with taper curve
            // power < 1 = aggressive early taper, power > 1 = gradual taper
            let taper_factor = t.powf(self.trunk_taper_curve);
            let radius = lerp(base_radius, tip_radius, taper_factor);

            // Apply trunk randomness (cumulative wobble with height)
            // Wobble increases toward top (multiplied by t) for natural lean
            let wobble_x = if self.trunk_randomness > 0.0 {
                (t * std::f32::consts::PI + phase_x).sin()
                    * self.trunk_randomness
                    * t
                    * self.trunk_height
                    * 0.3
            } else {
                0.0
            };
            let wobble_z = if self.trunk_randomness > 0.0 {
                (t * std::f32::consts::E + phase_z).cos()
                    * self.trunk_randomness
                    * t
                    * self.trunk_height
                    * 0.3
            } else {
                0.0
            };

            // Apply trunk twist rotation based on height
            let twist_angle = t * self.trunk_twist.to_radians();
            let cos_t = twist_angle.cos();
            let sin_t = twist_angle.sin();

            // Calculate root flare blend factor (1.0 at base, 0.0 at root_height and above)
            let root_blend = if has_root_flares && y < root_height {
                1.0 - (y / root_height)
            } else {
                0.0
            };

            for seg in 0..=segments {
                let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;

                // Apply root flare: sinusoidal radial bulge based on angle
                let root_bulge = if root_blend > 0.0 {
                    // Use root_flare_count to create that many bulges around the circumference
                    // Each root creates a bulge at evenly spaced angles
                    let bulge_angle = angle * self.root_flare_count as f32;
                    // Use squared cosine for sharper, more defined root ridges
                    let raw_bulge = (bulge_angle.cos() * 0.5 + 0.5).powi(2);
                    raw_bulge * self.root_flare_spread * base_radius * root_blend
                } else {
                    0.0
                };

                let effective_radius = radius + root_bulge;
                let base_x = angle.cos() * effective_radius;
                let base_z = angle.sin() * effective_radius;

                // Apply twist rotation around Y axis
                let twisted_x = base_x * cos_t - base_z * sin_t;
                let twisted_z = base_x * sin_t + base_z * cos_t;

                // Add wobble offset
                let x = twisted_x + wobble_x;
                let z = twisted_z + wobble_z;

                mesh.vertices.push(Vector3::new(x, y, z));

                // Rotate normal as well
                let base_nx = angle.cos();
                let base_nz = angle.sin();
                let nx = base_nx * cos_t - base_nz * sin_t;
                let nz = base_nx * sin_t + base_nz * cos_t;
                mesh.normals.push(Vector3::new(nx, 0.0, nz));
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

        // Add bottom cap (use base_radius with flare and root flares)
        let bottom_center_idx = mesh.vertices.len() as i32;
        mesh.vertices.push(Vector3::new(0.0, 0.0, 0.0));
        mesh.normals.push(Vector3::new(0.0, -1.0, 0.0));
        mesh.uvs.push(Vector2::new(0.5, 0.5));

        for seg in 0..=segments {
            let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;

            // Apply root flare to bottom cap (full strength at y=0)
            let root_bulge = if has_root_flares {
                let bulge_angle = angle * self.root_flare_count as f32;
                let raw_bulge = (bulge_angle.cos() * 0.5 + 0.5).powi(2);
                raw_bulge * self.root_flare_spread * base_radius
            } else {
                0.0
            };

            let cap_radius = base_radius + root_bulge;
            let x = angle.cos() * cap_radius;
            let z = angle.sin() * cap_radius;

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

        // Add top cap only for FlatCap mode and if trunk_taper is large enough
        // PointedTip: trunk tapers to point (no cap needed)
        // LeaderBranch: leader branch covers the top (no cap needed)
        let should_add_top_cap = self.trunk_termination == TrunkTermination::FlatCap
            && self.trunk_taper >= 0.1;

        if should_add_top_cap {
            let top_twist_angle = self.trunk_twist.to_radians();
            let top_cos_t = top_twist_angle.cos();
            let top_sin_t = top_twist_angle.sin();

            // Calculate top wobble (t=1.0)
            let top_wobble_x = if self.trunk_randomness > 0.0 {
                (std::f32::consts::PI + seed_f * 0.1).sin()
                    * self.trunk_randomness
                    * self.trunk_height
                    * 0.3
            } else {
                0.0
            };
            let top_wobble_z = if self.trunk_randomness > 0.0 {
                (std::f32::consts::E + seed_f * 0.2).cos()
                    * self.trunk_randomness
                    * self.trunk_height
                    * 0.3
            } else {
                0.0
            };

            let top_center_idx = mesh.vertices.len() as i32;
            mesh.vertices
                .push(Vector3::new(top_wobble_x, self.trunk_height, top_wobble_z));
            mesh.normals.push(Vector3::new(0.0, 1.0, 0.0));
            mesh.uvs.push(Vector2::new(0.5, 0.5));

            for seg in 0..=segments {
                let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;
                let base_x = angle.cos() * tip_radius;
                let base_z = angle.sin() * tip_radius;

                // Apply twist rotation for top cap
                let twisted_x = base_x * top_cos_t - base_z * top_sin_t;
                let twisted_z = base_x * top_sin_t + base_z * top_cos_t;

                // Add wobble offset
                let x = twisted_x + top_wobble_x;
                let z = twisted_z + top_wobble_z;

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
        }

        mesh
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Manifold Mesh Generation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Collect branch attachment information for manifold mesh generation
    fn collect_branch_attachments(&self, branches: &[BranchSegment]) -> Vec<BranchAttachment> {
        let mut attachments = Vec::new();

        for branch in branches {
            // Calculate height ratio
            let height_ratio = branch.start.y / self.trunk_height;

            // Calculate trunk radius at this height
            let t = height_ratio;
            let taper_factor = t.powf(self.trunk_taper_curve);
            let base_r = self.trunk_radius * self.trunk_flare;
            let tip_r = self.trunk_radius * self.trunk_taper;
            let trunk_r = base_r + (tip_r - base_r) * taper_factor;

            // Calculate angle around trunk from attachment point
            // Account for wobble offset
            let seed_f = self.seed as f32;
            let wobble_x = if self.trunk_randomness > 0.0 {
                (t * std::f32::consts::PI + seed_f * 0.1).sin()
                    * self.trunk_randomness
                    * t
                    * self.trunk_height
                    * 0.3
            } else {
                0.0
            };
            let wobble_z = if self.trunk_randomness > 0.0 {
                (t * std::f32::consts::E + seed_f * 0.2).cos()
                    * self.trunk_randomness
                    * t
                    * self.trunk_height
                    * 0.3
            } else {
                0.0
            };

            // Remove wobble to get angle
            let adjusted_x = branch.start.x - wobble_x;
            let adjusted_z = branch.start.z - wobble_z;
            let angle = adjusted_z.atan2(adjusted_x);

            // Calculate hole segments
            let (hole_start, hole_end) = calculate_hole_segments(
                angle,
                branch.base_radius,
                trunk_r,
                self.radial_segments,
            );

            // Find trunk ring index for this height
            let rings = self.height_segments.max(1) as usize + 1;
            let trunk_ring_index = (height_ratio * (rings - 1) as f32).round() as usize;

            attachments.push(BranchAttachment {
                height_ratio,
                angle,
                branch_radius: branch.base_radius,
                direction: branch.direction,
                attachment_point: branch.start,
                trunk_radius_at_height: trunk_r,
                trunk_ring_index,
                hole_segment_start: hole_start,
                hole_segment_end: hole_end,
            });
        }

        attachments
    }

    /// Create trunk mesh with holes for manifold branch connections
    /// Returns the mesh data and hole boundaries for each hole
    fn create_trunk_mesh_with_holes(
        &self,
        attachments: &[BranchAttachment],
    ) -> (ManifoldMeshData, Vec<HoleBoundary>) {
        let mut mesh = ManifoldMeshData::new();
        let mut hole_boundaries: Vec<HoleBoundary> = Vec::new();

        let segments = self.radial_segments.max(3) as usize;
        let rings = self.height_segments.max(1) as usize + 1;

        // Calculate base and tip radii with taper and flare
        let base_radius = self.trunk_radius * self.trunk_flare;
        let tip_radius = self.trunk_radius * self.trunk_taper;

        // Seed-based phase offsets for deterministic wobble
        let seed_f = self.seed as f32;
        let phase_x = seed_f * 0.1;
        let phase_z = seed_f * 0.2;

        // Root flare settings
        let root_height = self.root_flare_height * self.trunk_height;
        let has_root_flares = self.root_flare_count > 0 && self.root_flare_spread > 0.0;

        // Build a set of (ring, segment) pairs that are holes
        // IMPORTANT: Mark holes on BOTH ring_idx AND ring_idx+1 since quads span two rings
        let mut hole_segments: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();

        for attachment in attachments {
            let ring_idx = attachment.trunk_ring_index;
            let start = attachment.hole_segment_start;
            let end = attachment.hole_segment_end;

            // Mark segments as holes on both rings that the quad spans
            let ring_indices = [ring_idx, (ring_idx + 1).min(rings - 1)];

            for &ring in &ring_indices {
                // Mark segments as holes (handle wraparound)
                if start <= end {
                    for seg in start..=end {
                        hole_segments.insert((ring, seg));
                    }
                } else {
                    // Wraparound case
                    for seg in start..segments {
                        hole_segments.insert((ring, seg));
                    }
                    for seg in 0..=end {
                        hole_segments.insert((ring, seg));
                    }
                }
            }
        }

        // Track ring vertex indices for connecting rings and creating hole boundaries
        let mut ring_vertex_indices: Vec<Vec<u32>> = Vec::with_capacity(rings);

        // Helper for lerp
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;

        // Generate rings of vertices for the cylinder sides
        for ring in 0..rings {
            let t = ring as f32 / (rings - 1) as f32;
            let y = t * self.trunk_height;
            let v = t;

            // Calculate radius at this height with taper curve
            let taper_factor = t.powf(self.trunk_taper_curve);
            let radius = lerp(base_radius, tip_radius, taper_factor);

            // Apply trunk randomness (cumulative wobble with height)
            let wobble_x = if self.trunk_randomness > 0.0 {
                (t * std::f32::consts::PI + phase_x).sin()
                    * self.trunk_randomness
                    * t
                    * self.trunk_height
                    * 0.3
            } else {
                0.0
            };
            let wobble_z = if self.trunk_randomness > 0.0 {
                (t * std::f32::consts::E + phase_z).cos()
                    * self.trunk_randomness
                    * t
                    * self.trunk_height
                    * 0.3
            } else {
                0.0
            };

            // Apply trunk twist rotation based on height
            let twist_angle = t * self.trunk_twist.to_radians();
            let cos_t = twist_angle.cos();
            let sin_t = twist_angle.sin();

            // Calculate root flare blend factor
            let root_blend = if has_root_flares && y < root_height {
                1.0 - (y / root_height)
            } else {
                0.0
            };

            let mut current_ring_indices: Vec<u32> = Vec::with_capacity(segments);

            for seg in 0..segments {
                let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;

                // Apply root flare
                let root_bulge = if root_blend > 0.0 {
                    let bulge_angle = angle * self.root_flare_count as f32;
                    let raw_bulge = (bulge_angle.cos() * 0.5 + 0.5).powi(2);
                    raw_bulge * self.root_flare_spread * base_radius * root_blend
                } else {
                    0.0
                };

                let effective_radius = radius + root_bulge;
                let base_x = angle.cos() * effective_radius;
                let base_z = angle.sin() * effective_radius;

                // Apply twist rotation around Y axis
                let twisted_x = base_x * cos_t - base_z * sin_t;
                let twisted_z = base_x * sin_t + base_z * cos_t;

                // Add wobble offset
                let x = twisted_x + wobble_x;
                let z = twisted_z + wobble_z;

                // Rotate normal as well
                let base_nx = angle.cos();
                let base_nz = angle.sin();
                let nx = base_nx * cos_t - base_nz * sin_t;
                let nz = base_nx * sin_t + base_nz * cos_t;

                let idx = mesh.pool.add_vertex(
                    Vector3::new(x, y, z),
                    Vector3::new(nx, 0.0, nz),
                    Vector2::new(seg as f32 / segments as f32, v),
                );
                current_ring_indices.push(idx);
            }

            ring_vertex_indices.push(current_ring_indices);
        }

        // Generate indices for cylinder sides, skipping quads inside holes
        for ring in 0..(rings - 1) {
            for seg in 0..segments {
                let next_seg = (seg + 1) % segments;

                // Check if this quad falls within a hole region
                // A quad spans (ring, seg) to (ring+1, next_seg)
                // Skip if the segment is marked as a hole on BOTH rings it spans
                let is_hole_lower = hole_segments.contains(&(ring, seg));
                let is_hole_upper = hole_segments.contains(&(ring + 1, seg));

                if is_hole_lower && is_hole_upper {
                    continue; // Skip this quad - it's inside a hole
                }

                let current = ring_vertex_indices[ring][seg];
                let next = ring_vertex_indices[ring][next_seg];
                let above = ring_vertex_indices[ring + 1][seg];
                let above_next = ring_vertex_indices[ring + 1][next_seg];

                mesh.add_quad(current, next, above, above_next);
            }
        }

        // Create hole boundaries for each branch (collecting from both rings)
        for attachment in attachments {
            let ring_idx = attachment.trunk_ring_index;
            let upper_ring_idx = (ring_idx + 1).min(rings - 1);
            let start = attachment.hole_segment_start;
            let end = attachment.hole_segment_end;

            let mut lower_indices: Vec<u32> = Vec::new();
            let mut upper_indices: Vec<u32> = Vec::new();
            let mut lower_positions: Vec<Vector3> = Vec::new();
            let mut upper_positions: Vec<Vector3> = Vec::new();

            // Collect hole boundary vertices from both rings in segment order
            // Lower ring (ring_idx) and Upper ring (ring_idx + 1)
            // Note: Each quad at segment N uses vertices N and N+1, so we need to include
            // the closing vertex at (end + 1) % segments to fully cover the hole boundary.
            if start <= end {
                for seg in start..=end {
                    if seg < ring_vertex_indices[ring_idx].len() {
                        let idx = ring_vertex_indices[ring_idx][seg];
                        lower_indices.push(idx);
                        lower_positions.push(mesh.pool.get_position(idx));
                    }
                    if seg < ring_vertex_indices[upper_ring_idx].len() {
                        let idx = ring_vertex_indices[upper_ring_idx][seg];
                        upper_indices.push(idx);
                        upper_positions.push(mesh.pool.get_position(idx));
                    }
                }
                // Add closing vertex at (end + 1) % segments
                let closing_seg = (end + 1) % segments;
                if closing_seg < ring_vertex_indices[ring_idx].len() {
                    let idx = ring_vertex_indices[ring_idx][closing_seg];
                    lower_indices.push(idx);
                    lower_positions.push(mesh.pool.get_position(idx));
                }
                if closing_seg < ring_vertex_indices[upper_ring_idx].len() {
                    let idx = ring_vertex_indices[upper_ring_idx][closing_seg];
                    upper_indices.push(idx);
                    upper_positions.push(mesh.pool.get_position(idx));
                }
            } else {
                // Wraparound case: collect from start to end of ring, then 0 to end
                for seg in start..segments {
                    if seg < ring_vertex_indices[ring_idx].len() {
                        let idx = ring_vertex_indices[ring_idx][seg];
                        lower_indices.push(idx);
                        lower_positions.push(mesh.pool.get_position(idx));
                    }
                    if seg < ring_vertex_indices[upper_ring_idx].len() {
                        let idx = ring_vertex_indices[upper_ring_idx][seg];
                        upper_indices.push(idx);
                        upper_positions.push(mesh.pool.get_position(idx));
                    }
                }
                for seg in 0..=end {
                    if seg < ring_vertex_indices[ring_idx].len() {
                        let idx = ring_vertex_indices[ring_idx][seg];
                        lower_indices.push(idx);
                        lower_positions.push(mesh.pool.get_position(idx));
                    }
                    if seg < ring_vertex_indices[upper_ring_idx].len() {
                        let idx = ring_vertex_indices[upper_ring_idx][seg];
                        upper_indices.push(idx);
                        upper_positions.push(mesh.pool.get_position(idx));
                    }
                }
                // Add closing vertex at (end + 1) % segments
                let closing_seg = (end + 1) % segments;
                if closing_seg < ring_vertex_indices[ring_idx].len() {
                    let idx = ring_vertex_indices[ring_idx][closing_seg];
                    lower_indices.push(idx);
                    lower_positions.push(mesh.pool.get_position(idx));
                }
                if closing_seg < ring_vertex_indices[upper_ring_idx].len() {
                    let idx = ring_vertex_indices[upper_ring_idx][closing_seg];
                    upper_indices.push(idx);
                    upper_positions.push(mesh.pool.get_position(idx));
                }
            }

            // Calculate arc extent and starting angle for the hole
            let segment_angle = std::f32::consts::TAU / segments as f32;
            let arc_segments = lower_indices.len();
            let arc_extent = arc_segments as f32 * segment_angle;
            let arc_start_angle = start as f32 * segment_angle;

            hole_boundaries.push(HoleBoundary::new(
                lower_indices,
                upper_indices,
                lower_positions,
                upper_positions,
                attachment.attachment_point,
                attachment.direction,
                attachment.branch_radius,
                attachment.trunk_radius_at_height,
                arc_extent,
                arc_start_angle,
            ));
        }

        // Add bottom cap
        let bottom_center_idx =
            mesh.pool
                .add_unique_vertex(Vector3::ZERO, Vector3::new(0.0, -1.0, 0.0), Vector2::new(0.5, 0.5));

        let mut bottom_ring_indices: Vec<u32> = Vec::new();
        for seg in 0..segments {
            let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;

            let root_bulge = if has_root_flares {
                let bulge_angle = angle * self.root_flare_count as f32;
                let raw_bulge = (bulge_angle.cos() * 0.5 + 0.5).powi(2);
                raw_bulge * self.root_flare_spread * base_radius
            } else {
                0.0
            };

            let cap_radius = base_radius + root_bulge;
            let x = angle.cos() * cap_radius;
            let z = angle.sin() * cap_radius;

            let idx = mesh.pool.add_vertex(
                Vector3::new(x, 0.0, z),
                Vector3::new(0.0, -1.0, 0.0),
                Vector2::new(0.5 + angle.cos() * 0.5, 0.5 + angle.sin() * 0.5),
            );
            bottom_ring_indices.push(idx);
        }

        // Bottom cap triangles (clockwise for bottom-facing)
        for seg in 0..segments {
            let current = bottom_ring_indices[seg];
            let next = bottom_ring_indices[(seg + 1) % segments];
            mesh.add_triangle(bottom_center_idx, next, current);
        }

        // Add top cap only for FlatCap mode and if trunk_taper is large enough
        let should_add_top_cap =
            self.trunk_termination == TrunkTermination::FlatCap && self.trunk_taper >= 0.1;

        if should_add_top_cap {
            let top_twist_angle = self.trunk_twist.to_radians();
            let top_cos_t = top_twist_angle.cos();
            let top_sin_t = top_twist_angle.sin();

            // Calculate top wobble (t=1.0)
            let top_wobble_x = if self.trunk_randomness > 0.0 {
                (std::f32::consts::PI + seed_f * 0.1).sin()
                    * self.trunk_randomness
                    * self.trunk_height
                    * 0.3
            } else {
                0.0
            };
            let top_wobble_z = if self.trunk_randomness > 0.0 {
                (std::f32::consts::E + seed_f * 0.2).cos()
                    * self.trunk_randomness
                    * self.trunk_height
                    * 0.3
            } else {
                0.0
            };

            let top_center_idx = mesh.pool.add_unique_vertex(
                Vector3::new(top_wobble_x, self.trunk_height, top_wobble_z),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(0.5, 0.5),
            );

            let mut top_ring_indices: Vec<u32> = Vec::new();
            for seg in 0..segments {
                let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;
                let base_x = angle.cos() * tip_radius;
                let base_z = angle.sin() * tip_radius;

                let twisted_x = base_x * top_cos_t - base_z * top_sin_t;
                let twisted_z = base_x * top_sin_t + base_z * top_cos_t;

                let x = twisted_x + top_wobble_x;
                let z = twisted_z + top_wobble_z;

                let idx = mesh.pool.add_vertex(
                    Vector3::new(x, self.trunk_height, z),
                    Vector3::new(0.0, 1.0, 0.0),
                    Vector2::new(0.5 + angle.cos() * 0.5, 0.5 + angle.sin() * 0.5),
                );
                top_ring_indices.push(idx);
            }

            // Top cap triangles (counter-clockwise for top-facing)
            for seg in 0..segments {
                let current = top_ring_indices[seg];
                let next = top_ring_indices[(seg + 1) % segments];
                mesh.add_triangle(top_center_idx, current, next);
            }
        }

        (mesh, hole_boundaries)
    }

    /// Generate the entire tree using manifold mesh topology
    fn generate_manifold(&mut self) {
        self.clear();

        // 1. Generate branch origins first (we need attachment info before trunk)
        let mut config = self.create_branch_config();
        let mut rng = SeededRng::new(self.seed);

        // Apply preview recursion depth override
        if self.preview_recursion_depth >= 0 {
            config.branch_recursion = self.preview_recursion_depth;
        }

        let mut primary_branches = generate_branch_origins(&config, &mut rng);

        // Apply preview branch limit
        if self.preview_branch_limit > 0 {
            primary_branches.truncate(self.preview_branch_limit as usize);
        }

        // 2. Collect branch attachments for hole cutting
        let attachments = self.collect_branch_attachments(&primary_branches);

        // 3. Generate trunk with holes
        let (mut trunk_mesh, hole_boundaries) = self.create_trunk_mesh_with_holes(&attachments);

        // 4. Generate branches connected to holes
        let branch_segments = (self.radial_segments / 2).max(4);
        let mut all_branches: Vec<BranchSegment> = Vec::new();

        // Generate leader branch if enabled
        if self.trunk_termination == TrunkTermination::LeaderBranch {
            let leader = self.create_leader_branch();
            let leader_mesh = generate_branch_mesh_with_config(
                &leader,
                branch_segments,
                self.branch_twist,
                self.gravity_strength,
                self.stiffness,
            );
            // For leader, use regular extend since it's not connected to a hole
            trunk_mesh.extend_mesh_data(&leader_mesh);
            all_branches.push(leader.clone());

            // Generate sub-branches on leader if enabled
            if self.leader_has_branches {
                let sub_branches = generate_sub_branches(&leader, &config, &mut rng, 0);
                for sub in &sub_branches {
                    let sub_mesh = generate_branch_mesh_with_config(
                        sub,
                        4,
                        self.branch_twist,
                        self.gravity_strength,
                        self.stiffness,
                    );
                    trunk_mesh.extend_mesh_data(&sub_mesh);
                    all_branches.push(sub.clone());
                }
            }
        }

        // Generate primary branches connected to hole boundaries
        for (i, branch) in primary_branches.iter().enumerate() {
            // Generate branch mesh connected to hole boundary
            if i < hole_boundaries.len() && !hole_boundaries[i].is_empty() {
                generate_branch_mesh_manifold(
                    branch,
                    &hole_boundaries[i],
                    &mut trunk_mesh,
                    self.branch_twist,
                    self.gravity_strength,
                    self.stiffness,
                );
            } else {
                // Fallback: generate collar + regular branch if no hole boundary
                if self.branch_collar_enabled {
                    let t = branch.start.y / self.trunk_height;
                    let taper_factor = t.powf(self.trunk_taper_curve);
                    let base_r = self.trunk_radius * self.trunk_flare;
                    let tip_r = self.trunk_radius * self.trunk_taper;
                    let trunk_r_at_height = base_r + (tip_r - base_r) * taper_factor;

                    let collar = crate::junction::generate_branch_collar(
                        branch.start,
                        branch.direction,
                        trunk_r_at_height,
                        branch.base_radius,
                        branch.base_radius * self.branch_collar_length,
                        self.radial_segments / 2,
                    );
                    trunk_mesh.extend_mesh_data(&collar);
                }

                let branch_mesh = generate_branch_mesh_with_config(
                    branch,
                    branch_segments,
                    self.branch_twist,
                    self.gravity_strength,
                    self.stiffness,
                );
                trunk_mesh.extend_mesh_data(&branch_mesh);
            }

            all_branches.push(branch.clone());

            // Generate sub-branches (these don't connect to trunk holes)
            if let Some((stub, split1, split2)) = generate_split_branches(branch, &config, &mut rng)
            {
                // Handle splits - generate stub and split branches
                let stub_mesh = generate_branch_mesh_with_config(
                    &stub,
                    branch_segments,
                    self.branch_twist,
                    self.gravity_strength,
                    self.stiffness,
                );
                trunk_mesh.extend_mesh_data(&stub_mesh);

                for split_branch in [&split1, &split2] {
                    let split_mesh = generate_branch_mesh_with_config(
                        split_branch,
                        branch_segments,
                        self.branch_twist,
                        self.gravity_strength,
                        self.stiffness,
                    );
                    trunk_mesh.extend_mesh_data(&split_mesh);
                    all_branches.push(split_branch.clone());

                    let sub_branches = generate_sub_branches(split_branch, &config, &mut rng, 0);
                    for sub in &sub_branches {
                        let sub_mesh = generate_branch_mesh_with_config(
                            sub,
                            4,
                            self.branch_twist,
                            self.gravity_strength,
                            self.stiffness,
                        );
                        trunk_mesh.extend_mesh_data(&sub_mesh);
                        all_branches.push(sub.clone());
                    }
                }
            } else {
                // Normal sub-branches
                let sub_branches = generate_sub_branches(branch, &config, &mut rng, 0);
                for sub in &sub_branches {
                    let sub_mesh = generate_branch_mesh_with_config(
                        sub,
                        4,
                        self.branch_twist,
                        self.gravity_strength,
                        self.stiffness,
                    );
                    trunk_mesh.extend_mesh_data(&sub_mesh);
                    all_branches.push(sub.clone());
                }
            }
        }

        // 5. Apply junction smoothing (optional, controlled by junction_blend)
        let mut mesh_data = trunk_mesh.to_mesh_data();

        if self.junction_blend > 0.0 {
            // Collect all hole boundary vertex indices for smoothing
            let mut junction_indices = std::collections::HashSet::new();
            for boundary in &hole_boundaries {
                for &idx in &boundary.lower_indices {
                    junction_indices.insert(idx);
                }
                for &idx in &boundary.upper_indices {
                    junction_indices.insert(idx);
                }
            }

            // Apply Laplacian smoothing to junction vertices
            if !junction_indices.is_empty() {
                let iterations = if self.junction_blend > 0.5 { 2 } else { 1 };
                let factor = self.junction_blend * 0.5; // Scale to reasonable range

                smoothing::laplacian_smooth(
                    &mut mesh_data.vertices,
                    &mut mesh_data.normals,
                    &mesh_data.indices,
                    &junction_indices,
                    iterations,
                    factor,
                );
            }
        }

        // 6. Build and apply mesh
        let mesh = self.build_array_mesh(
            mesh_data.vertices,
            mesh_data.normals,
            mesh_data.uvs,
            mesh_data.indices,
        );
        self.apply_mesh(mesh);

        // 7. Generate foliage
        if self.foliage_enabled {
            let foliage_config = self.create_foliage_config();
            let branch_infos = self.branches_to_branch_infos(&all_branches);
            let leaves = collect_leaf_points(&branch_infos, &foliage_config, &mut rng);

            if !leaves.is_empty() {
                let foliage_mesh_data = generate_foliage_mesh(&leaves, self.leaf_style);

                let foliage_mesh = self.build_array_mesh(
                    foliage_mesh_data.vertices,
                    foliage_mesh_data.normals,
                    foliage_mesh_data.uvs,
                    foliage_mesh_data.indices,
                );
                self.apply_foliage_mesh(foliage_mesh);
            }
        }

        // 8. Emit signal
        let height = self.trunk_height;
        let radius = self.trunk_radius;
        self.base_mut().emit_signal(
            "tree_generated",
            &[height.to_variant(), radius.to_variant()],
        );
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

    fn create_color_material(color: Color) -> Gd<StandardMaterial3D> {
        let mut material = StandardMaterial3D::new_gd();
        material.set_albedo(color);
        material
    }

    fn apply_mesh(&mut self, mesh: Gd<ArrayMesh>) {
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_mesh(&mesh);
        instance.set_name("TrunkMesh");

        let material = Self::create_color_material(self.trunk_color);
        instance.set_surface_override_material(0, &material);

        self.base_mut().add_child(&instance);
        self.mesh_instance = Some(instance);
    }

    fn apply_foliage_mesh(&mut self, mesh: Gd<ArrayMesh>) {
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_mesh(&mesh);
        instance.set_name("FoliageMesh");

        let material = Self::create_color_material(self.foliage_color);
        instance.set_surface_override_material(0, &material);

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
        self.foliage_color = values.foliage_color;
    }
}
