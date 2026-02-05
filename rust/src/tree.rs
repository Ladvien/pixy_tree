use godot::classes::base_material_3d::{CullMode, Feature, TextureParam, Transparency};
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, Engine, MeshInstance3D, StandardMaterial3D, Texture2D};
use godot::prelude::*;

use crate::branch::{
    apply_pipe_radius_model, generate_branch_collar, generate_branch_mesh_with_resolution,
    generate_branch_origins, generate_branch_origins_multi_segment, generate_split_branches,
    generate_sub_branches, BranchConfig, BranchSegment, MeshData, SeededRng,
};
use crate::crown_shape::CrownShape;
use crate::foliage::{
    collect_leaf_points, generate_foliage_mesh, BranchInfo, FoliageConfig, FoliagePlacement,
    FoliagePresetValues, LeafOrientation, LeafStyle,
};
use crate::growth::{
    convert_growth_to_branches, create_trunk_structure, simulate_growth, GrowthConfig,
};
use crate::manifold_mesher::{segments_to_tree, ManifoldMesherConfig};
use crate::property::{BranchProperty, PropertyMode};
use crate::smoothing::{laplacian_smooth, laplacian_smooth_weighted, recalculate_normals};
use crate::tree_preset::{
    BonsaiStyle, BonsaiStyleValues, GrowthPreset, GrowthPresetValues, ScaleModifier,
    ScaleModifierValues, SeasonModifier, SeasonModifierValues, StyleModifier, StyleModifierValues,
    TreePreset, TreePresetValues,
};

/// How the trunk terminates at the top
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum TrunkTermination {
    #[default]
    FlatCap = 0, // Current behavior - flat circular cap
    PointedTip = 1,   // Taper trunk to a point (no cap)
    LeaderBranch = 2, // Generate central leader extending up
}

/// M4: Branch taper parameterization style
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum TaperMode {
    /// Legacy Rust style: tip = base * (1 - taper)
    #[default]
    Legacy = 0,
    /// C++ style: tip = base * end_radius (direct end ratio)
    EndRadius = 1,
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

    /// Style modifier for different game art styles
    #[export]
    #[var(get = get_style_modifier, set = set_style_modifier)]
    #[init(val = StyleModifier::None)]
    style_modifier: StyleModifier,

    /// Scale modifier for age/size variants
    #[export]
    #[var(get = get_scale_modifier, set = set_scale_modifier)]
    #[init(val = ScaleModifier::Mature)]
    scale_modifier: ScaleModifier,

    /// Season modifier for seasonal appearance
    #[export]
    #[var(get = get_season_modifier, set = set_season_modifier)]
    #[init(val = SeasonModifier::None)]
    season_modifier: SeasonModifier,

    /// Bonsai style modifier for artistic tree shaping
    #[export]
    #[var(get = get_bonsai_style, set = set_bonsai_style)]
    #[init(val = BonsaiStyle::None)]
    bonsai_style: BonsaiStyle,

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
    #[init(val = 0.1)]
    branch_start: f32,

    /// Where branches end on trunk (0-1 ratio of height)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 1.0)]
    branch_end: f32,

    /// Branches per unit length
    #[export(range = (0.1, 5.0, 0.1))]
    #[init(val = 2.0)]
    branch_density: f32,

    /// Branch length relative to trunk height
    #[export(range = (0.1, 1.0, 0.05))]
    #[init(val = 0.4)]
    branch_length: f32,

    /// Angle from trunk (degrees, 0=up, 90=horizontal)
    #[export(range = (0.0, 90.0, 1.0))]
    #[init(val = 45.0)]
    branch_angle: f32,

    /// H7: Angle property mode (Constant/Random/Curve)
    #[export]
    #[init(val = PropertyMode::Constant)]
    branch_angle_mode: PropertyMode,

    /// H7: Angle curve end ratio (for Curve mode)
    #[export(range = (0.0, 2.0, 0.1))]
    #[init(val = 1.0)]
    branch_angle_curve_end: f32,

    /// H7: Angle curve power (for Curve mode)
    #[export(range = (0.1, 3.0, 0.1))]
    #[init(val = 1.0)]
    branch_angle_curve_power: f32,

    /// H7: Angle variation (for Random mode)
    #[export(range = (0.0, 30.0, 1.0))]
    #[init(val = 5.0)]
    branch_angle_variation: f32,

    /// Branch radius relative to trunk radius at attachment point
    #[export(range = (0.1, 0.8, 0.05))]
    #[init(val = 0.3)]
    branch_radius_ratio: f32,

    /// Taper from base to tip (0=none, 1=point) - used when taper_mode=Legacy
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.7)]
    branch_taper: f32,

    /// M4: Taper parameterization mode
    #[export]
    #[init(val = TaperMode::Legacy)]
    taper_mode: TaperMode,

    /// M4: End radius ratio (C++ style) - used when taper_mode=EndRadius
    /// Tip radius = base_radius * branch_end_radius
    #[export(range = (0.01, 0.5, 0.01))]
    #[init(val = 0.05)]
    branch_end_radius: f32,

    /// Spiral angle between branches (137.5° = golden angle)
    #[export(range = (0.0, 360.0, 0.5))]
    #[init(val = 137.5)]
    phyllotaxis_angle: f32,

    /// Direction randomness base value
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.4)]
    branch_randomness: f32,

    /// H7: Randomness property mode (Constant/Random/Curve)
    #[export]
    #[init(val = PropertyMode::Constant)]
    branch_randomness_mode: PropertyMode,

    /// H7: Randomness curve end ratio (for Curve mode)
    #[export(range = (0.0, 2.0, 0.1))]
    #[init(val = 1.0)]
    branch_randomness_curve_end: f32,

    /// H7: Randomness curve power (for Curve mode)
    #[export(range = (0.1, 3.0, 0.1))]
    #[init(val = 1.0)]
    branch_randomness_curve_power: f32,

    /// H7: Randomness variation (for Random mode)
    #[export(range = (0.0, 0.5, 0.05))]
    #[init(val = 0.1)]
    branch_randomness_variation: f32,

    /// Upward growth tendency
    #[export(range = (-1.0, 1.0, 0.05))]
    #[init(val = 0.25)]
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

    /// Multi-segment branch resolution: segments per unit length (0 = legacy single-segment)
    #[export(range = (0.0, 10.0, 0.5))]
    #[init(val = 0.0)]
    branch_resolution: f32,

    /// Length multiplier per recursion level
    #[export(range = (0.3, 0.8, 0.05))]
    #[init(val = 0.5)]
    sub_branch_scale: f32,

    /// Random length variation (0 = uniform, 0.5 = high variation)
    #[export(range = (0.0, 0.5, 0.05))]
    #[init(val = 0.15)]
    branch_length_variation: f32,

    /// Branch length curve: ratio at top vs bottom of crown (1.0 = uniform, 0.1 = much shorter at top)
    /// When != 1.0, overrides flat branch_length with a height-dependent curve
    #[export(range = (0.1, 2.0, 0.05))]
    #[init(val = 1.0)]
    branch_length_curve_end: f32,

    /// Branch length curve power (1.0 = linear, 2.0 = quadratic, 0.5 = sqrt)
    #[export(range = (0.1, 3.0, 0.1))]
    #[init(val = 1.0)]
    branch_length_curve_power: f32,

    /// Branch radius curve: ratio at top vs bottom of crown (1.0 = uniform)
    #[export(range = (0.1, 2.0, 0.05))]
    #[init(val = 1.0)]
    branch_radius_curve_end: f32,

    /// Branch radius curve power
    #[export(range = (0.1, 3.0, 0.1))]
    #[init(val = 1.0)]
    branch_radius_curve_power: f32,

    /// Sub-branch position bias (-1 = base, 0 = uniform, 1 = tip)
    #[export(range = (-1.0, 1.0, 0.1))]
    #[init(val = 0.0)]
    sub_branch_position_bias: f32,

    /// Leader dominance over side branches (0 = equal growth, 1 = strong leader)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.7)]
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
    /// Cumulative downward bend along branches (C++ scale: 0-50, default ~10)
    #[export(range = (0.0, 50.0, 0.5))]
    #[init(val = 0.0)]
    gravity_strength: f32,

    /// Resistance to gravity bending (higher = stiffer wood)
    #[export(range = (0.0, 10.0, 0.1))]
    #[init(val = 0.1)]
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
    #[init(val = 0.5)]
    split_probability: f32,

    /// Angle between split branches (degrees)
    #[export(range = (10.0, 60.0, 1.0))]
    #[init(val = 45.0)]
    split_angle: f32,

    /// Where along branch split occurs (0.0-1.0)
    #[export(range = (0.2, 0.8, 0.05))]
    #[init(val = 0.5)]
    split_position: f32,

    /// Minimum branch radius for splitting to occur
    #[export(range = (0.0, 0.5, 0.01))]
    #[init(val = 0.05)]
    split_radius_threshold: f32,

    /// M5: Split child radius multiplier (C++ default 0.9)
    #[export(range = (0.5, 1.0, 0.05))]
    #[init(val = 0.9)]
    split_radius_multiplier: f32,

    // ═══════════════════════════════════════════
    // Floor Avoidance Settings
    // ═══════════════════════════════════════════
    /// Prevent branches from growing below floor level
    #[export]
    #[init(val = true)]
    floor_avoidance: bool,

    /// Y-level of the floor (branches won't grow below this)
    #[export(range = (-10.0, 10.0, 0.1))]
    #[init(val = 0.0)]
    floor_level: f32,

    // ═══════════════════════════════════════════
    // Mesh Smoothing Settings
    // ═══════════════════════════════════════════
    /// Enable Laplacian smoothing for organic surfaces
    #[export]
    #[init(val = false)]
    smooth_enabled: bool,

    /// Number of smoothing passes (more = smoother)
    #[export(range = (1.0, 10.0, 1.0))]
    #[init(val = 2)]
    smooth_iterations: i32,

    /// Smoothing strength per iteration (0.0-1.0)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.5)]
    smooth_factor: f32,

    // ═══════════════════════════════════════════
    // Adaptive Mesh Resolution Settings
    // ═══════════════════════════════════════════
    /// Vary radial segments based on branch radius
    #[export]
    #[init(val = false)]
    adaptive_resolution: bool,

    /// Segments per unit radius (higher = more segments on thick branches)
    #[export(range = (5.0, 50.0, 1.0))]
    #[init(val = 16.0)]
    resolution_scale: f32,

    /// Minimum radial segments for any branch
    #[export(range = (3.0, 8.0, 1.0))]
    #[init(val = 4)]
    min_radial_segments: i32,

    /// Maximum radial segments for any branch
    #[export(range = (8.0, 32.0, 1.0))]
    #[init(val = 16)]
    max_radial_segments: i32,

    /// Length resolution: rings per unit length along branches (0 = legacy fixed rings)
    /// Higher values = smoother curves on longer branches, fewer wasted segments on short ones
    #[export(range = (0.0, 8.0, 0.5))]
    #[init(val = 0.0)]
    length_resolution: f32,

    // ═══════════════════════════════════════════
    // Pipe Radius Model (Da Vinci's Rule)
    // ═══════════════════════════════════════════
    /// Enable pipe radius model for realistic branch thickness
    #[export]
    #[init(val = false)]
    pipe_radius_enabled: bool,

    /// Pipe radius exponent (2.0 = Da Vinci's rule, 1.8-2.3 typical)
    #[export(range = (1.5, 3.0, 0.1))]
    #[init(val = 2.0)]
    pipe_radius_exponent: f32,

    /// Minimum branch radius (prevents branches from disappearing)
    #[export(range = (0.01, 0.2, 0.01))]
    #[init(val = 0.02)]
    pipe_radius_min: f32,

    /// Constant growth: additive radius proportional to branch length
    #[export(range = (0.0, 1.0, 0.01))]
    #[init(val = 0.0)]
    pipe_radius_constant_growth: f32,

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
    // Manifold Mesher
    // ═══════════════════════════════════════════
    /// Enable manifold meshing (watertight branch junctions)
    #[export]
    #[init(val = false)]
    manifold_mesh_enabled: bool,

    /// Enable Pivot Painter 2.0 per-vertex attributes (CUSTOM0/CUSTOM1)
    #[export]
    #[init(val = false)]
    pivot_painter_enabled: bool,

    // ═══════════════════════════════════════════
    // Multiple Stems
    // ═══════════════════════════════════════════
    /// Number of stems (independent trunks)
    #[export(range = (1.0, 5.0, 1.0))]
    #[init(val = 1)]
    stem_count: i32,

    /// Radial spread between stems
    #[export(range = (0.0, 2.0, 0.1))]
    #[init(val = 0.5)]
    stem_spread: f32,

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

    /// Crown base size: fraction of height where crown starts (0 = from branch_start)
    #[export(range = (0.0, 0.9, 0.05))]
    #[init(val = 0.0)]
    crown_base_size: f32,

    /// Crown height override (-1 = auto, uses trunk_height)
    #[export(range = (-1.0, 50.0, 0.5))]
    #[init(val = -1.0)]
    crown_height: f32,

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
    // Texture Settings
    // ═══════════════════════════════════════════
    /// Bark albedo/diffuse texture
    #[export]
    bark_texture: Option<Gd<Texture2D>>,

    /// Bark normal map texture
    #[export]
    bark_normal_map: Option<Gd<Texture2D>>,

    /// Bark roughness map texture
    #[export]
    bark_roughness_map: Option<Gd<Texture2D>>,

    /// Leaf texture (with alpha for transparency)
    #[export]
    foliage_texture: Option<Gd<Texture2D>>,

    /// Leaf normal map texture
    #[export]
    foliage_normal_map: Option<Gd<Texture2D>>,

    /// UV tiling scale for bark textures
    #[export]
    #[init(val = Vector3::new(1.0, 1.0, 1.0))]
    bark_uv_scale: Vector3,

    /// UV tiling scale for foliage textures
    #[export]
    #[init(val = Vector3::new(1.0, 1.0, 1.0))]
    foliage_uv_scale: Vector3,

    /// Trunk roughness when no texture assigned (0 = smooth, 1 = rough)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.8)]
    trunk_roughness: f32,

    /// Trunk metallic value (0 = non-metallic, 1 = metallic)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.0)]
    trunk_metallic: f32,

    /// Foliage roughness (0 = smooth/glossy, 1 = rough/matte)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.5)]
    foliage_roughness: f32,

    /// Foliage metallic value
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.0)]
    foliage_metallic: f32,

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
    // L-System Growth Settings
    // ═══════════════════════════════════════════
    /// Enable L-system growth simulation
    #[export]
    #[init(val = false)]
    growth_enabled: bool,

    /// Growth preset
    #[export]
    #[init(val = GrowthPreset::Custom)]
    growth_preset: GrowthPreset,

    /// Number of growth iterations (years)
    #[export(range = (1.0, 20.0, 1.0))]
    #[init(val = 5)]
    growth_iterations: i32,

    /// Preview iteration (-1 = run all, otherwise stop at that iteration for preview)
    #[export(range = (-1.0, 20.0, 1.0))]
    #[init(val = -1)]
    preview_iteration: i32,

    /// Minimum vigor to extend a branch
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.5)]
    grow_threshold: f32,

    /// Vigor below which branches are pruned
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.1)]
    cut_threshold: f32,

    /// Vigor above which branches bifurcate
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.7)]
    split_threshold: f32,

    /// Enable lateral branching from dormant buds
    #[export]
    #[init(val = true)]
    lateral_enabled: bool,

    /// Where lateral buds start on trunk (0-1)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.1)]
    lateral_start: f32,

    /// Where lateral buds end on trunk (0-1)
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.9)]
    lateral_end: f32,

    /// Lateral bud density (buds per unit length)
    #[export(range = (0.5, 5.0, 0.5))]
    #[init(val = 2.0)]
    lateral_density: f32,

    /// Vigor threshold to activate dormant buds
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.4)]
    lateral_activation: f32,

    /// Initial angle of lateral branches (degrees)
    #[export(range = (20.0, 80.0, 5.0))]
    #[init(val = 45.0)]
    lateral_angle: f32,

    /// Upward/downward growth tendency for L-system
    #[export(range = (-0.5, 0.5, 0.05))]
    #[init(val = 0.1)]
    growth_gravitropism: f32,

    /// Growth direction randomness
    #[export(range = (0.0, 0.5, 0.05))]
    #[init(val = 0.2)]
    growth_randomness: f32,

    /// Enable flowering at low-vigor tips
    #[export]
    #[init(val = false)]
    flowering_enabled: bool,

    /// Vigor threshold for flower conversion
    #[export(range = (0.0, 0.5, 0.05))]
    #[init(val = 0.15)]
    flower_threshold: f32,

    /// Enable dynamic cut threshold adaptation (auto-balances branch count)
    #[export]
    #[init(val = true)]
    dynamic_cut_threshold: bool,

    /// Extension taper: radius ratio when extending (0.95 = original, 0.8 = previous)
    #[export(range = (0.5, 1.0, 0.05))]
    #[init(val = 0.95)]
    growth_extension_taper: f32,

    /// Split taper: radius ratio at bifurcation (0.9 = original, 0.6 = previous)
    #[export(range = (0.3, 1.0, 0.05))]
    #[init(val = 0.9)]
    growth_split_taper: f32,

    /// Enable secondary growth (radius thickening with age)
    #[export]
    #[init(val = false)]
    secondary_growth: bool,

    /// Use competitive vigor distribution (original formula)
    #[export]
    #[init(val = true)]
    competitive_vigor: bool,

    /// Trunk segments per unit height for growth mode (multi-segment trunk)
    #[export(range = (1.0, 10.0, 0.5))]
    #[init(val = 3.0)]
    growth_trunk_resolution: f32,

    /// Trunk vertical bias per segment in growth mode
    #[export(range = (0.0, 1.0, 0.05))]
    #[init(val = 0.6)]
    growth_trunk_up_attraction: f32,

    /// Split angle for growth bifurcation (degrees)
    #[export(range = (10.0, 90.0, 1.0))]
    #[init(val = 60.0)]
    growth_split_angle: f32,

    /// Phyllotaxis angle for growth splits (degrees)
    #[export(range = (0.0, 360.0, 0.5))]
    #[init(val = 137.5)]
    growth_phyllotaxis_angle: f32,

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
            // Apply modifiers after preset
            self.apply_modifiers();
        }
    }

    #[func]
    fn get_style_modifier(&self) -> StyleModifier {
        self.style_modifier
    }

    #[func]
    fn set_style_modifier(&mut self, value: StyleModifier) {
        self.style_modifier = value;
        // Re-apply preset with new modifier
        if let Some(preset_values) = self.preset.get_values() {
            self.reset_to_defaults();
            self.apply_preset_values(&preset_values);
            self.apply_modifiers();
        } else {
            // Custom preset - just apply modifiers to current values
            self.apply_modifiers();
        }
    }

    #[func]
    fn get_scale_modifier(&self) -> ScaleModifier {
        self.scale_modifier
    }

    #[func]
    fn set_scale_modifier(&mut self, value: ScaleModifier) {
        self.scale_modifier = value;
        // Re-apply preset with new modifier
        if let Some(preset_values) = self.preset.get_values() {
            self.reset_to_defaults();
            self.apply_preset_values(&preset_values);
            self.apply_modifiers();
        } else {
            // Custom preset - just apply modifiers to current values
            self.apply_modifiers();
        }
    }

    #[func]
    fn get_season_modifier(&self) -> SeasonModifier {
        self.season_modifier
    }

    #[func]
    fn set_season_modifier(&mut self, value: SeasonModifier) {
        self.season_modifier = value;
        // Re-apply preset with new modifier
        if let Some(preset_values) = self.preset.get_values() {
            self.reset_to_defaults();
            self.apply_preset_values(&preset_values);
            self.apply_modifiers();
        } else {
            // Custom preset - just apply modifiers to current values
            self.apply_modifiers();
        }
    }

    #[func]
    fn get_bonsai_style(&self) -> BonsaiStyle {
        self.bonsai_style
    }

    #[func]
    fn set_bonsai_style(&mut self, value: BonsaiStyle) {
        self.bonsai_style = value;
        // Re-apply preset with new modifier
        if let Some(preset_values) = self.preset.get_values() {
            self.reset_to_defaults();
            self.apply_preset_values(&preset_values);
            self.apply_modifiers();
        } else {
            // Custom preset - just apply modifiers to current values
            self.apply_modifiers();
        }
    }

    /// Apply style, scale, season, and bonsai modifiers to current values
    fn apply_modifiers(&mut self) {
        // Apply style modifier
        if let Some(style_values) = self.style_modifier.get_values() {
            self.apply_style_modifier_values(&style_values);
        }
        // Apply scale modifier
        if let Some(scale_values) = self.scale_modifier.get_values() {
            self.apply_scale_modifier_values(&scale_values);
        }
        // Apply season modifier
        if let Some(season_values) = self.season_modifier.get_values() {
            self.apply_season_modifier_values(&season_values);
        }
        // Apply bonsai style modifier
        if let Some(bonsai_values) = self.bonsai_style.get_values() {
            self.apply_bonsai_style_values(&bonsai_values);
        }
    }

    fn apply_style_modifier_values(&mut self, values: &StyleModifierValues) {
        self.trunk_radius *= values.trunk_radius_mult;
        self.trunk_randomness = (self.trunk_randomness + values.randomness_add).max(0.0);
        self.branch_randomness = (self.branch_randomness + values.randomness_add).max(0.0);
        self.trunk_twist += values.twist_add;
        self.branch_twist += values.twist_add * 0.5;
        self.break_chance = (self.break_chance + values.break_chance_add).clamp(0.0, 0.5);
        if let Some(segments) = values.radial_segments {
            self.radial_segments = segments;
        }
        // Apply recursion multiplier
        self.branch_recursion =
            ((self.branch_recursion as f32 * values.recursion_mult).round() as i32).max(0);
        // Apply foliage density multiplier
        self.foliage_density *= values.foliage_density_mult;
        // Apply smoothing iterations if specified
        if values.smooth_iterations >= 0 {
            self.smooth_iterations = values.smooth_iterations;
            self.smooth_enabled = values.smooth_iterations > 0;
        }
    }

    fn apply_scale_modifier_values(&mut self, values: &ScaleModifierValues) {
        self.trunk_height *= values.height_mult;
        self.trunk_radius *= values.radius_mult;
        self.branch_density *= values.density_mult;
        self.trunk_randomness = (self.trunk_randomness + values.randomness_add).max(0.0);
        self.break_chance = (self.break_chance + values.break_chance_add).clamp(0.0, 0.5);
        if !values.foliage_enabled {
            self.foliage_enabled = false;
        }
    }

    fn apply_season_modifier_values(&mut self, values: &SeasonModifierValues) {
        // Apply foliage density multiplier
        self.foliage_density *= values.foliage_density_mult;
        // Apply leaf size multiplier
        self.leaf_size *= values.leaf_scale_mult;
        // Override foliage color if specified
        if let Some(color) = values.foliage_color {
            self.foliage_color = color;
        }
        // Disable foliage for deciduous winter
        if values.foliage_disabled {
            self.foliage_enabled = false;
        }
        // Note: snow_enabled would be used by a shader or additional mesh generation
        // For now, the value is available in SeasonModifier::get_values() for runtime query
    }

    fn apply_bonsai_style_values(&mut self, values: &BonsaiStyleValues) {
        // Override trunk parameters
        self.trunk_twist = values.trunk_twist;
        self.trunk_taper = values.trunk_taper;

        // Override branch parameters
        self.branch_randomness = values.branch_randomness;
        self.up_attraction = values.up_attraction;
        self.gravity_strength = values.gravity_strength;
        self.branch_start = values.branch_start;
        self.stiffness = values.stiffness;

        // Apply branch density multiplier
        self.branch_density *= values.branch_density_mult;

        // Branch direction bias is stored but not directly applied here
        // It would be used during branch generation if implemented
        // For now, we can approximate it with branch_flatness and twist
        if values.branch_direction_bias.abs() > 0.1 {
            // One-sided bias: increase flatness to make branches more directional
            self.branch_flatness =
                (self.branch_flatness + values.branch_direction_bias.abs() * 0.3).clamp(0.0, 1.0);
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
        self.branch_start = 0.1;
        self.branch_end = 1.0;
        self.branch_density = 2.0;
        self.branch_length = 0.4;
        self.branch_angle = 45.0;
        self.branch_angle_mode = PropertyMode::Constant;
        self.branch_angle_curve_end = 1.0;
        self.branch_angle_curve_power = 1.0;
        self.branch_angle_variation = 5.0;
        self.branch_radius_ratio = 0.3;
        self.branch_taper = 0.7;
        self.taper_mode = TaperMode::Legacy;
        self.branch_end_radius = 0.05;
        self.phyllotaxis_angle = 137.5;
        self.branch_randomness = 0.4;
        self.branch_randomness_mode = PropertyMode::Constant;
        self.branch_randomness_curve_end = 1.0;
        self.branch_randomness_curve_power = 1.0;
        self.branch_randomness_variation = 0.1;
        self.up_attraction = 0.25;
        self.branch_recursion = 1;
        self.sub_branch_count = 2;
        self.sub_branch_scale = 0.5;
        self.branch_length_variation = 0.15;
        self.branch_length_curve_end = 1.0;
        self.branch_length_curve_power = 1.0;
        self.branch_radius_curve_end = 1.0;
        self.branch_radius_curve_power = 1.0;
        self.sub_branch_position_bias = 0.0;
        self.apical_dominance = 0.5;
        self.branch_flatness = 0.0;
        self.branch_angle_curve = 0.0;
        self.crown_angle_variation = 0.0;
        self.branch_resolution = 0.0;

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
        self.split_probability = 0.5;
        self.split_angle = 45.0;
        self.split_position = 0.5;
        self.split_radius_threshold = 0.05;
        self.split_radius_multiplier = 0.9;

        // Floor Avoidance
        self.floor_avoidance = true;
        self.floor_level = 0.0;

        // Mesh Smoothing
        self.smooth_enabled = false;
        self.smooth_iterations = 2;
        self.smooth_factor = 0.5;

        // Adaptive Resolution
        self.adaptive_resolution = false;
        self.resolution_scale = 16.0;
        self.min_radial_segments = 4;
        self.max_radial_segments = 16;

        // Adaptive Resolution
        self.length_resolution = 0.0;

        // Pipe Radius Model
        self.pipe_radius_enabled = false;
        self.pipe_radius_exponent = 2.0;
        self.pipe_radius_min = 0.02;
        self.pipe_radius_constant_growth = 0.0;

        // Branch Collar
        self.branch_collar_enabled = true;
        self.branch_collar_length = 1.5;

        // Manifold Mesher
        self.manifold_mesh_enabled = false;
        self.pivot_painter_enabled = false;

        // Multiple Stems
        self.stem_count = 1;
        self.stem_spread = 0.5;

        // Crown
        self.crown_shape = CrownShape::Cylindrical;
        self.crown_influence = 1.0;
        self.crown_base_size = 0.0;
        self.crown_height = -1.0;

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

        // Textures - reset to None (use colors as fallback)
        self.bark_texture = None;
        self.bark_normal_map = None;
        self.bark_roughness_map = None;
        self.foliage_texture = None;
        self.foliage_normal_map = None;
        self.bark_uv_scale = Vector3::new(1.0, 1.0, 1.0);
        self.foliage_uv_scale = Vector3::new(1.0, 1.0, 1.0);
        self.trunk_roughness = 0.8;
        self.trunk_metallic = 0.0;
        self.foliage_roughness = 0.5;
        self.foliage_metallic = 0.0;

        // L-System Growth
        self.growth_enabled = false;
        self.growth_preset = GrowthPreset::Custom;
        self.growth_iterations = 5;
        self.preview_iteration = -1;
        self.grow_threshold = 0.3;
        self.cut_threshold = 0.1;
        self.split_threshold = 0.7;
        self.lateral_enabled = true;
        self.lateral_start = 0.1;
        self.lateral_end = 0.9;
        self.lateral_density = 2.0;
        self.lateral_activation = 0.4;
        self.lateral_angle = 45.0;
        self.growth_gravitropism = 0.1;
        self.growth_randomness = 0.2;
        self.flowering_enabled = false;
        self.flower_threshold = 0.15;
        self.dynamic_cut_threshold = true;
        self.growth_extension_taper = 0.95;
        self.growth_split_taper = 0.9;
        self.secondary_growth = false;
        self.competitive_vigor = true;
        self.growth_trunk_resolution = 3.0;
        self.growth_trunk_up_attraction = 0.6;
        self.growth_split_angle = 60.0;
        self.growth_phyllotaxis_angle = 137.5;
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
        self.split_radius_multiplier = values.split_radius_multiplier;

        // Floor Avoidance
        self.floor_avoidance = values.floor_avoidance;
        self.floor_level = values.floor_level;

        // Branch Collar
        self.branch_collar_enabled = values.branch_collar_enabled;
        self.branch_collar_length = values.branch_collar_length;

        // Crown
        self.crown_shape = values.crown_shape;
        self.crown_influence = values.crown_influence;
        self.crown_base_size = values.crown_base_size;
        self.crown_height = values.crown_height;

        // Materials
        self.trunk_color = values.trunk_color;
        self.foliage_color = values.foliage_color;

        // Foliage
        if let Some(foliage) = &values.foliage {
            self.apply_foliage_preset_values(foliage);
        }

        // Growth
        if let Some(growth) = &values.growth {
            self.apply_growth_preset_values(growth);
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
        hash = hash.wrapping_add((self.branch_angle_mode as u64).wrapping_mul(863));
        hash = hash.wrapping_add((self.branch_angle_curve_end.to_bits() as u64).wrapping_mul(877));
        hash =
            hash.wrapping_add((self.branch_angle_curve_power.to_bits() as u64).wrapping_mul(881));
        hash = hash.wrapping_add((self.branch_angle_variation.to_bits() as u64).wrapping_mul(883));
        hash = hash.wrapping_add((self.branch_radius_ratio.to_bits() as u64).wrapping_mul(71));
        hash = hash.wrapping_add((self.branch_taper.to_bits() as u64).wrapping_mul(73));
        hash = hash.wrapping_add((self.taper_mode as u64).wrapping_mul(929));
        hash = hash.wrapping_add((self.branch_end_radius.to_bits() as u64).wrapping_mul(937));
        hash = hash.wrapping_add((self.phyllotaxis_angle.to_bits() as u64).wrapping_mul(79));
        hash = hash.wrapping_add((self.branch_randomness.to_bits() as u64).wrapping_mul(83));
        hash = hash.wrapping_add((self.branch_randomness_mode as u64).wrapping_mul(887));
        hash = hash
            .wrapping_add((self.branch_randomness_curve_end.to_bits() as u64).wrapping_mul(907));
        hash = hash
            .wrapping_add((self.branch_randomness_curve_power.to_bits() as u64).wrapping_mul(911));
        hash = hash
            .wrapping_add((self.branch_randomness_variation.to_bits() as u64).wrapping_mul(919));
        hash = hash.wrapping_add((self.up_attraction.to_bits() as u64).wrapping_mul(89));
        hash = hash.wrapping_add((self.branch_recursion as u64).wrapping_mul(97));
        hash = hash.wrapping_add((self.sub_branch_count as u64).wrapping_mul(101));
        hash = hash.wrapping_add((self.sub_branch_scale.to_bits() as u64).wrapping_mul(103));
        hash = hash.wrapping_add((self.branch_flatness.to_bits() as u64).wrapping_mul(313));
        hash = hash.wrapping_add((self.branch_angle_curve.to_bits() as u64).wrapping_mul(317));
        hash = hash.wrapping_add((self.crown_angle_variation.to_bits() as u64).wrapping_mul(373));
        hash = hash.wrapping_add((self.branch_resolution.to_bits() as u64).wrapping_mul(853));
        hash = hash.wrapping_add((self.branch_length_variation.to_bits() as u64).wrapping_mul(337));
        hash = hash.wrapping_add((self.branch_length_curve_end.to_bits() as u64).wrapping_mul(743));
        hash =
            hash.wrapping_add((self.branch_length_curve_power.to_bits() as u64).wrapping_mul(751));
        hash = hash.wrapping_add((self.branch_radius_curve_end.to_bits() as u64).wrapping_mul(757));
        hash =
            hash.wrapping_add((self.branch_radius_curve_power.to_bits() as u64).wrapping_mul(761));
        hash =
            hash.wrapping_add((self.sub_branch_position_bias.to_bits() as u64).wrapping_mul(347));
        hash = hash.wrapping_add((self.apical_dominance.to_bits() as u64).wrapping_mul(349));
        hash = hash.wrapping_add((self.crown_shape as u64).wrapping_mul(109));
        hash = hash.wrapping_add((self.crown_influence.to_bits() as u64).wrapping_mul(113));
        hash = hash.wrapping_add((self.crown_base_size.to_bits() as u64).wrapping_mul(829));
        hash = hash.wrapping_add((self.crown_height.to_bits() as u64).wrapping_mul(839));
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
        hash = hash.wrapping_add((self.split_radius_multiplier.to_bits() as u64).wrapping_mul(857));
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
        // L-System Growth parameters
        hash = hash.wrapping_add((self.growth_enabled as u64).wrapping_mul(431));
        hash = hash.wrapping_add((self.growth_preset as u64).wrapping_mul(433));
        hash = hash.wrapping_add((self.growth_iterations as u64).wrapping_mul(439));
        hash = hash.wrapping_add((self.preview_iteration as u64).wrapping_mul(440));
        hash = hash.wrapping_add((self.grow_threshold.to_bits() as u64).wrapping_mul(443));
        hash = hash.wrapping_add((self.cut_threshold.to_bits() as u64).wrapping_mul(449));
        hash = hash.wrapping_add((self.split_threshold.to_bits() as u64).wrapping_mul(457));
        hash = hash.wrapping_add((self.lateral_enabled as u64).wrapping_mul(461));
        hash = hash.wrapping_add((self.lateral_start.to_bits() as u64).wrapping_mul(463));
        hash = hash.wrapping_add((self.lateral_end.to_bits() as u64).wrapping_mul(467));
        hash = hash.wrapping_add((self.lateral_density.to_bits() as u64).wrapping_mul(479));
        hash = hash.wrapping_add((self.lateral_activation.to_bits() as u64).wrapping_mul(487));
        hash = hash.wrapping_add((self.lateral_angle.to_bits() as u64).wrapping_mul(491));
        hash = hash.wrapping_add((self.growth_gravitropism.to_bits() as u64).wrapping_mul(499));
        hash = hash.wrapping_add((self.growth_randomness.to_bits() as u64).wrapping_mul(503));
        hash = hash.wrapping_add((self.flowering_enabled as u64).wrapping_mul(509));
        hash = hash.wrapping_add((self.flower_threshold.to_bits() as u64).wrapping_mul(521));
        hash = hash.wrapping_add((self.dynamic_cut_threshold as u64).wrapping_mul(709));
        hash = hash.wrapping_add((self.growth_extension_taper.to_bits() as u64).wrapping_mul(719));
        hash = hash.wrapping_add((self.growth_split_taper.to_bits() as u64).wrapping_mul(727));
        hash = hash.wrapping_add((self.secondary_growth as u64).wrapping_mul(733));
        hash = hash.wrapping_add((self.competitive_vigor as u64).wrapping_mul(739));
        hash = hash.wrapping_add((self.growth_trunk_resolution.to_bits() as u64).wrapping_mul(811));
        hash =
            hash.wrapping_add((self.growth_trunk_up_attraction.to_bits() as u64).wrapping_mul(821));
        hash = hash.wrapping_add((self.growth_split_angle.to_bits() as u64).wrapping_mul(823));
        hash =
            hash.wrapping_add((self.growth_phyllotaxis_angle.to_bits() as u64).wrapping_mul(827));
        // Manifold mesher parameters
        hash = hash.wrapping_add((self.manifold_mesh_enabled as u64).wrapping_mul(773));
        hash = hash.wrapping_add((self.pivot_painter_enabled as u64).wrapping_mul(787));
        // Multiple stems parameters
        hash = hash.wrapping_add((self.stem_count as u64).wrapping_mul(797));
        hash = hash.wrapping_add((self.stem_spread.to_bits() as u64).wrapping_mul(809));
        // Floor avoidance parameters
        hash = hash.wrapping_add((self.floor_avoidance as u64).wrapping_mul(523));
        hash = hash.wrapping_add((self.floor_level.to_bits() as u64).wrapping_mul(541));
        // Mesh smoothing parameters
        hash = hash.wrapping_add((self.smooth_enabled as u64).wrapping_mul(547));
        hash = hash.wrapping_add((self.smooth_iterations as u64).wrapping_mul(557));
        hash = hash.wrapping_add((self.smooth_factor.to_bits() as u64).wrapping_mul(563));
        // Adaptive resolution parameters
        hash = hash.wrapping_add((self.adaptive_resolution as u64).wrapping_mul(569));
        hash = hash.wrapping_add((self.resolution_scale.to_bits() as u64).wrapping_mul(571));
        hash = hash.wrapping_add((self.min_radial_segments as u64).wrapping_mul(577));
        hash = hash.wrapping_add((self.max_radial_segments as u64).wrapping_mul(587));
        hash = hash.wrapping_add((self.length_resolution.to_bits() as u64).wrapping_mul(701));
        // Pipe radius parameters
        hash = hash.wrapping_add((self.pipe_radius_enabled as u64).wrapping_mul(593));
        hash = hash.wrapping_add((self.pipe_radius_exponent.to_bits() as u64).wrapping_mul(599));
        hash = hash.wrapping_add((self.pipe_radius_min.to_bits() as u64).wrapping_mul(601));
        hash = hash
            .wrapping_add((self.pipe_radius_constant_growth.to_bits() as u64).wrapping_mul(769));
        // Texture parameters (use instance_id as proxy for texture change detection)
        if let Some(ref tex) = self.bark_texture {
            hash = hash.wrapping_add((tex.instance_id().to_i64() as u64).wrapping_mul(607));
        }
        if let Some(ref tex) = self.bark_normal_map {
            hash = hash.wrapping_add((tex.instance_id().to_i64() as u64).wrapping_mul(613));
        }
        if let Some(ref tex) = self.bark_roughness_map {
            hash = hash.wrapping_add((tex.instance_id().to_i64() as u64).wrapping_mul(617));
        }
        if let Some(ref tex) = self.foliage_texture {
            hash = hash.wrapping_add((tex.instance_id().to_i64() as u64).wrapping_mul(619));
        }
        if let Some(ref tex) = self.foliage_normal_map {
            hash = hash.wrapping_add((tex.instance_id().to_i64() as u64).wrapping_mul(631));
        }
        // UV scale parameters
        hash = hash.wrapping_add((self.bark_uv_scale.x.to_bits() as u64).wrapping_mul(641));
        hash = hash.wrapping_add((self.bark_uv_scale.y.to_bits() as u64).wrapping_mul(643));
        hash = hash.wrapping_add((self.bark_uv_scale.z.to_bits() as u64).wrapping_mul(647));
        hash = hash.wrapping_add((self.foliage_uv_scale.x.to_bits() as u64).wrapping_mul(653));
        hash = hash.wrapping_add((self.foliage_uv_scale.y.to_bits() as u64).wrapping_mul(659));
        hash = hash.wrapping_add((self.foliage_uv_scale.z.to_bits() as u64).wrapping_mul(661));
        // Material properties
        hash = hash.wrapping_add((self.trunk_roughness.to_bits() as u64).wrapping_mul(673));
        hash = hash.wrapping_add((self.trunk_metallic.to_bits() as u64).wrapping_mul(677));
        hash = hash.wrapping_add((self.foliage_roughness.to_bits() as u64).wrapping_mul(683));
        hash = hash.wrapping_add((self.foliage_metallic.to_bits() as u64).wrapping_mul(691));
        hash
    }

    #[func]
    pub fn generate(&mut self) {
        if self.growth_enabled {
            self.generate_with_growth();
            return;
        }

        self.clear();

        // Calculate stem offsets for multi-stem support
        let stem_offsets: Vec<Vector3> = if self.stem_count > 1 {
            (0..self.stem_count)
                .map(|i| {
                    let angle = i as f32 / self.stem_count as f32 * std::f32::consts::TAU;
                    Vector3::new(
                        angle.cos() * self.stem_spread,
                        0.0,
                        angle.sin() * self.stem_spread,
                    )
                })
                .collect()
        } else {
            vec![Vector3::ZERO]
        };

        let mut all_mesh_data = MeshData::new();
        let mut all_branch_segments: Vec<BranchSegment> = Vec::new();

        for (stem_idx, &stem_offset) in stem_offsets.iter().enumerate() {
            // 1. Generate branches (collect all first, then generate meshes)
            let mut config = self.create_branch_config();
            // Vary seed per stem for unique branching
            let stem_seed = self.seed.wrapping_add(stem_idx as i32 * 1337);
            let mut rng = SeededRng::new(stem_seed);
            config.seed = stem_seed;

            // Apply preview recursion depth override
            if self.preview_recursion_depth >= 0 {
                config.branch_recursion = self.preview_recursion_depth;
            }

            // Default branch segments (used when adaptive resolution is off)
            let default_branch_segments = (self.radial_segments / 2).max(4);

            // Collect all branches (with their collar info for later mesh generation)
            let mut all_branches: Vec<BranchSegment>;
            let mut stub_branches: Vec<BranchSegment> = Vec::new();

            if config.resolution > 0.0 {
                // ═══════════════════════════════════════════
                // Multi-segment BFS path (AG1-AG5)
                // ═══════════════════════════════════════════
                all_branches = generate_branch_origins_multi_segment(&config, &mut rng);

                // Leader branch (unchanged, single-segment)
                if self.trunk_termination == TrunkTermination::LeaderBranch {
                    let leader = self.create_leader_branch();
                    all_branches.push(leader.clone());
                    if self.leader_has_branches {
                        let sub_branches = generate_sub_branches(
                            &leader,
                            &config,
                            &mut rng,
                            0,
                            self.gravity_strength,
                            self.stiffness,
                        );
                        all_branches.extend(sub_branches);
                    }
                }
            } else {
                // ═══════════════════════════════════════════
                // Legacy single-segment path (unchanged)
                // ═══════════════════════════════════════════
                all_branches = Vec::new();
                let mut primary_branches = generate_branch_origins(&config, &mut rng);

                // Apply preview branch limit
                if self.preview_branch_limit > 0 {
                    primary_branches.truncate(self.preview_branch_limit as usize);
                }

                // Collect leader branch if enabled
                if self.trunk_termination == TrunkTermination::LeaderBranch {
                    let leader = self.create_leader_branch();
                    all_branches.push(leader.clone());

                    if self.leader_has_branches {
                        let sub_branches = generate_sub_branches(
                            &leader,
                            &config,
                            &mut rng,
                            0,
                            self.gravity_strength,
                            self.stiffness,
                        );
                        all_branches.extend(sub_branches);
                    }
                }

                // Collect all primary branches and their sub-branches
                for branch in &primary_branches {
                    if let Some((stub, split1, split2)) =
                        generate_split_branches(branch, &config, &mut rng)
                    {
                        stub_branches.push(stub);
                        all_branches.push(split1.clone());
                        all_branches.push(split2.clone());
                        let sub1 = generate_sub_branches(
                            &split1,
                            &config,
                            &mut rng,
                            0,
                            self.gravity_strength,
                            self.stiffness,
                        );
                        let sub2 = generate_sub_branches(
                            &split2,
                            &config,
                            &mut rng,
                            0,
                            self.gravity_strength,
                            self.stiffness,
                        );
                        all_branches.extend(sub1);
                        all_branches.extend(sub2);
                    } else {
                        all_branches.push(branch.clone());
                        let sub_branches = generate_sub_branches(
                            branch,
                            &config,
                            &mut rng,
                            0,
                            self.gravity_strength,
                            self.stiffness,
                        );
                        all_branches.extend(sub_branches);
                    }
                }
            }

            // 1.5. Apply pipe radius model if enabled (before mesh generation)
            if self.pipe_radius_enabled {
                apply_pipe_radius_model(
                    &mut all_branches,
                    self.pipe_radius_exponent,
                    self.pipe_radius_min,
                    self.pipe_radius_constant_growth,
                );
                // Also apply to stub branches
                apply_pipe_radius_model(
                    &mut stub_branches,
                    self.pipe_radius_exponent,
                    self.pipe_radius_min,
                    self.pipe_radius_constant_growth,
                );
            }

            // 2. Generate trunk mesh data
            let mut mesh_data = self.create_trunk_mesh_data();

            if self.manifold_mesh_enabled {
                // Manifold mesher path: convert branches to tree graph and generate watertight mesh
                let branch_trees = segments_to_tree(&all_branches);
                let positions: Vec<Vector3> = branch_trees
                    .iter()
                    .map(|_| Vector3::ZERO) // Positions relative to trunk
                    .collect();
                let manifold_config = ManifoldMesherConfig {
                    radial_resolution: self.radial_segments.max(4) as usize,
                    smooth_iterations: if self.smooth_enabled {
                        self.smooth_iterations as u32
                    } else {
                        0
                    },
                    smooth_factor: self.smooth_factor,
                    pivot_painter_enabled: self.pivot_painter_enabled,
                };
                let manifold_result =
                    crate::manifold_mesher::mesh_tree(&branch_trees, &positions, &manifold_config);
                mesh_data.extend(&manifold_result.mesh);
            } else {
                // Legacy per-branch cylinder path
                // 2.5. Generate stub meshes (from splits, no collars)
                for stub in &stub_branches {
                    let stub_segments = if self.adaptive_resolution {
                        self.get_radial_segments_for_radius(stub.base_radius)
                    } else {
                        default_branch_segments
                    };
                    let stub_mesh = generate_branch_mesh_with_resolution(
                        stub,
                        stub_segments,
                        self.branch_twist,
                        self.gravity_strength,
                        self.stiffness,
                        self.length_resolution,
                    );
                    mesh_data.extend(&stub_mesh);
                }

                // 3. Generate branch meshes (with collars for primary branches)
                for branch in &all_branches {
                    // Generate branch collar for depth-0 branches (primary branches from trunk)
                    if self.branch_collar_enabled && branch.depth == 0 {
                        // Calculate trunk radius at branch height
                        let t = branch.start.y / self.trunk_height;
                        let taper_factor = t.powf(self.trunk_taper_curve);
                        let base_r = self.trunk_radius * self.trunk_flare;
                        let tip_r = self.trunk_radius * self.trunk_taper;
                        let trunk_r_at_height = base_r + (tip_r - base_r) * taper_factor;

                        // Compute trunk center at branch height (accounts for wobble)
                        let trunk_center = self.trunk_center_at_height(branch.start.y);

                        let collar = generate_branch_collar(
                            branch.start,
                            branch.direction,
                            trunk_center,
                            trunk_r_at_height,
                            Vector3::UP,
                            branch.base_radius,
                            branch.base_radius * self.branch_collar_length,
                            self.radial_segments / 2,
                        );
                        mesh_data.extend(&collar);
                    }

                    // Generate branch mesh
                    let branch_seg_count = if self.adaptive_resolution {
                        self.get_radial_segments_for_radius(branch.base_radius)
                    } else if branch.depth > 0 {
                        4 // Sub-branches get fewer segments
                    } else {
                        default_branch_segments
                    };
                    let branch_mesh = generate_branch_mesh_with_resolution(
                        branch,
                        branch_seg_count,
                        self.branch_twist,
                        self.gravity_strength,
                        self.stiffness,
                        self.length_resolution,
                    );
                    mesh_data.extend(&branch_mesh);
                }

                // 4. Apply mesh smoothing if enabled
                if self.smooth_enabled && self.smooth_iterations > 0 {
                    if !mesh_data.smooth_weights.is_empty()
                        && mesh_data.smooth_weights.len() == mesh_data.vertices.len()
                    {
                        // Use weighted smoothing (stronger at junctions, lighter on branches)
                        laplacian_smooth_weighted(
                            &mut mesh_data.vertices,
                            &mesh_data.indices,
                            &mesh_data.smooth_weights,
                            self.smooth_iterations as u32,
                            self.smooth_factor,
                        );
                    } else {
                        laplacian_smooth(
                            &mut mesh_data.vertices,
                            &mesh_data.indices,
                            self.smooth_iterations as u32,
                            self.smooth_factor,
                        );
                    }
                    // Recalculate normals after smoothing
                    recalculate_normals(
                        &mesh_data.vertices,
                        &mesh_data.indices,
                        &mut mesh_data.normals,
                    );
                }
            }

            // Apply stem offset to all vertices
            if stem_offset != Vector3::ZERO {
                for v in &mut mesh_data.vertices {
                    *v += stem_offset;
                }
                // Also offset branches for foliage generation
                for b in &mut all_branches {
                    b.start += stem_offset;
                }
            }

            // Accumulate into combined mesh
            all_mesh_data.extend(&mesh_data);
            all_branch_segments.extend(all_branches);
        } // end stem loop

        // 5. Build final trunk/branch mesh
        let mesh = self.build_array_mesh(
            all_mesh_data.vertices,
            all_mesh_data.normals,
            all_mesh_data.uvs,
            all_mesh_data.indices,
        );
        self.apply_mesh(mesh);

        // 5. Generate foliage
        if self.foliage_enabled {
            let foliage_config = self.create_foliage_config();
            let branch_infos = self.branches_to_branch_infos(&all_branch_segments);
            let mut rng = SeededRng::new(self.seed);
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

        // 6. Emit signal with tree dimensions for camera framing
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
            // M4: Compute branch_taper based on taper_mode
            // Legacy: tip = base * (1 - taper), so end_ratio = 1 - taper
            // EndRadius: tip = base * end_radius, so we set taper = 1 - end_radius
            branch_taper: match self.taper_mode {
                TaperMode::Legacy => self.branch_taper,
                TaperMode::EndRadius => 1.0 - self.branch_end_radius,
            },
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
            split_radius_multiplier: self.split_radius_multiplier,
            floor_avoidance: self.floor_avoidance,
            floor_level: self.floor_level,
            branch_length_curve_end: self.branch_length_curve_end,
            branch_length_curve_power: self.branch_length_curve_power,
            branch_radius_curve_end: self.branch_radius_curve_end,
            branch_radius_curve_power: self.branch_radius_curve_power,
            crown_base_size: self.crown_base_size,
            crown_height: self.crown_height,
            resolution: self.branch_resolution,
            length_property: if (self.branch_length_curve_end - 1.0).abs() > 0.001 {
                BranchProperty::curve(
                    self.branch_length * self.trunk_height,
                    self.branch_length_curve_end,
                    self.branch_length_curve_power,
                )
            } else {
                BranchProperty::constant(self.branch_length * self.trunk_height)
            },
            // H7: Randomness property with mode selection
            randomness_property: match self.branch_randomness_mode {
                PropertyMode::Curve => BranchProperty::curve(
                    self.branch_randomness,
                    self.branch_randomness_curve_end,
                    self.branch_randomness_curve_power,
                ),
                PropertyMode::Random => BranchProperty {
                    mode: PropertyMode::Random,
                    base_value: self.branch_randomness,
                    curve_power: 1.0,
                    curve_end_ratio: 1.0,
                    random_variation: self.branch_randomness_variation,
                    x_min: 0.0,
                    x_max: 1.0,
                },
                PropertyMode::Constant => BranchProperty::constant(self.branch_randomness),
            },
            // H7: Angle property with mode selection
            start_angle_property: match self.branch_angle_mode {
                PropertyMode::Curve => BranchProperty::curve(
                    self.branch_angle,
                    self.branch_angle_curve_end,
                    self.branch_angle_curve_power,
                ),
                PropertyMode::Random => BranchProperty {
                    mode: PropertyMode::Random,
                    base_value: self.branch_angle,
                    curve_power: 1.0,
                    curve_end_ratio: 1.0,
                    random_variation: self.branch_angle_variation,
                    x_min: 0.0,
                    x_max: 1.0,
                },
                PropertyMode::Constant => BranchProperty::constant(self.branch_angle),
            },
            start_radius_property: if (self.branch_radius_curve_end - 1.0).abs() > 0.001 {
                BranchProperty::curve(
                    self.branch_radius_ratio,
                    self.branch_radius_curve_end,
                    self.branch_radius_curve_power,
                )
            } else {
                BranchProperty::constant(self.branch_radius_ratio)
            },
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

    /// Calculate radial segments for a branch based on its radius.
    /// Returns adaptive segment count if enabled, otherwise the default.
    fn get_radial_segments_for_radius(&self, radius: f32) -> i32 {
        if self.adaptive_resolution {
            // Scale segments by radius
            let segments = (radius * self.resolution_scale).round() as i32;
            segments.clamp(self.min_radial_segments, self.max_radial_segments)
        } else {
            self.radial_segments
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

        let leader_len = self.trunk_height * self.leader_length;
        BranchSegment {
            start: Vector3::new(top_wobble_x, self.trunk_height, top_wobble_z),
            direction: Vector3::UP,
            length: leader_len,
            base_radius: trunk_top_radius,
            tip_radius: trunk_top_radius * self.leader_taper,
            depth: 0,
            is_terminal: !self.leader_has_branches,
            height_ratio: 1.0,
            subtree_weight: leader_len,
        }
    }

    #[func]
    pub fn clear(&mut self) {
        if let Some(mut instance) = self.mesh_instance.take() {
            if instance.is_instance_valid() {
                // Explicitly remove from parent before freeing to ensure immediate visual removal
                if let Some(mut parent) = instance.get_parent() {
                    parent.remove_child(&instance);
                }
                instance.queue_free();
            }
        }

        if let Some(mut instance) = self.foliage_mesh_instance.take() {
            if instance.is_instance_valid() {
                // Explicitly remove from parent before freeing to ensure immediate visual removal
                if let Some(mut parent) = instance.get_parent() {
                    parent.remove_child(&instance);
                }
                instance.queue_free();
            }
        }
    }

    /// Compute trunk center at a given height, accounting for wobble/randomness.
    /// Used for both trunk mesh generation and collar positioning.
    fn trunk_center_at_height(&self, height: f32) -> Vector3 {
        let trunk_height = self.trunk_height.max(0.001);
        let t = height / trunk_height;
        let seed_f = self.seed as f32;

        let wobble_x = if self.trunk_randomness > 0.0 {
            (t * std::f32::consts::PI + seed_f * 0.1).sin()
                * self.trunk_randomness
                * t
                * trunk_height
                * 0.3
        } else {
            0.0
        };
        let wobble_z = if self.trunk_randomness > 0.0 {
            (t * std::f32::consts::E + seed_f * 0.2).cos()
                * self.trunk_randomness
                * t
                * trunk_height
                * 0.3
        } else {
            0.0
        };

        Vector3::new(wobble_x, height, wobble_z)
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

            // Use shared helper for trunk wobble (consistent with collar positioning)
            let trunk_center = self.trunk_center_at_height(y);
            let wobble_x = trunk_center.x;
            let wobble_z = trunk_center.z;

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
        let should_add_top_cap =
            self.trunk_termination == TrunkTermination::FlatCap && self.trunk_taper >= 0.1;

        if should_add_top_cap {
            let top_twist_angle = self.trunk_twist.to_radians();
            let top_cos_t = top_twist_angle.cos();
            let top_sin_t = top_twist_angle.sin();

            // Use shared helper for top wobble (consistent with collar positioning)
            let top_center = self.trunk_center_at_height(self.trunk_height);
            let top_wobble_x = top_center.x;
            let top_wobble_z = top_center.z;

            let top_center_idx = mesh.vertices.len() as i32;
            mesh.vertices.push(top_center);
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

    /// Create trunk/bark material with optional PBR textures
    fn create_trunk_material(&self) -> Gd<StandardMaterial3D> {
        let mut material = StandardMaterial3D::new_gd();
        material.set_albedo(self.trunk_color);

        // Apply bark albedo texture if provided
        if let Some(ref texture) = self.bark_texture {
            material.set_texture(TextureParam::ALBEDO, texture);
        }

        // Apply bark normal map if provided
        if let Some(ref normal) = self.bark_normal_map {
            material.set_texture(TextureParam::NORMAL, normal);
            material.set_feature(Feature::NORMAL_MAPPING, true);
        }

        // Apply bark roughness map if provided, otherwise use scalar roughness
        if let Some(ref roughness) = self.bark_roughness_map {
            material.set_texture(TextureParam::ROUGHNESS, roughness);
        } else {
            material.set_roughness(self.trunk_roughness);
        }

        material.set_metallic(self.trunk_metallic);
        material.set_uv1_scale(self.bark_uv_scale);

        material
    }

    /// Create foliage material with optional textures and alpha transparency
    fn create_foliage_material(&self) -> Gd<StandardMaterial3D> {
        let mut material = StandardMaterial3D::new_gd();
        material.set_albedo(self.foliage_color);

        // Apply foliage albedo texture if provided (with alpha scissor for transparency)
        if let Some(ref texture) = self.foliage_texture {
            material.set_texture(TextureParam::ALBEDO, texture);
            material.set_transparency(Transparency::ALPHA_SCISSOR);
            material.set_alpha_scissor_threshold(0.5);
        }

        // Apply foliage normal map if provided
        if let Some(ref normal) = self.foliage_normal_map {
            material.set_texture(TextureParam::NORMAL, normal);
            material.set_feature(Feature::NORMAL_MAPPING, true);
        }

        material.set_roughness(self.foliage_roughness);
        material.set_metallic(self.foliage_metallic);
        material.set_uv1_scale(self.foliage_uv_scale);

        // Double-sided rendering for leaves
        material.set_cull_mode(CullMode::DISABLED);

        material
    }

    fn apply_mesh(&mut self, mesh: Gd<ArrayMesh>) {
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_mesh(&mesh);
        instance.set_name("TrunkMesh");

        let material = self.create_trunk_material();
        instance.set_surface_override_material(0, &material);

        self.base_mut().add_child(&instance);
        self.mesh_instance = Some(instance);
    }

    fn apply_foliage_mesh(&mut self, mesh: Gd<ArrayMesh>) {
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_mesh(&mesh);
        instance.set_name("FoliageMesh");

        let material = self.create_foliage_material();
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

    fn apply_growth_preset_values(&mut self, values: &GrowthPresetValues) {
        self.growth_iterations = values.iterations as i32;
        self.grow_threshold = values.grow_threshold;
        self.cut_threshold = values.cut_threshold;
        self.split_threshold = values.split_threshold;
        self.flower_threshold = values.flower_threshold;
        self.apical_dominance = values.apical_dominance;
        self.lateral_start = values.lateral_start;
        self.lateral_end = values.lateral_end;
        self.lateral_density = values.lateral_density;
        self.lateral_activation = values.lateral_activation;
        self.lateral_angle = values.lateral_angle;
        self.growth_gravitropism = values.gravitropism;
        self.growth_randomness = values.randomness;
        self.gravity_strength = values.gravity_strength;
        self.stiffness = values.stiffness;
    }

    /// Create growth configuration from export variables
    fn create_growth_config(&self) -> GrowthConfig {
        let mut config = GrowthConfig {
            grow_threshold: self.grow_threshold,
            cut_threshold: self.cut_threshold,
            split_threshold: self.split_threshold,
            flower_threshold: self.flower_threshold,
            apical_dominance: self.apical_dominance,
            enable_lateral: self.lateral_enabled,
            lateral_start: self.lateral_start,
            lateral_end: self.lateral_end,
            lateral_density: self.lateral_density,
            lateral_activation: self.lateral_activation,
            lateral_angle: self.lateral_angle,
            iterations: self.growth_iterations as u32,
            branch_length: self.branch_length * self.trunk_height * 0.1,
            gravitropism: self.growth_gravitropism,
            randomness: self.growth_randomness,
            gravity_strength: self.gravity_strength,
            stiffness: self.stiffness,
            enable_flowering: self.flowering_enabled,
            trunk_height: self.trunk_height,
            trunk_radius: self.trunk_radius,
            trunk_taper: self.trunk_taper,
            seed: self.seed,
            dynamic_cut_threshold: self.dynamic_cut_threshold,
            extension_taper: self.growth_extension_taper,
            split_taper: self.growth_split_taper,
            lateral_radius_ratio: 0.8,
            secondary_growth: self.secondary_growth,
            competitive_vigor: self.competitive_vigor,
            trunk_resolution: self.growth_trunk_resolution,
            trunk_up_attraction: self.growth_trunk_up_attraction,
            split_angle: self.growth_split_angle,
            phyllotaxis_angle: self.growth_phyllotaxis_angle,
            preview_iteration: self.preview_iteration,
        };

        // Apply growth preset if not Custom
        if let Some(preset_values) = self.growth_preset.get_values() {
            preset_values.apply_to_config(&mut config);
        }

        config
    }

    /// Generate tree using L-system growth simulation
    fn generate_with_growth(&mut self) {
        self.clear();

        // 1. Create growth configuration
        let config = self.create_growth_config();

        // 2. Create initial trunk structure
        let trunk = create_trunk_structure(&config);

        // 3. Simulate growth
        let grown_tree = simulate_growth(trunk, &config);

        // 4. Convert growth tree to branch segments
        let mut branch_segments = convert_growth_to_branches(&grown_tree, config.trunk_height, 0);

        // 4.5. Apply pipe radius model if enabled
        if self.pipe_radius_enabled {
            apply_pipe_radius_model(
                &mut branch_segments,
                self.pipe_radius_exponent,
                self.pipe_radius_min,
                self.pipe_radius_constant_growth,
            );
        }

        // 5. Generate trunk mesh data
        let mut mesh_data = self.create_trunk_mesh_data();

        // 6. Generate branch meshes
        let mut all_branches: Vec<BranchSegment> = Vec::new();

        // Filter out trunk-like segments
        let renderable_branches: Vec<&BranchSegment> = branch_segments
            .iter()
            .filter(|b| !(b.depth == 0 && b.start.y < 0.1))
            .collect();

        if self.manifold_mesh_enabled {
            // Manifold mesher path: convert to tree and generate watertight mesh
            let filtered: Vec<BranchSegment> =
                renderable_branches.iter().map(|b| (*b).clone()).collect();
            let branch_trees = segments_to_tree(&filtered);
            let positions: Vec<Vector3> = branch_trees.iter().map(|_| Vector3::ZERO).collect();
            let manifold_config = ManifoldMesherConfig {
                radial_resolution: self.radial_segments.max(4) as usize,
                smooth_iterations: if self.smooth_enabled {
                    self.smooth_iterations as u32
                } else {
                    0
                },
                smooth_factor: self.smooth_factor,
                pivot_painter_enabled: self.pivot_painter_enabled,
            };
            let manifold_result =
                crate::manifold_mesher::mesh_tree(&branch_trees, &positions, &manifold_config);
            mesh_data.extend(&manifold_result.mesh);
            all_branches = filtered;
        } else {
            // Legacy per-branch cylinder path
            let default_branch_mesh_segments = (self.radial_segments / 2).max(4);

            for branch in &renderable_branches {
                // Generate branch collar for smooth trunk-branch junction
                if self.branch_collar_enabled && branch.depth == 0 {
                    let t = branch.start.y / self.trunk_height;
                    let taper_factor = t.powf(self.trunk_taper_curve);
                    let base_r = self.trunk_radius * self.trunk_flare;
                    let tip_r = self.trunk_radius * self.trunk_taper;
                    let trunk_r_at_height = base_r + (tip_r - base_r) * taper_factor;

                    // Compute trunk center at branch height (accounts for wobble)
                    let trunk_center = self.trunk_center_at_height(branch.start.y);

                    let collar = generate_branch_collar(
                        branch.start,
                        branch.direction,
                        trunk_center,
                        trunk_r_at_height,
                        Vector3::UP,
                        branch.base_radius,
                        branch.base_radius * self.branch_collar_length,
                        self.radial_segments / 2,
                    );
                    mesh_data.extend(&collar);
                }

                // Generate branch mesh
                let branch_seg_count = if self.adaptive_resolution {
                    self.get_radial_segments_for_radius(branch.base_radius)
                } else {
                    default_branch_mesh_segments
                };
                let branch_mesh = generate_branch_mesh_with_resolution(
                    branch,
                    branch_seg_count,
                    self.branch_twist,
                    self.gravity_strength,
                    self.stiffness,
                    self.length_resolution,
                );
                mesh_data.extend(&branch_mesh);
                all_branches.push((*branch).clone());
            }

            // 7. Apply mesh smoothing if enabled
            if self.smooth_enabled && self.smooth_iterations > 0 {
                if !mesh_data.smooth_weights.is_empty()
                    && mesh_data.smooth_weights.len() == mesh_data.vertices.len()
                {
                    laplacian_smooth_weighted(
                        &mut mesh_data.vertices,
                        &mesh_data.indices,
                        &mesh_data.smooth_weights,
                        self.smooth_iterations as u32,
                        self.smooth_factor,
                    );
                } else {
                    laplacian_smooth(
                        &mut mesh_data.vertices,
                        &mesh_data.indices,
                        self.smooth_iterations as u32,
                        self.smooth_factor,
                    );
                }
                recalculate_normals(
                    &mesh_data.vertices,
                    &mesh_data.indices,
                    &mut mesh_data.normals,
                );
            }
        }

        // 8. Build final trunk/branch mesh
        let mesh = self.build_array_mesh(
            mesh_data.vertices,
            mesh_data.normals,
            mesh_data.uvs,
            mesh_data.indices,
        );
        self.apply_mesh(mesh);

        // 9. Generate foliage
        if self.foliage_enabled {
            let foliage_config = self.create_foliage_config();
            let branch_infos = self.branches_to_branch_infos(&all_branches);
            let mut rng = SeededRng::new(self.seed);
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

        // 10. Emit signal with tree dimensions
        let height = self.trunk_height;
        let radius = self.trunk_radius;
        self.base_mut().emit_signal(
            "tree_generated",
            &[height.to_variant(), radius.to_variant()],
        );
    }
}
