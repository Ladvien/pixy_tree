use godot::prelude::*;

use crate::crown_shape::CrownShape;
use crate::foliage::FoliagePresetValues;
use crate::growth::GrowthConfig;
use crate::tree::TrunkTermination;

/// Pre-configured tree presets for common tree types
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum TreePreset {
    #[default]
    Custom = 0,
    Oak = 1,
    Pine = 2,
    Willow = 3,
    Birch = 4,
    Palm = 5,
    Cypress = 6,
    Bonsai = 7,
    Maple = 8,          // Opposite branching pattern
    Spruce = 9,         // Perfect conical shape
    Poplar = 10,        // Columnar growth
    Baobab = 11,        // Bottle trunk, crown-only branches
    DragonTree = 12,    // Binary forking
    JapaneseMaple = 13, // Ornamental, layered
    DeadTree = 14,      // Fantasy/horror, no foliage
    Redwood = 15,       // Massive scale
}

/// Pre-configured growth presets for L-system growth simulation
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum GrowthPreset {
    #[default]
    Custom = 0,
    /// Conifer-like, strong central leader
    Structured = 1,
    /// Oak-like, co-dominant branches
    Spreading = 2,
    /// Willow-like, drooping branches
    Weeping = 3,
    /// Twisted, chaotic growth
    Gnarled = 4,
    /// Columnar growth - extreme vertical, minimal lateral spread (Poplar, Lombardy)
    Columnar = 5,
    /// Forking growth - frequent dichotomous branching (Dragon tree, Joshua tree)
    Forking = 6,
}

/// Holds growth simulation parameters for a preset
#[derive(Clone, Debug)]
pub struct GrowthPresetValues {
    pub grow_threshold: f32,
    pub cut_threshold: f32,
    pub split_threshold: f32,
    pub flower_threshold: f32,
    pub apical_dominance: f32,
    pub lateral_start: f32,
    pub lateral_end: f32,
    pub lateral_density: f32,
    pub lateral_activation: f32,
    pub lateral_angle: f32,
    pub iterations: u32,
    pub branch_length: f32,
    pub gravitropism: f32,
    pub randomness: f32,
    pub gravity_strength: f32,
    pub stiffness: f32,
}

impl GrowthPreset {
    /// Get the preset values, or None for Custom preset
    pub fn get_values(&self) -> Option<GrowthPresetValues> {
        match self {
            GrowthPreset::Custom => None,
            GrowthPreset::Structured => Some(GrowthPresetValues::structured()),
            GrowthPreset::Spreading => Some(GrowthPresetValues::spreading()),
            GrowthPreset::Weeping => Some(GrowthPresetValues::weeping()),
            GrowthPreset::Gnarled => Some(GrowthPresetValues::gnarled()),
            GrowthPreset::Columnar => Some(GrowthPresetValues::columnar()),
            GrowthPreset::Forking => Some(GrowthPresetValues::forking()),
        }
    }
}

impl GrowthPresetValues {
    /// Structured: Conifer-like with strong central leader
    pub fn structured() -> Self {
        Self {
            grow_threshold: 0.4,
            cut_threshold: 0.2,
            split_threshold: 0.8,
            flower_threshold: 0.15,
            apical_dominance: 0.85,
            lateral_start: 0.1,
            lateral_end: 0.9,
            lateral_density: 2.5,
            lateral_activation: 0.5,
            lateral_angle: 60.0,
            iterations: 5,
            branch_length: 0.4,
            gravitropism: 0.15,
            randomness: 0.05,
            gravity_strength: 0.0,
            stiffness: 0.8,
        }
    }

    /// Spreading: Oak-like with co-dominant branches
    pub fn spreading() -> Self {
        Self {
            grow_threshold: 0.3,
            cut_threshold: 0.1,
            split_threshold: 0.6,
            flower_threshold: 0.15,
            apical_dominance: 0.5,
            lateral_start: 0.2,
            lateral_end: 0.8,
            lateral_density: 2.0,
            lateral_activation: 0.35,
            lateral_angle: 50.0,
            iterations: 5,
            branch_length: 0.5,
            gravitropism: 0.05,
            randomness: 0.1,
            gravity_strength: 0.1,
            stiffness: 0.6,
        }
    }

    /// Weeping: Willow-like with drooping branches
    pub fn weeping() -> Self {
        Self {
            grow_threshold: 0.3,
            cut_threshold: 0.1,
            split_threshold: 0.7,
            flower_threshold: 0.15,
            apical_dominance: 0.6,
            lateral_start: 0.3,
            lateral_end: 0.9,
            lateral_density: 3.0,
            lateral_activation: 0.3,
            lateral_angle: 45.0,
            iterations: 6,
            branch_length: 0.6,
            gravitropism: -0.1,
            randomness: 0.15,
            gravity_strength: 0.3,
            stiffness: 0.3,
        }
    }

    /// Gnarled: Twisted, chaotic growth
    pub fn gnarled() -> Self {
        Self {
            grow_threshold: 0.25,
            cut_threshold: 0.1,
            split_threshold: 0.5,
            flower_threshold: 0.15,
            apical_dominance: 0.4,
            lateral_start: 0.15,
            lateral_end: 0.85,
            lateral_density: 1.5,
            lateral_activation: 0.3,
            lateral_angle: 55.0,
            iterations: 6,
            branch_length: 0.4,
            gravitropism: 0.0,
            randomness: 0.3,
            gravity_strength: 0.15,
            stiffness: 0.4,
        }
    }

    /// Columnar: Extreme vertical growth for Poplar, Lombardy types
    pub fn columnar() -> Self {
        Self {
            grow_threshold: 0.4,
            cut_threshold: 0.15,
            split_threshold: 0.9, // Rarely splits
            flower_threshold: 0.1,
            apical_dominance: 0.95, // Very strong central leader
            lateral_start: 0.2,
            lateral_end: 0.95,
            lateral_density: 1.5,
            lateral_activation: 0.4,
            lateral_angle: 25.0, // Very narrow angle
            iterations: 5,
            branch_length: 0.35,
            gravitropism: 0.3, // Strong upward tendency
            randomness: 0.03,  // Very consistent
            gravity_strength: 0.0,
            stiffness: 0.95, // Very rigid
        }
    }

    /// Forking: Dichotomous branching for Dragon tree, Joshua tree
    pub fn forking() -> Self {
        Self {
            grow_threshold: 0.3,
            cut_threshold: 0.1,
            split_threshold: 0.3, // Frequent forking
            flower_threshold: 0.15,
            apical_dominance: 0.4, // Low - branches compete equally
            lateral_start: 0.3,
            lateral_end: 0.9,
            lateral_density: 1.0, // Moderate density
            lateral_activation: 0.35,
            lateral_angle: 40.0,
            iterations: 5,
            branch_length: 0.45,
            gravitropism: 0.05,
            randomness: 0.12,
            gravity_strength: 0.1,
            stiffness: 0.6,
        }
    }

    /// Apply these preset values to a GrowthConfig
    pub fn apply_to_config(&self, config: &mut GrowthConfig) {
        config.grow_threshold = self.grow_threshold;
        config.cut_threshold = self.cut_threshold;
        config.split_threshold = self.split_threshold;
        config.flower_threshold = self.flower_threshold;
        config.apical_dominance = self.apical_dominance;
        config.lateral_start = self.lateral_start;
        config.lateral_end = self.lateral_end;
        config.lateral_density = self.lateral_density;
        config.lateral_activation = self.lateral_activation;
        config.lateral_angle = self.lateral_angle;
        config.iterations = self.iterations;
        config.branch_length = self.branch_length;
        config.gravitropism = self.gravitropism;
        config.randomness = self.randomness;
        config.gravity_strength = self.gravity_strength;
        config.stiffness = self.stiffness;
    }
}

/// Holds all tree parameters for a preset
#[derive(Clone, Debug)]
pub struct TreePresetValues {
    // Trunk
    pub trunk_height: f32,
    pub trunk_radius: f32,
    pub radial_segments: i32,
    pub height_segments: i32,

    // Trunk Taper
    pub trunk_taper: f32,
    pub trunk_taper_curve: f32,
    pub trunk_flare: f32,
    pub trunk_randomness: f32,
    pub root_flare_count: i32,
    pub root_flare_spread: f32,
    pub root_flare_height: f32,

    // Trunk Termination
    pub trunk_termination: TrunkTermination,
    pub leader_length: f32,
    pub leader_taper: f32,
    pub leader_has_branches: bool,

    // Branch
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

    // Twist
    pub trunk_twist: f32,
    pub branch_twist: f32,

    // Gravity
    pub gravity_strength: f32,
    pub stiffness: f32,

    // Branch Randomness
    pub break_chance: f32,

    // Splitting
    pub split_enabled: bool,
    pub split_probability: f32,
    pub split_angle: f32,
    pub split_position: f32,
    pub split_radius_threshold: f32,

    // Floor Avoidance
    pub floor_avoidance: bool,
    pub floor_level: f32,

    // Branch Collar
    pub branch_collar_enabled: bool,
    pub branch_collar_length: f32,

    // Crown
    pub crown_shape: CrownShape,
    pub crown_influence: f32,
    pub crown_base_size: f32,
    pub crown_height: f32,

    // Materials
    pub trunk_color: Color,
    pub foliage_color: Color,

    // Foliage
    pub foliage: Option<FoliagePresetValues>,

    // Growth
    pub growth: Option<GrowthPresetValues>,
}

impl TreePreset {
    /// Get the preset values, or None for Custom preset
    pub fn get_values(&self) -> Option<TreePresetValues> {
        match self {
            TreePreset::Custom => None,
            TreePreset::Oak => Some(TreePresetValues::oak()),
            TreePreset::Pine => Some(TreePresetValues::pine()),
            TreePreset::Willow => Some(TreePresetValues::willow()),
            TreePreset::Birch => Some(TreePresetValues::birch()),
            TreePreset::Palm => Some(TreePresetValues::palm()),
            TreePreset::Cypress => Some(TreePresetValues::cypress()),
            TreePreset::Bonsai => Some(TreePresetValues::bonsai()),
            TreePreset::Maple => Some(TreePresetValues::maple()),
            TreePreset::Spruce => Some(TreePresetValues::spruce()),
            TreePreset::Poplar => Some(TreePresetValues::poplar()),
            TreePreset::Baobab => Some(TreePresetValues::baobab()),
            TreePreset::DragonTree => Some(TreePresetValues::dragon_tree()),
            TreePreset::JapaneseMaple => Some(TreePresetValues::japanese_maple()),
            TreePreset::DeadTree => Some(TreePresetValues::dead_tree()),
            TreePreset::Redwood => Some(TreePresetValues::redwood()),
        }
    }
}

impl TreePresetValues {
    /// Oak: Wide spreading branches, medium height, spherical crown
    pub fn oak() -> Self {
        Self {
            trunk_height: 6.0,
            trunk_radius: 0.6,
            radial_segments: 8,
            height_segments: 4,
            trunk_taper: 0.3,
            trunk_taper_curve: 0.5,
            trunk_flare: 1.2,
            trunk_randomness: 0.05,
            root_flare_count: 5,
            root_flare_spread: 0.6,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.1,
            leader_taper: 0.15,
            leader_has_branches: true,
            branch_start: 0.35,
            branch_end: 0.85,
            branch_density: 2.0, // Updated: was 1.2, research suggests 2.5
            branch_length: 0.65, // Updated: was 0.5, research suggests 0.8
            branch_angle: 55.0,
            branch_radius_ratio: 0.35,
            branch_taper: 0.7,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.25,
            up_attraction: 0.15,
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.2,
            sub_branch_position_bias: 0.2,
            apical_dominance: 0.3,
            branch_flatness: 0.3,
            branch_angle_curve: 0.0,
            crown_angle_variation: -0.2, // Lower branches spread horizontally
            trunk_twist: 15.0,
            branch_twist: 10.0,
            gravity_strength: 0.3, // Updated: was 0.1, research suggests 0.35
            stiffness: 0.6,
            break_chance: 0.05,
            split_enabled: true,
            split_probability: 0.3,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.9,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.365, 0.227, 0.102), // #5D3A1A Dark Brown
            foliage_color: Color::from_rgb(0.176, 0.314, 0.086), // #2D5016 Dark Green
            foliage: Some(FoliagePresetValues::oak()),
            growth: Some(GrowthPresetValues::spreading()), // Oak: co-dominant branches, moderate droop
        }
    }

    /// Pine: Christmas tree shape, conical crown, drooping branches
    pub fn pine() -> Self {
        Self {
            trunk_height: 8.0,
            trunk_radius: 0.4,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.1,
            trunk_taper_curve: 0.7,
            trunk_flare: 1.1,
            trunk_randomness: 0.0,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.2,
            leader_taper: 0.05,
            leader_has_branches: false,
            branch_start: 0.25, // Updated: was 0.15, research suggests 0.35-0.5
            branch_end: 0.95,
            branch_density: 1.8,
            branch_length: 0.45,
            branch_angle: 70.0,
            branch_radius_ratio: 0.25,
            branch_taper: 0.8,
            phyllotaxis_angle: 72.0, // Updated: was 137.5, whorled branching pattern
            branch_randomness: 0.15,
            up_attraction: -0.1,
            branch_recursion: 1,
            sub_branch_count: 2,
            sub_branch_scale: 0.4,
            branch_length_variation: 0.1,
            sub_branch_position_bias: 0.0,
            apical_dominance: 0.8,
            branch_flatness: 0.0,
            branch_angle_curve: 0.3,
            crown_angle_variation: 0.3, // Lower branches steeper, upper droopy
            trunk_twist: 0.0,
            branch_twist: 5.0,
            gravity_strength: 0.05,
            stiffness: 0.8,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Conical,
            crown_influence: 1.0,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.420, 0.267, 0.137), // #6B4423 Reddish Brown
            foliage_color: Color::from_rgb(0.106, 0.302, 0.243), // #1B4D3E Pine Green
            foliage: Some(FoliagePresetValues::pine()),
            growth: Some(GrowthPresetValues::structured()), // Pine: strong central leader, rigid
        }
    }

    /// Willow: Long drooping branches, hemispherical crown
    pub fn willow() -> Self {
        Self {
            trunk_height: 5.0,
            trunk_radius: 0.5,
            radial_segments: 8,
            height_segments: 4,
            trunk_taper: 0.4,
            trunk_taper_curve: 0.4,
            trunk_flare: 1.0,
            trunk_randomness: 0.1,
            root_flare_count: 4,
            root_flare_spread: 0.4,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.15,
            leader_taper: 0.1,
            leader_has_branches: false,
            branch_start: 0.4,
            branch_end: 0.9,
            branch_density: 2.0, // Updated: was 1.5, willows have dense cascading branches
            branch_length: 0.7,
            branch_angle: 50.0,
            branch_radius_ratio: 0.2,
            branch_taper: 0.85,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.3,
            up_attraction: -0.75, // Updated: was -0.5, research suggests -0.8
            branch_recursion: 3,  // Updated: was 2, research suggests 4
            sub_branch_count: 3,
            sub_branch_scale: 0.7, // Updated: was 0.6, research suggests 0.75
            branch_length_variation: 0.25,
            sub_branch_position_bias: 0.3,
            apical_dominance: 0.2,
            branch_flatness: 0.2,
            branch_angle_curve: -0.4,
            crown_angle_variation: -0.3, // Strong horizontal spread at bottom
            trunk_twist: 10.0,
            branch_twist: 15.0,
            gravity_strength: 0.5, // Reduced from 0.7 - real willows rely on flexibility (stiffness 0.2), not extreme gravity
            stiffness: 0.2,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.05,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Hemispherical,
            crown_influence: 0.8,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.478, 0.361, 0.239), // #7A5C3D Grayish Brown
            foliage_color: Color::from_rgb(0.565, 0.690, 0.376), // #90B060 Yellow-Green
            foliage: Some(FoliagePresetValues::willow()),
            growth: Some(GrowthPresetValues::weeping()), // Willow: drooping branches, high flexibility
        }
    }

    /// Birch: Slender trunk, delicate upward-reaching branches
    pub fn birch() -> Self {
        Self {
            trunk_height: 7.0,
            trunk_radius: 0.3,
            radial_segments: 6,
            height_segments: 5,
            trunk_taper: 0.2,
            trunk_taper_curve: 0.5,
            trunk_flare: 1.0,
            trunk_randomness: 0.15,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.15,
            leader_taper: 0.1,
            leader_has_branches: false,
            branch_start: 0.4,
            branch_end: 0.95,
            branch_density: 1.5, // Updated: was 1.0, birches have many fine branches
            branch_length: 0.35,
            branch_angle: 40.0,
            branch_radius_ratio: 0.2,
            branch_taper: 0.75,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.2,
            up_attraction: 0.3,
            branch_recursion: 1,
            sub_branch_count: 2,
            sub_branch_scale: 0.5,
            branch_length_variation: 0.15,
            sub_branch_position_bias: 0.1,
            apical_dominance: 0.55, // Updated: was 0.6, research suggests 0.55
            branch_flatness: 0.1,
            branch_angle_curve: 0.2,
            crown_angle_variation: 0.1, // Slight vertical bias at bottom
            trunk_twist: 5.0,
            branch_twist: 8.0,
            gravity_strength: 0.1, // Updated: birch branches reach upward, minimal droop
            stiffness: 0.7,
            break_chance: 0.1,
            split_enabled: true,
            split_probability: 0.2,
            split_angle: 25.0,
            split_position: 0.5,
            split_radius_threshold: 0.05,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::TaperedCylindrical,
            crown_influence: 0.7,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.961, 0.961, 0.863), // #F5F5DC Beige/White
            foliage_color: Color::from_rgb(0.486, 0.804, 0.486), // #7CCD7C Light Green
            foliage: Some(FoliagePresetValues::birch()),
            growth: Some(GrowthPresetValues::structured()), // Birch: upright, delicate
        }
    }

    /// Palm: Branches only at top, no sub-branches
    pub fn palm() -> Self {
        Self {
            trunk_height: 6.0,
            trunk_radius: 0.35,
            radial_segments: 8,
            height_segments: 6,
            trunk_taper: 0.35, // Reduced from 0.5 - less thick at tip for crown junction
            trunk_taper_curve: 0.3,
            trunk_flare: 1.3,
            trunk_randomness: 0.0,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.15,
            leader_taper: 0.1,
            leader_has_branches: false,
            branch_start: 0.85,
            branch_end: 0.98,
            branch_density: 2.5,
            branch_length: 0.6,
            branch_angle: 60.0,
            branch_radius_ratio: 0.28, // Increased from 0.15 - palm fronds have mass
            branch_taper: 0.9,
            phyllotaxis_angle: 45.0,
            branch_randomness: 0.1,
            up_attraction: -0.2,
            branch_recursion: 0,
            sub_branch_count: 0,
            sub_branch_scale: 0.5,
            branch_length_variation: 0.1,
            sub_branch_position_bias: 0.0,
            apical_dominance: 0.9,
            branch_flatness: 0.0,
            branch_angle_curve: 0.0,
            crown_angle_variation: 0.0, // All branches same angle
            trunk_twist: 0.0,
            branch_twist: 0.0,
            gravity_strength: 0.15,
            stiffness: 0.5,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Cylindrical,
            crown_influence: 0.5,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.545, 0.451, 0.333), // #8B7355 Tan Brown
            foliage_color: Color::from_rgb(0.133, 0.545, 0.133), // #228B22 Forest Green
            foliage: Some(FoliagePresetValues::palm()),
            growth: None, // Palm: minimal branching (only crown)
        }
    }

    /// Cypress: Tall narrow shape, flame crown, strong upward growth
    pub fn cypress() -> Self {
        Self {
            trunk_height: 10.0,
            trunk_radius: 0.4,
            radial_segments: 8,
            height_segments: 6,
            trunk_taper: 0.15,
            trunk_taper_curve: 0.6,
            trunk_flare: 1.0,
            trunk_randomness: 0.0,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::PointedTip,
            leader_length: 0.15,
            leader_taper: 0.1,
            leader_has_branches: false,
            branch_start: 0.1,
            branch_end: 0.95,
            branch_density: 3.0, // Updated: was 2.0, research suggests 3.5-4.5
            branch_length: 0.25,
            branch_angle: 18.0, // Updated: was 25.0, research suggests 12-15
            branch_radius_ratio: 0.28, // Increased from 0.2 - conifer needs structural branches
            branch_taper: 0.7,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.1,
            up_attraction: 0.85, // Updated: was 0.6, research suggests 0.95
            branch_recursion: 1,
            sub_branch_count: 2,
            sub_branch_scale: 0.4,
            branch_length_variation: 0.05,
            sub_branch_position_bias: -0.2,
            apical_dominance: 0.9,
            branch_flatness: 0.0,
            branch_angle_curve: 0.5,
            crown_angle_variation: 0.4, // Strong vertical bias at bottom
            trunk_twist: 5.0,
            branch_twist: 3.0,
            gravity_strength: 0.0,
            stiffness: 0.9,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Flame,
            crown_influence: 1.0,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.290, 0.235, 0.165), // #4A3C2A Dark Olive
            foliage_color: Color::from_rgb(0.208, 0.369, 0.231), // #355E3B Hunter Green
            foliage: Some(FoliagePresetValues::cypress()),
            growth: Some(GrowthPresetValues::structured()), // Cypress: strong upward growth
        }
    }

    /// Bonsai: Small, artistic, high randomness
    pub fn bonsai() -> Self {
        Self {
            trunk_height: 2.0,
            trunk_radius: 0.25,
            radial_segments: 8,
            height_segments: 3,
            trunk_taper: 0.25,
            trunk_taper_curve: 0.4,
            trunk_flare: 1.4,
            trunk_randomness: 0.25,
            root_flare_count: 3,
            root_flare_spread: 0.8,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.15,
            leader_taper: 0.1,
            leader_has_branches: false,
            branch_start: 0.3,
            branch_end: 0.8,
            branch_density: 0.8,
            branch_length: 0.5,
            branch_angle: 60.0,
            branch_radius_ratio: 0.3, // Reduced from 0.4 - bonsai should show delicate branching
            branch_taper: 0.6,
            phyllotaxis_angle: 90.0,
            branch_randomness: 0.5,
            up_attraction: 0.0,
            branch_recursion: 2,
            sub_branch_count: 2,
            sub_branch_scale: 0.6,
            branch_length_variation: 0.3,
            sub_branch_position_bias: 0.4,
            apical_dominance: 0.4,
            branch_flatness: 0.4,
            branch_angle_curve: -0.2,
            crown_angle_variation: -0.1, // Slight horizontal spread
            trunk_twist: 30.0,
            branch_twist: 20.0,
            gravity_strength: 0.15,
            stiffness: 0.4,
            break_chance: 0.1,
            split_enabled: true,
            split_probability: 0.4,
            split_angle: 35.0,
            split_position: 0.5,
            split_radius_threshold: 0.06,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.6,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.361, 0.251, 0.200), // #5C4033 Dark Brown
            foliage_color: Color::from_rgb(0.208, 0.369, 0.231), // #355E3B Hunter Green
            foliage: Some(FoliagePresetValues::bonsai()),
            growth: Some(GrowthPresetValues::gnarled()), // Bonsai: artistic, chaotic form
        }
    }

    /// Maple: Opposite branching pattern, spherical crown
    pub fn maple() -> Self {
        Self {
            trunk_height: 8.0,
            trunk_radius: 0.5,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.35,
            trunk_taper_curve: 0.5,
            trunk_flare: 1.15,
            trunk_randomness: 0.05,
            root_flare_count: 4,
            root_flare_spread: 0.5,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.1,
            leader_taper: 0.12,
            leader_has_branches: true,
            branch_start: 0.35,
            branch_end: 0.9,
            branch_density: 1.8,
            branch_length: 0.55,
            branch_angle: 52.0,
            branch_radius_ratio: 0.32,
            branch_taper: 0.7,
            phyllotaxis_angle: 180.0, // Key: opposite branching pairs
            branch_randomness: 0.2,
            up_attraction: 0.1,
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.2,
            sub_branch_position_bias: 0.15,
            apical_dominance: 0.45, // Low - co-dominant branching
            branch_flatness: 0.25,
            branch_angle_curve: 0.0,
            crown_angle_variation: -0.15,
            trunk_twist: 10.0,
            branch_twist: 8.0,
            gravity_strength: 0.25,
            stiffness: 0.55,
            break_chance: 0.05,
            split_enabled: true,
            split_probability: 0.25,
            split_angle: 28.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.85,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.32, 0.21, 0.12), // Dark brown
            foliage_color: Color::from_rgb(0.133, 0.322, 0.133), // Forest green
            foliage: Some(FoliagePresetValues::maple()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Spruce: Perfect conical shape, whorled branches near horizontal
    pub fn spruce() -> Self {
        Self {
            trunk_height: 10.0,
            trunk_radius: 0.35,
            radial_segments: 8,
            height_segments: 6,
            trunk_taper: 0.12, // Increased from 0.08 - better leader branch visual connection
            trunk_taper_curve: 0.8,
            trunk_flare: 1.05,
            trunk_randomness: 0.0,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.25,
            leader_taper: 0.03,
            leader_has_branches: false,
            branch_start: 0.12, // Nearly to ground
            branch_end: 0.95,
            branch_density: 2.2,
            branch_length: 0.5,
            branch_angle: 78.0, // Near horizontal
            branch_radius_ratio: 0.22,
            branch_taper: 0.82,
            phyllotaxis_angle: 72.0, // Whorled branching
            branch_randomness: 0.1,
            up_attraction: -0.15, // Slight droop
            branch_recursion: 1,
            sub_branch_count: 2,
            sub_branch_scale: 0.35,
            branch_length_variation: 0.08,
            sub_branch_position_bias: 0.0,
            apical_dominance: 0.88, // Strong central leader
            branch_flatness: 0.0,
            branch_angle_curve: 0.35,
            crown_angle_variation: 0.35,
            trunk_twist: 0.0,
            branch_twist: 3.0,
            gravity_strength: 0.08, // Reduced from 0.2 - spruces hold branches stiffly, similar to Pine
            stiffness: 0.75,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Conical,
            crown_influence: 1.0,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.380, 0.243, 0.141), // Reddish-brown bark
            foliage_color: Color::from_rgb(0.086, 0.278, 0.212), // Blue-green
            foliage: Some(FoliagePresetValues::spruce()),
            growth: Some(GrowthPresetValues::structured()),
        }
    }

    /// Poplar: Columnar growth, extreme vertical tendency
    pub fn poplar() -> Self {
        Self {
            trunk_height: 12.0,
            trunk_radius: 0.35,
            radial_segments: 8,
            height_segments: 7,
            trunk_taper: 0.12,
            trunk_taper_curve: 0.7,
            trunk_flare: 1.0,
            trunk_randomness: 0.0,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.2,
            leader_taper: 0.05,
            leader_has_branches: false,
            branch_start: 0.2,
            branch_end: 0.95,
            branch_density: 1.6,
            branch_length: 0.3,
            branch_angle: 15.0, // Nearly vertical
            branch_radius_ratio: 0.2,
            branch_taper: 0.75,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.05,
            up_attraction: 0.92, // Extreme upward tendency
            branch_recursion: 1,
            sub_branch_count: 2,
            sub_branch_scale: 0.4,
            branch_length_variation: 0.1,
            sub_branch_position_bias: -0.1,
            apical_dominance: 0.92, // Very strong central leader
            branch_flatness: 0.0,
            branch_angle_curve: 0.4,
            crown_angle_variation: 0.3,
            trunk_twist: 0.0,
            branch_twist: 2.0,
            gravity_strength: 0.0,
            stiffness: 0.9,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 20.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Flame,
            crown_influence: 1.0,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.553, 0.514, 0.435), // Gray-brown bark
            foliage_color: Color::from_rgb(0.404, 0.545, 0.306), // Yellow-green
            foliage: Some(FoliagePresetValues::poplar()),
            growth: Some(GrowthPresetValues::columnar()),
        }
    }

    /// Baobab: Massive bottle trunk, branches only at crown
    pub fn baobab() -> Self {
        Self {
            trunk_height: 6.0,
            trunk_radius: 1.5, // Massive trunk
            radial_segments: 12,
            height_segments: 4,
            trunk_taper: 0.5,
            trunk_taper_curve: 0.3,
            trunk_flare: 1.6, // Large root flare
            trunk_randomness: 0.08,
            root_flare_count: 6,
            root_flare_spread: 0.7,
            root_flare_height: 0.2,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.05,
            leader_taper: 0.2,
            leader_has_branches: false,
            branch_start: 0.75, // Branches only at crown
            branch_end: 0.95,
            branch_density: 0.8, // Sparse
            branch_length: 0.45,
            branch_angle: 55.0,
            branch_radius_ratio: 0.35,
            branch_taper: 0.6,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.35,
            up_attraction: 0.2,
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.5,
            branch_length_variation: 0.25,
            sub_branch_position_bias: 0.3,
            apical_dominance: 0.25,
            branch_flatness: 0.3,
            branch_angle_curve: -0.2,
            crown_angle_variation: -0.2,
            trunk_twist: 5.0,
            branch_twist: 12.0,
            gravity_strength: 0.15,
            stiffness: 0.5,
            break_chance: 0.1,
            split_enabled: true,
            split_probability: 0.3,
            split_angle: 35.0,
            split_position: 0.6,
            split_radius_threshold: 0.15,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.8,
            crown_shape: CrownShape::Hemispherical,
            crown_influence: 0.7,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.506, 0.424, 0.369), // Gray-tan bark
            foliage_color: Color::from_rgb(0.306, 0.463, 0.235), // Olive green
            foliage: Some(FoliagePresetValues::baobab()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Dragon Tree: Dichotomous (binary) forking pattern
    pub fn dragon_tree() -> Self {
        Self {
            trunk_height: 5.0,
            trunk_radius: 0.4,
            radial_segments: 8,
            height_segments: 4,
            trunk_taper: 0.3,
            trunk_taper_curve: 0.5,
            trunk_flare: 1.1,
            trunk_randomness: 0.05,
            root_flare_count: 3,
            root_flare_spread: 0.4,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.05,
            leader_taper: 0.15,
            leader_has_branches: false,
            branch_start: 0.5,
            branch_end: 0.9,
            branch_density: 1.0,
            branch_length: 0.4,
            branch_angle: 45.0,
            branch_radius_ratio: 0.3, // Reduced from 0.45 - binary forking should be thinner
            branch_taper: 0.5,
            phyllotaxis_angle: 180.0, // Opposite pairs for binary look
            branch_randomness: 0.15,
            up_attraction: 0.25,
            branch_recursion: 3,
            sub_branch_count: 2,   // Binary only
            sub_branch_scale: 0.7, // Each fork similar size
            branch_length_variation: 0.15,
            sub_branch_position_bias: 0.5,
            apical_dominance: 0.35, // Low - equal branching
            branch_flatness: 0.15,
            branch_angle_curve: 0.1,
            crown_angle_variation: 0.0,
            trunk_twist: 8.0,
            branch_twist: 5.0,
            gravity_strength: 0.1,
            stiffness: 0.65,
            break_chance: 0.0,
            split_enabled: true,
            split_probability: 1.0, // Always forks
            split_angle: 40.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.6,
            crown_shape: CrownShape::Hemispherical,
            crown_influence: 0.6,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.463, 0.396, 0.322), // Gray-brown
            foliage_color: Color::from_rgb(0.286, 0.443, 0.255), // Blue-green
            foliage: Some(FoliagePresetValues::dragon_tree()),
            growth: Some(GrowthPresetValues::forking()),
        }
    }

    /// Japanese Maple: Ornamental, layered horizontal branches
    pub fn japanese_maple() -> Self {
        Self {
            trunk_height: 3.5, // Small ornamental
            trunk_radius: 0.15,
            radial_segments: 6,
            height_segments: 3,
            trunk_taper: 0.25,
            trunk_taper_curve: 0.4,
            trunk_flare: 1.1,
            trunk_randomness: 0.15,
            root_flare_count: 3,
            root_flare_spread: 0.5,
            root_flare_height: 0.1,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.08,
            leader_taper: 0.1,
            leader_has_branches: false,
            branch_start: 0.25,
            branch_end: 0.85,
            branch_density: 1.4,
            branch_length: 0.45,
            branch_angle: 65.0, // Wide spreading
            branch_radius_ratio: 0.25,
            branch_taper: 0.7,
            phyllotaxis_angle: 180.0, // Opposite branching (maple family)
            branch_randomness: 0.25,
            up_attraction: -0.15, // Slight droop
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.2,
            sub_branch_position_bias: 0.2,
            apical_dominance: 0.35,
            branch_flatness: 0.65, // Layered horizontal branches
            branch_angle_curve: -0.3,
            crown_angle_variation: -0.25,
            trunk_twist: 15.0,
            branch_twist: 12.0,
            gravity_strength: 0.2,
            stiffness: 0.45,
            break_chance: 0.05,
            split_enabled: true,
            split_probability: 0.2,
            split_angle: 30.0,
            split_position: 0.4,
            split_radius_threshold: 0.05,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.4,
            crown_shape: CrownShape::Spreading,
            crown_influence: 0.8,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.376, 0.282, 0.212), // Red-brown bark
            foliage_color: Color::from_rgb(0.698, 0.133, 0.133), // Red foliage
            foliage: Some(FoliagePresetValues::japanese_maple()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Dead Tree: Fantasy/horror style, no foliage, broken branches
    pub fn dead_tree() -> Self {
        Self {
            trunk_height: 5.0,
            trunk_radius: 0.4,
            radial_segments: 8,
            height_segments: 4,
            trunk_taper: 0.35,
            trunk_taper_curve: 0.5,
            trunk_flare: 1.2,
            trunk_randomness: 0.2,
            root_flare_count: 4,
            root_flare_spread: 0.6,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::FlatCap, // Broken top
            leader_length: 0.05,
            leader_taper: 0.2,
            leader_has_branches: false,
            branch_start: 0.2,
            branch_end: 0.7, // Branches end early (broken top)
            branch_density: 0.8,
            branch_length: 0.4,
            branch_angle: 50.0,
            branch_radius_ratio: 0.35,
            branch_taper: 0.6,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.45, // Very irregular
            up_attraction: 0.0,
            branch_recursion: 2,
            sub_branch_count: 2,
            sub_branch_scale: 0.5,
            branch_length_variation: 0.35,
            sub_branch_position_bias: 0.3,
            apical_dominance: 0.3,
            branch_flatness: 0.2,
            branch_angle_curve: 0.0,
            crown_angle_variation: 0.0,
            trunk_twist: 40.0, // Twisted trunk
            branch_twist: 25.0,
            gravity_strength: 0.4, // Increased from 0.25 - skeletal hanging appearance
            stiffness: 0.35,
            break_chance: 0.35, // Many broken branches
            split_enabled: true,
            split_probability: 0.25,
            split_angle: 40.0,
            split_position: 0.4,
            split_radius_threshold: 0.1,
            floor_avoidance: false, // Branches can touch ground
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.6,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.4,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.388, 0.361, 0.325), // Gray-brown (weathered)
            foliage_color: Color::from_rgb(0.3, 0.3, 0.3),     // Unused but set to gray
            foliage: None,                                     // No foliage
            growth: Some(GrowthPresetValues::gnarled()),
        }
    }

    /// Redwood: Massive scale, columnar trunk, minimal taper
    pub fn redwood() -> Self {
        Self {
            trunk_height: 20.0, // Very tall
            trunk_radius: 1.0,  // Massive trunk
            radial_segments: 12,
            height_segments: 10,
            trunk_taper: 0.05, // Reduced from 0.1 - more columnar for redwood
            trunk_taper_curve: 0.8,
            trunk_flare: 1.3,
            trunk_randomness: 0.02,
            root_flare_count: 6,
            root_flare_spread: 0.5,
            root_flare_height: 0.2,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.15,
            leader_taper: 0.05,
            leader_has_branches: false,
            branch_start: 0.5, // Self-prunes lower branches
            branch_end: 0.95,
            branch_density: 1.2,
            branch_length: 0.35,
            branch_angle: 75.0,        // Near horizontal
            branch_radius_ratio: 0.25, // Increased from 0.18 - less wire-like on massive tree
            branch_taper: 0.8,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.1,
            up_attraction: -0.1,
            branch_recursion: 1,
            sub_branch_count: 2,
            sub_branch_scale: 0.4,
            branch_length_variation: 0.1,
            sub_branch_position_bias: 0.0,
            apical_dominance: 0.85,
            branch_flatness: 0.0,
            branch_angle_curve: 0.25,
            crown_angle_variation: 0.2,
            trunk_twist: 0.0,
            branch_twist: 3.0,
            gravity_strength: 0.15,
            stiffness: 0.85,
            break_chance: 0.02,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Conical,
            crown_influence: 0.9,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.463, 0.263, 0.161), // Red-brown bark
            foliage_color: Color::from_rgb(0.12, 0.32, 0.21),  // Dark green
            foliage: Some(FoliagePresetValues::redwood()),
            growth: Some(GrowthPresetValues::structured()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_returns_none() {
        assert!(TreePreset::Custom.get_values().is_none());
    }

    #[test]
    fn test_presets_return_values() {
        assert!(TreePreset::Oak.get_values().is_some());
        assert!(TreePreset::Pine.get_values().is_some());
        assert!(TreePreset::Willow.get_values().is_some());
        assert!(TreePreset::Birch.get_values().is_some());
        assert!(TreePreset::Palm.get_values().is_some());
        assert!(TreePreset::Cypress.get_values().is_some());
        assert!(TreePreset::Bonsai.get_values().is_some());
        // New presets
        assert!(TreePreset::Maple.get_values().is_some());
        assert!(TreePreset::Spruce.get_values().is_some());
        assert!(TreePreset::Poplar.get_values().is_some());
        assert!(TreePreset::Baobab.get_values().is_some());
        assert!(TreePreset::DragonTree.get_values().is_some());
        assert!(TreePreset::JapaneseMaple.get_values().is_some());
        assert!(TreePreset::DeadTree.get_values().is_some());
        assert!(TreePreset::Redwood.get_values().is_some());
    }

    #[test]
    fn test_oak_values() {
        let oak = TreePresetValues::oak();
        assert_eq!(oak.crown_shape, CrownShape::Spherical);
        assert!(oak.branch_angle > 45.0); // Wide spreading
    }

    #[test]
    fn test_pine_values() {
        let pine = TreePresetValues::pine();
        assert_eq!(pine.crown_shape, CrownShape::Conical);
        assert!(pine.up_attraction < 0.0); // Drooping
    }

    #[test]
    fn test_willow_values() {
        let willow = TreePresetValues::willow();
        assert_eq!(willow.crown_shape, CrownShape::Hemispherical);
        assert!(willow.up_attraction < -0.3); // Strong droop
    }

    #[test]
    fn test_palm_values() {
        let palm = TreePresetValues::palm();
        assert!(palm.branch_start > 0.8); // Branches only at top
        assert_eq!(palm.branch_recursion, 0); // No sub-branches
    }

    #[test]
    fn test_cypress_values() {
        let cypress = TreePresetValues::cypress();
        assert_eq!(cypress.crown_shape, CrownShape::Flame);
        assert!(cypress.up_attraction > 0.5); // Strong upward
        assert!(cypress.branch_angle < 30.0); // Narrow
    }

    #[test]
    fn test_maple_opposite_branching() {
        let maple = TreePresetValues::maple();
        assert_eq!(maple.phyllotaxis_angle, 180.0); // Opposite branching
        assert_eq!(maple.crown_shape, CrownShape::Spherical);
    }

    #[test]
    fn test_spruce_whorled_branching() {
        let spruce = TreePresetValues::spruce();
        assert_eq!(spruce.phyllotaxis_angle, 72.0); // Whorled
        assert!(spruce.branch_angle > 70.0); // Near horizontal
        assert!(spruce.apical_dominance > 0.85);
    }

    #[test]
    fn test_poplar_columnar() {
        let poplar = TreePresetValues::poplar();
        assert!(poplar.up_attraction > 0.9); // Extreme upward
        assert!(poplar.branch_angle < 20.0); // Nearly vertical
        assert!(poplar.apical_dominance > 0.9);
    }

    #[test]
    fn test_baobab_bottle_trunk() {
        let baobab = TreePresetValues::baobab();
        assert!(baobab.trunk_radius > 1.0); // Massive trunk
        assert!(baobab.branch_start > 0.7); // Crown only
        assert!(baobab.branch_density < 1.0); // Sparse
    }

    #[test]
    fn test_dragon_tree_forking() {
        let dragon = TreePresetValues::dragon_tree();
        assert_eq!(dragon.split_probability, 1.0); // Always forks
        assert_eq!(dragon.sub_branch_count, 2); // Binary
    }

    #[test]
    fn test_japanese_maple_ornamental() {
        let jmaple = TreePresetValues::japanese_maple();
        assert!(jmaple.branch_flatness > 0.5); // Layered
        assert_eq!(jmaple.phyllotaxis_angle, 180.0); // Opposite (maple family)
        assert_eq!(jmaple.crown_shape, CrownShape::Spreading);
    }

    #[test]
    fn test_dead_tree_no_foliage() {
        let dead = TreePresetValues::dead_tree();
        assert!(dead.foliage.is_none()); // No foliage
        assert!(dead.break_chance > 0.3); // Many broken branches
        assert!(dead.trunk_twist > 30.0); // Twisted
    }

    #[test]
    fn test_redwood_massive() {
        let redwood = TreePresetValues::redwood();
        assert!(redwood.trunk_height >= 20.0); // Very tall
        assert!(redwood.trunk_radius >= 1.0); // Massive trunk
        assert!(redwood.trunk_taper < 0.15); // Minimal taper (columnar)
    }

    #[test]
    fn test_growth_presets_return_values() {
        assert!(GrowthPreset::Custom.get_values().is_none());
        assert!(GrowthPreset::Structured.get_values().is_some());
        assert!(GrowthPreset::Spreading.get_values().is_some());
        assert!(GrowthPreset::Weeping.get_values().is_some());
        assert!(GrowthPreset::Gnarled.get_values().is_some());
        assert!(GrowthPreset::Columnar.get_values().is_some());
        assert!(GrowthPreset::Forking.get_values().is_some());
    }

    #[test]
    fn test_columnar_growth_preset() {
        let columnar = GrowthPresetValues::columnar();
        assert!(columnar.apical_dominance > 0.9);
        assert!(columnar.lateral_angle < 30.0);
        assert!(columnar.stiffness > 0.9);
    }

    #[test]
    fn test_forking_growth_preset() {
        let forking = GrowthPresetValues::forking();
        assert!(forking.split_threshold < 0.4); // Frequent forking
        assert!(forking.apical_dominance < 0.5); // Low dominance
    }
}
