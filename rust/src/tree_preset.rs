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
    Elm = 16,           // Vase-shaped crown
    Fir = 17,           // Symmetric pyramid
    Cedar = 18,         // Flat sprays, tiered
    JoshuaTree = 19,    // Chaotic dichotomous
    Olive = 20,         // Gnarled twisted trunk
    // Phase 2: New species
    CherryBlossom = 21, // Sakura - graceful ornamental
    Acacia = 22,        // Umbrella tree - flat-topped African
    Beech = 23,         // Dense spherical crown
    Ginkgo = 24,        // Fan-shaped leaves, angular branches
    WeepingCherry = 25, // Cascade/waterfall effect
    // Phase 3: Fantasy presets
    WorldTree = 26,     // Yggdrasil-scale massive landmark
    CrystalTree = 27,   // Faceted geometric crystal formations
    CorruptedTree = 28, // Twisted, diseased dark appearance
    GlowingTree = 29,   // Magical tree with glowing foliage
    // Phase 4: New species from research
    BlueSpruce = 30,      // Blue-gray foliage, compact
    DouglasFir = 31,      // Tiered branches, flat sprays
    PonderosaPine = 32,   // Cinnamon bark, needle clusters
    BristleconePine = 33, // Ancient gnarled, twisted
    Ash = 34,             // Opposite branching, open crown
    Linden = 35,          // Dense heart leaves, conical-spherical
    Sycamore = 36,        // Buttressed trunk, irregular crown
    Aspen = 37,           // Slender, trembling leaves
    RoyalPalm = 38,       // Perfectly straight, gray-white trunk
    FanPalm = 39,         // Palmate fronds, petticoat skirt
    Eucalyptus = 40,      // Tall, pendulous, hanging leaves
}

/// Style modifiers for different game art styles
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum StyleModifier {
    #[default]
    None = 0,
    /// Thicker trunk, smoother surfaces, simplified silhouette
    StylizedCartoon = 1,
    /// Reduced mesh segments for performance (desktop low-poly)
    LowPoly = 2,
    /// Twisted, broken branches, spooky appearance
    Horror = 3,
    /// Higher mesh detail, more natural variation
    Realistic = 4,
    /// Extreme optimization for mobile (300-1500 triangles target)
    MobileOptimized = 5,
    /// AAA-quality realistic detail (high poly)
    AAARealistic = 6,
}

/// Scale modifiers for tree age/size variants
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum ScaleModifier {
    #[default]
    Mature = 0,
    /// Young tree: small, sparse branches
    Sapling = 1,
    /// Old tree: large, thick, weathered
    Ancient = 2,
    /// Dead tree: no foliage, broken branches
    Dead = 3,
}

/// Seasonal appearance modifiers
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum SeasonModifier {
    #[default]
    None = 0,
    /// 65% foliage, light green, small leaves
    Spring = 1,
    /// 100% foliage (baseline)
    Summer = 2,
    /// Gradient colors (orange/red/yellow), 70% density
    Autumn = 3,
    /// No foliage for deciduous, snow on evergreen
    Winter = 4,
}

/// Bonsai style modifiers for artistic tree shaping
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum BonsaiStyle {
    #[default]
    None = 0,
    /// Formal Upright (Chokkan): Straight trunk, triangular silhouette
    Chokkan = 1,
    /// Informal Upright (Moyogi): S-curved trunk, natural asymmetry
    Moyogi = 2,
    /// Slanting (Shakan): 60-80° trunk angle, opposite root emphasis
    Shakan = 3,
    /// Cascade (Kengai): Trunk extends below base, waterfall effect
    Kengai = 4,
    /// Windswept (Fukinagashi): Branches biased to one side, wind-shaped
    Fukinagashi = 5,
    /// Literati (Bunjingi): Tall thin trunk, sparse top branches only
    Bunjingi = 6,
}

/// Values applied by bonsai style modifiers
#[derive(Clone, Debug)]
pub struct BonsaiStyleValues {
    /// Trunk twist override (degrees)
    pub trunk_twist: f32,
    /// Branch randomness override
    pub branch_randomness: f32,
    /// Up attraction override
    pub up_attraction: f32,
    /// Gravity strength override
    pub gravity_strength: f32,
    /// Branch start override (0.0-1.0)
    pub branch_start: f32,
    /// Branch density multiplier
    pub branch_density_mult: f32,
    /// Trunk taper override
    pub trunk_taper: f32,
    /// Branch direction bias (-1.0 to 1.0, 0 = neutral)
    pub branch_direction_bias: f32,
    /// Stiffness override
    pub stiffness: f32,
}

impl BonsaiStyle {
    /// Get the modifier values, or None for None style
    pub fn get_values(&self) -> Option<BonsaiStyleValues> {
        match self {
            BonsaiStyle::None => None,
            BonsaiStyle::Chokkan => Some(BonsaiStyleValues::chokkan()),
            BonsaiStyle::Moyogi => Some(BonsaiStyleValues::moyogi()),
            BonsaiStyle::Shakan => Some(BonsaiStyleValues::shakan()),
            BonsaiStyle::Kengai => Some(BonsaiStyleValues::kengai()),
            BonsaiStyle::Fukinagashi => Some(BonsaiStyleValues::fukinagashi()),
            BonsaiStyle::Bunjingi => Some(BonsaiStyleValues::bunjingi()),
        }
    }
}

impl BonsaiStyleValues {
    /// Chokkan (Formal Upright): Straight trunk, triangular, minimal twist
    pub fn chokkan() -> Self {
        Self {
            trunk_twist: 0.0,
            branch_randomness: 0.05,
            up_attraction: 0.85,
            gravity_strength: 0.0,
            branch_start: 0.3,
            branch_density_mult: 1.0,
            trunk_taper: 0.25,
            branch_direction_bias: 0.0,
            stiffness: 0.9,
        }
    }

    /// Moyogi (Informal Upright): S-curved trunk, natural asymmetry
    pub fn moyogi() -> Self {
        Self {
            trunk_twist: 35.0,
            branch_randomness: 0.2,
            up_attraction: 0.3,
            gravity_strength: 0.1,
            branch_start: 0.25,
            branch_density_mult: 1.0,
            trunk_taper: 0.3,
            branch_direction_bias: 0.0,
            stiffness: 0.6,
        }
    }

    /// Shakan (Slanting): Leaning trunk, opposite root bias
    pub fn shakan() -> Self {
        Self {
            trunk_twist: 15.0,
            branch_randomness: 0.15,
            up_attraction: 0.4,
            gravity_strength: 0.3, // Trunk leans
            branch_start: 0.3,
            branch_density_mult: 1.0,
            trunk_taper: 0.28,
            branch_direction_bias: 0.3, // Slight bias opposite lean
            stiffness: 0.5,
        }
    }

    /// Kengai (Cascade): Trunk cascades below base
    pub fn kengai() -> Self {
        Self {
            trunk_twist: 20.0,
            branch_randomness: 0.2,
            up_attraction: -1.0, // Strong downward
            gravity_strength: 1.0,
            branch_start: 0.2,
            branch_density_mult: 0.8,
            trunk_taper: 0.35,
            branch_direction_bias: 0.0,
            stiffness: 0.2, // Very flexible
        }
    }

    /// Fukinagashi (Windswept): All branches to one side
    pub fn fukinagashi() -> Self {
        Self {
            trunk_twist: 25.0,
            branch_randomness: 0.15,
            up_attraction: 0.2,
            gravity_strength: 0.2,
            branch_start: 0.3,
            branch_density_mult: 0.9,
            trunk_taper: 0.3,
            branch_direction_bias: 1.0, // Strong one-sided bias
            stiffness: 0.4,
        }
    }

    /// Bunjingi (Literati): Tall thin, sparse top branches
    pub fn bunjingi() -> Self {
        Self {
            trunk_twist: 40.0, // Artistic curves
            branch_randomness: 0.25,
            up_attraction: 0.6,
            gravity_strength: 0.05,
            branch_start: 0.8,        // Branches only at top
            branch_density_mult: 0.3, // Very sparse
            trunk_taper: 0.65,        // Strong taper
            branch_direction_bias: 0.0,
            stiffness: 0.7,
        }
    }
}

/// Values applied by style modifiers
#[derive(Clone, Debug)]
pub struct StyleModifierValues {
    pub trunk_radius_mult: f32,
    pub randomness_add: f32,
    pub twist_add: f32,
    pub radial_segments: Option<i32>,
    pub break_chance_add: f32,
    /// Multiplier for branch_recursion (0.5 = half the recursion levels)
    pub recursion_mult: f32,
    /// Multiplier for foliage density
    pub foliage_density_mult: f32,
    /// Smoothing iterations override (-1 = no change)
    pub smooth_iterations: i32,
}

impl StyleModifier {
    /// Get the modifier values, or None for None modifier
    pub fn get_values(&self) -> Option<StyleModifierValues> {
        match self {
            StyleModifier::None => None,
            StyleModifier::StylizedCartoon => Some(StyleModifierValues::stylized_cartoon()),
            StyleModifier::LowPoly => Some(StyleModifierValues::low_poly()),
            StyleModifier::Horror => Some(StyleModifierValues::horror()),
            StyleModifier::Realistic => Some(StyleModifierValues::realistic()),
            StyleModifier::MobileOptimized => Some(StyleModifierValues::mobile_optimized()),
            StyleModifier::AAARealistic => Some(StyleModifierValues::aaa_realistic()),
        }
    }
}

impl StyleModifierValues {
    /// Stylized Cartoon: Thicker trunk, smoother, fewer segments
    pub fn stylized_cartoon() -> Self {
        Self {
            trunk_radius_mult: 1.5, // Updated: more exaggerated
            randomness_add: -0.1,
            twist_add: 0.0,
            radial_segments: Some(6),
            break_chance_add: 0.0,
            recursion_mult: 1.0,
            foliage_density_mult: 1.0,
            smooth_iterations: -1, // No change
        }
    }

    /// Low Poly: Reduced mesh complexity (desktop low-poly aesthetic)
    pub fn low_poly() -> Self {
        Self {
            trunk_radius_mult: 1.0,
            randomness_add: 0.0,
            twist_add: 0.0,
            radial_segments: Some(5), // Slightly more than 4 for better silhouette
            break_chance_add: 0.0,
            recursion_mult: 0.75,      // Reduce recursion
            foliage_density_mult: 0.8, // Slightly less foliage
            smooth_iterations: 0,      // No smoothing
        }
    }

    /// Horror: Twisted, broken, spooky
    pub fn horror() -> Self {
        Self {
            trunk_radius_mult: 1.0,
            randomness_add: 0.25,
            twist_add: 45.0,        // Updated: more extreme twist
            radial_segments: None,  // Keep original
            break_chance_add: 0.35, // Updated: more broken branches
            recursion_mult: 1.0,
            foliage_density_mult: 0.5, // Sparse, dying foliage
            smooth_iterations: -1,
        }
    }

    /// Realistic: Higher detail
    pub fn realistic() -> Self {
        Self {
            trunk_radius_mult: 1.0,
            randomness_add: 0.05,
            twist_add: 0.0,
            radial_segments: Some(12),
            break_chance_add: 0.0,
            recursion_mult: 1.0,
            foliage_density_mult: 1.2, // Denser foliage
            smooth_iterations: 2,      // Some smoothing
        }
    }

    /// Mobile Optimized: Extreme optimization for mobile (300-1500 triangles)
    pub fn mobile_optimized() -> Self {
        Self {
            trunk_radius_mult: 1.0,
            randomness_add: -0.05, // Less randomness = fewer vertices
            twist_add: 0.0,
            radial_segments: Some(4), // Minimum segments
            break_chance_add: 0.0,
            recursion_mult: 0.33,      // Updated: 300-1500 tri target
            foliage_density_mult: 0.5, // Half foliage
            smooth_iterations: 0,      // No smoothing
        }
    }

    /// AAA Realistic: Maximum detail for high-end platforms
    pub fn aaa_realistic() -> Self {
        Self {
            trunk_radius_mult: 1.0,
            randomness_add: 0.08, // Natural variation
            twist_add: 0.0,
            radial_segments: Some(16), // High segment count
            break_chance_add: 0.02,    // Occasional realistic breakage
            recursion_mult: 1.25,      // More recursion
            foliage_density_mult: 1.5, // Dense canopy
            smooth_iterations: 3,      // Good smoothing
        }
    }
}

/// Values applied by scale modifiers
#[derive(Clone, Debug)]
pub struct ScaleModifierValues {
    pub height_mult: f32,
    pub radius_mult: f32,
    pub density_mult: f32,
    pub randomness_add: f32,
    pub break_chance_add: f32,
    pub foliage_enabled: bool,
}

impl ScaleModifier {
    /// Get the modifier values, or None for Mature (baseline)
    pub fn get_values(&self) -> Option<ScaleModifierValues> {
        match self {
            ScaleModifier::Mature => None,
            ScaleModifier::Sapling => Some(ScaleModifierValues::sapling()),
            ScaleModifier::Ancient => Some(ScaleModifierValues::ancient()),
            ScaleModifier::Dead => Some(ScaleModifierValues::dead()),
        }
    }
}

impl ScaleModifierValues {
    /// Sapling: Small, sparse
    pub fn sapling() -> Self {
        Self {
            height_mult: 0.2,
            radius_mult: 0.1,
            density_mult: 0.4,
            randomness_add: 0.0,
            break_chance_add: 0.0,
            foliage_enabled: true,
        }
    }

    /// Ancient: Large, weathered
    pub fn ancient() -> Self {
        Self {
            height_mult: 1.25,
            radius_mult: 1.8,
            density_mult: 1.0,
            randomness_add: 0.1,
            break_chance_add: 0.08,
            foliage_enabled: true,
        }
    }

    /// Dead: No foliage, broken
    pub fn dead() -> Self {
        Self {
            height_mult: 0.9,
            radius_mult: 1.0,
            density_mult: 0.6,
            randomness_add: 0.15,
            break_chance_add: 0.25,
            foliage_enabled: false,
        }
    }
}

/// Values applied by season modifiers
#[derive(Clone, Debug)]
pub struct SeasonModifierValues {
    /// Foliage density multiplier (1.0 = normal)
    pub foliage_density_mult: f32,
    /// Override foliage color (None = use preset color)
    pub foliage_color: Option<Color>,
    /// Leaf size multiplier
    pub leaf_scale_mult: f32,
    /// Enable snow on branches (for evergreen winter)
    pub snow_enabled: bool,
    /// Disable foliage entirely (deciduous winter)
    pub foliage_disabled: bool,
}

impl SeasonModifier {
    /// Get the modifier values, or None for None modifier
    pub fn get_values(&self) -> Option<SeasonModifierValues> {
        match self {
            SeasonModifier::None => None,
            SeasonModifier::Spring => Some(SeasonModifierValues::spring()),
            SeasonModifier::Summer => Some(SeasonModifierValues::summer()),
            SeasonModifier::Autumn => Some(SeasonModifierValues::autumn()),
            SeasonModifier::Winter => Some(SeasonModifierValues::winter()),
        }
    }
}

impl SeasonModifierValues {
    /// Spring: 65% foliage, light green, smaller leaves
    pub fn spring() -> Self {
        Self {
            foliage_density_mult: 0.65,
            foliage_color: Some(Color::from_rgb(0.5, 0.75, 0.35)), // Light spring green
            leaf_scale_mult: 0.75,                                 // Smaller new leaves
            snow_enabled: false,
            foliage_disabled: false,
        }
    }

    /// Summer: 100% foliage (baseline)
    pub fn summer() -> Self {
        Self {
            foliage_density_mult: 1.0,
            foliage_color: None, // Use preset color
            leaf_scale_mult: 1.0,
            snow_enabled: false,
            foliage_disabled: false,
        }
    }

    /// Autumn: 70% density, warm colors
    pub fn autumn() -> Self {
        Self {
            foliage_density_mult: 0.7,
            foliage_color: Some(Color::from_rgb(0.85, 0.5, 0.2)), // Warm orange
            leaf_scale_mult: 1.0,
            snow_enabled: false,
            foliage_disabled: false,
        }
    }

    /// Winter: No foliage for deciduous, snow on evergreen
    pub fn winter() -> Self {
        Self {
            foliage_density_mult: 0.0,
            foliage_color: None,
            leaf_scale_mult: 1.0,
            snow_enabled: true,
            foliage_disabled: true, // Deciduous trees have no foliage
        }
    }
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
    /// C27: Split angle for growth bifurcation (degrees)
    pub split_angle: f32,
    /// C27: Phyllotaxis angle for growth (degrees)
    pub phyllotaxis_angle: f32,
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
            split_angle: 45.0,
            phyllotaxis_angle: 137.5,
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
            split_angle: 60.0,
            phyllotaxis_angle: 137.5,
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
            split_angle: 50.0,
            phyllotaxis_angle: 137.5,
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
            split_angle: 70.0,
            phyllotaxis_angle: 137.5,
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
            split_angle: 30.0,
            phyllotaxis_angle: 137.5,
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
            split_angle: 40.0,
            phyllotaxis_angle: 137.5,
        }
    }

    /// Apply these preset values to a GrowthConfig
    /// C25 fix: branch_length is scaled consistently with manual path
    /// (manual: branch_length * trunk_height * 0.1, preset must do the same)
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
        // C25 fix: Apply same scaling as manual path
        config.branch_length = self.branch_length * config.trunk_height * 0.1;
        config.gravitropism = self.gravitropism;
        config.randomness = self.randomness;
        config.gravity_strength = self.gravity_strength;
        config.stiffness = self.stiffness;
        // C27 fix: Apply split_angle and phyllotaxis_angle from preset
        config.split_angle = self.split_angle;
        config.phyllotaxis_angle = self.phyllotaxis_angle;
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
    pub split_radius_multiplier: f32,

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
            TreePreset::Elm => Some(TreePresetValues::elm()),
            TreePreset::Fir => Some(TreePresetValues::fir()),
            TreePreset::Cedar => Some(TreePresetValues::cedar()),
            TreePreset::JoshuaTree => Some(TreePresetValues::joshua_tree()),
            TreePreset::Olive => Some(TreePresetValues::olive()),
            // Phase 2: New species
            TreePreset::CherryBlossom => Some(TreePresetValues::cherry_blossom()),
            TreePreset::Acacia => Some(TreePresetValues::acacia()),
            TreePreset::Beech => Some(TreePresetValues::beech()),
            TreePreset::Ginkgo => Some(TreePresetValues::ginkgo()),
            TreePreset::WeepingCherry => Some(TreePresetValues::weeping_cherry()),
            // Phase 3: Fantasy presets
            TreePreset::WorldTree => Some(TreePresetValues::world_tree()),
            TreePreset::CrystalTree => Some(TreePresetValues::crystal_tree()),
            TreePreset::CorruptedTree => Some(TreePresetValues::corrupted_tree()),
            TreePreset::GlowingTree => Some(TreePresetValues::glowing_tree()),
            // Phase 4: New species
            TreePreset::BlueSpruce => Some(TreePresetValues::blue_spruce()),
            TreePreset::DouglasFir => Some(TreePresetValues::douglas_fir()),
            TreePreset::PonderosaPine => Some(TreePresetValues::ponderosa_pine()),
            TreePreset::BristleconePine => Some(TreePresetValues::bristlecone_pine()),
            TreePreset::Ash => Some(TreePresetValues::ash()),
            TreePreset::Linden => Some(TreePresetValues::linden()),
            TreePreset::Sycamore => Some(TreePresetValues::sycamore()),
            TreePreset::Aspen => Some(TreePresetValues::aspen()),
            TreePreset::RoyalPalm => Some(TreePresetValues::royal_palm()),
            TreePreset::FanPalm => Some(TreePresetValues::fan_palm()),
            TreePreset::Eucalyptus => Some(TreePresetValues::eucalyptus()),
        }
    }
}

impl TreePresetValues {
    /// Oak: Wide spreading branches, medium height, spherical crown
    pub fn oak() -> Self {
        Self {
            trunk_height: 8.0, // Updated: scaled from 18-25m reference
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
            branch_density: 2.5, // Research suggests 2.5 for dense oak crown
            branch_length: 0.8,  // Research suggests 0.8 for spreading branches
            branch_angle: 55.0,
            branch_radius_ratio: 0.45, // Research suggests 0.45 for thick oak branches
            branch_taper: 0.7,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.25,
            up_attraction: 0.15,
            branch_recursion: 2,
            sub_branch_count: 4, // Research suggests 4 for oak
            sub_branch_scale: 0.55,
            branch_length_variation: 0.2,
            sub_branch_position_bias: 0.2,
            apical_dominance: 0.3,
            branch_flatness: 0.3,
            branch_angle_curve: 0.0,
            crown_angle_variation: -0.2, // Lower branches spread horizontally
            trunk_twist: 15.0,
            branch_twist: 10.0,
            gravity_strength: 0.35, // Updated: was 0.3, research suggests 0.35
            stiffness: 0.6,
            break_chance: 0.05,
            split_enabled: true,
            split_probability: 0.65, // Updated: research suggests 0.65 for oak
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
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
            branch_start: 0.5, // Updated: pine self-prunes lower branches
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
            split_radius_multiplier: 0.9,
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
            up_attraction: -0.8, // Updated from -0.75 - research suggests -0.8
            branch_recursion: 4, // Research suggests 4 for willow cascades
            sub_branch_count: 3,
            sub_branch_scale: 0.75, // Research suggests 0.75 for willow
            branch_length_variation: 0.25,
            sub_branch_position_bias: 0.3,
            apical_dominance: 0.2,
            branch_flatness: 0.2,
            branch_angle_curve: -0.4,
            crown_angle_variation: -0.3, // Strong horizontal spread at bottom
            trunk_twist: 10.0,
            branch_twist: 15.0,
            gravity_strength: 0.9, // Research suggests 0.9 for weeping effect
            stiffness: 0.2,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.05,
            split_radius_multiplier: 0.9,
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
            trunk_radius: 0.2, // Updated: slender trunk
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
            gravity_strength: 0.4, // Updated: research suggests 0.4 for pendulous tips
            stiffness: 0.7,
            break_chance: 0.1,
            split_enabled: true,
            split_probability: 0.2,
            split_angle: 25.0,
            split_position: 0.5,
            split_radius_threshold: 0.05,
            split_radius_multiplier: 0.9,
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
            split_radius_multiplier: 0.9,
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
            branch_density: 4.0, // Research suggests 3.5-4.5
            branch_length: 0.25,
            branch_angle: 12.0, // Research suggests 12-15 for tight columnar form
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
            split_radius_multiplier: 0.9,
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
            split_radius_multiplier: 0.9,
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
            branch_density: 3.0, // Updated: sugar maples are dense
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
            split_probability: 0.7, // Updated: research suggests 0.7 for maple
            split_angle: 28.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
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
            branch_start: 0.15, // Research suggests 0.15, slight ground clearance
            branch_end: 0.95,
            branch_density: 2.2,
            branch_length: 0.5,
            branch_angle: 85.0, // Updated from 78.0 - research suggests 85-95°, near horizontal
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
            gravity_strength: 0.3, // Updated: older branches droop
            stiffness: 0.75,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            split_radius_multiplier: 0.9,
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
            split_radius_multiplier: 0.9,
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
            split_radius_multiplier: 0.9,
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
            split_radius_multiplier: 0.9,
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
            split_radius_multiplier: 0.9,
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
            split_radius_multiplier: 0.9,
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
            split_radius_multiplier: 0.9,
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

    /// Elm: Vase-shaped crown with arching branches
    pub fn elm() -> Self {
        Self {
            trunk_height: 12.0,
            trunk_radius: 0.6,
            radial_segments: 8,
            height_segments: 6,
            trunk_taper: 0.25,
            trunk_taper_curve: 0.5,
            trunk_flare: 1.15,
            trunk_randomness: 0.05,
            root_flare_count: 4,
            root_flare_spread: 0.5,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.1,
            leader_taper: 0.15,
            leader_has_branches: false,
            branch_start: 0.3,
            branch_end: 0.9,
            branch_density: 2.0,
            branch_length: 0.55,
            branch_angle: 45.0,
            branch_radius_ratio: 0.3,
            branch_taper: 0.7,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.2,
            up_attraction: 0.4, // Arches upward and outward
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.2,
            sub_branch_position_bias: 0.2,
            apical_dominance: 0.35, // Low - vase-shaped crown
            branch_flatness: 0.2,
            branch_angle_curve: -0.6, // Branches arch outward
            crown_angle_variation: -0.3,
            trunk_twist: 8.0,
            branch_twist: 10.0,
            gravity_strength: 0.2,
            stiffness: 0.5,
            break_chance: 0.05,
            split_enabled: true,
            split_probability: 0.35,
            split_angle: 35.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Hemispherical,
            crown_influence: 0.85,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.38, 0.32, 0.25), // Gray-brown bark
            foliage_color: Color::from_rgb(0.2, 0.4, 0.15), // Medium green
            foliage: Some(FoliagePresetValues::elm()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Fir: Noble/Douglas symmetric pyramid shape
    pub fn fir() -> Self {
        Self {
            trunk_height: 15.0,
            trunk_radius: 0.5,
            radial_segments: 8,
            height_segments: 8,
            trunk_taper: 0.08,
            trunk_taper_curve: 0.85,
            trunk_flare: 1.05,
            trunk_randomness: 0.0,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.2,
            leader_taper: 0.03,
            leader_has_branches: false,
            branch_start: 0.15,
            branch_end: 0.95,
            branch_density: 2.0,
            branch_length: 0.45,
            branch_angle: 85.0, // Near horizontal whorled branches
            branch_radius_ratio: 0.22,
            branch_taper: 0.8,
            phyllotaxis_angle: 72.0, // Whorled branching pattern
            branch_randomness: 0.08,
            up_attraction: -0.1,
            branch_recursion: 1,
            sub_branch_count: 2,
            sub_branch_scale: 0.4,
            branch_length_variation: 0.1,
            sub_branch_position_bias: 0.0,
            apical_dominance: 0.85, // Strong central leader
            branch_flatness: 0.5,   // Horizontal spray effect
            branch_angle_curve: 0.4,
            crown_angle_variation: 0.35,
            trunk_twist: 0.0,
            branch_twist: 3.0,
            gravity_strength: 0.05,
            stiffness: 0.8,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Conical,
            crown_influence: 1.0,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.35, 0.25, 0.18), // Gray-brown bark
            foliage_color: Color::from_rgb(0.1, 0.28, 0.2), // Blue-green
            foliage: Some(FoliagePresetValues::fir()),
            growth: Some(GrowthPresetValues::structured()),
        }
    }

    /// Cedar: Tiered flat sprays with horizontal branching
    pub fn cedar() -> Self {
        Self {
            trunk_height: 14.0,
            trunk_radius: 0.6,
            radial_segments: 8,
            height_segments: 7,
            trunk_taper: 0.15,
            trunk_taper_curve: 0.6,
            trunk_flare: 1.5, // Distinctive buttressed base
            trunk_randomness: 0.03,
            root_flare_count: 5,
            root_flare_spread: 0.6,
            root_flare_height: 0.2,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.15,
            leader_taper: 0.05,
            leader_has_branches: false,
            branch_start: 0.2,
            branch_end: 0.95,
            branch_density: 1.8,
            branch_length: 0.5,
            branch_angle: 75.0, // Near horizontal
            branch_radius_ratio: 0.25,
            branch_taper: 0.75,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.15,
            up_attraction: -0.15, // Slight droop for flat sprays
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.5,
            branch_length_variation: 0.15,
            sub_branch_position_bias: 0.1,
            apical_dominance: 0.7,
            branch_flatness: 0.65, // Flat spray effect
            branch_angle_curve: 0.2,
            crown_angle_variation: 0.2,
            trunk_twist: 5.0,
            branch_twist: 5.0,
            gravity_strength: 0.1,
            stiffness: 0.7,
            break_chance: 0.02,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.6,
            crown_shape: CrownShape::Cylindrical,
            crown_influence: 0.8,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.45, 0.3, 0.2), // Red-brown bark
            foliage_color: Color::from_rgb(0.15, 0.35, 0.22), // Dark green
            foliage: Some(FoliagePresetValues::cedar()),
            growth: Some(GrowthPresetValues::structured()),
        }
    }

    /// Joshua Tree: Chaotic dichotomous (binary) forking pattern
    pub fn joshua_tree() -> Self {
        Self {
            trunk_height: 4.0,
            trunk_radius: 0.25,
            radial_segments: 6,
            height_segments: 3,
            trunk_taper: 0.2,
            trunk_taper_curve: 0.5,
            trunk_flare: 1.1,
            trunk_randomness: 0.15,
            root_flare_count: 2,
            root_flare_spread: 0.3,
            root_flare_height: 0.1,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.05,
            leader_taper: 0.15,
            leader_has_branches: false,
            branch_start: 0.5,
            branch_end: 0.9,
            branch_density: 0.8,
            branch_length: 0.35,
            branch_angle: 50.0,
            branch_radius_ratio: 0.4,
            branch_taper: 0.5,
            phyllotaxis_angle: 180.0, // Binary forking
            branch_randomness: 0.7,   // High randomness for chaotic look
            up_attraction: 0.3,
            branch_recursion: 3,
            sub_branch_count: 2, // Binary forking
            sub_branch_scale: 0.75,
            branch_length_variation: 0.25,
            sub_branch_position_bias: 0.4,
            apical_dominance: 0.3, // Low - equal branching
            branch_flatness: 0.15,
            branch_angle_curve: 0.0,
            crown_angle_variation: 0.0,
            trunk_twist: 20.0,
            branch_twist: 15.0,
            gravity_strength: 0.1,
            stiffness: 0.55,
            break_chance: 0.08,
            split_enabled: true,
            split_probability: 0.85, // High split for dichotomous look
            split_angle: 45.0,
            split_position: 0.5,
            split_radius_threshold: 0.06,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.4,
            crown_shape: CrownShape::Hemispherical, // Irregular crown
            crown_influence: 0.5,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.5, 0.45, 0.35), // Light tan bark
            foliage_color: Color::from_rgb(0.35, 0.45, 0.25), // Yellow-green
            foliage: Some(FoliagePresetValues::joshua_tree()),
            growth: Some(GrowthPresetValues::forking()),
        }
    }

    /// Olive: Gnarled Mediterranean tree with twisted trunk
    pub fn olive() -> Self {
        Self {
            trunk_height: 4.5,
            trunk_radius: 0.4,
            radial_segments: 8,
            height_segments: 4,
            trunk_taper: 0.35,
            trunk_taper_curve: 0.4,
            trunk_flare: 1.3,
            trunk_randomness: 0.35, // High randomness for gnarled look
            root_flare_count: 4,
            root_flare_spread: 0.6,
            root_flare_height: 0.2,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.08,
            leader_taper: 0.15,
            leader_has_branches: false,
            branch_start: 0.35,
            branch_end: 0.85,
            branch_density: 1.5,
            branch_length: 0.4,
            branch_angle: 55.0,
            branch_radius_ratio: 0.35,
            branch_taper: 0.65,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.4,
            up_attraction: 0.1,
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.25,
            sub_branch_position_bias: 0.2,
            apical_dominance: 0.35,
            branch_flatness: 0.3,
            branch_angle_curve: -0.15,
            crown_angle_variation: -0.15,
            trunk_twist: 50.0, // High twist for gnarled appearance
            branch_twist: 20.0,
            gravity_strength: 0.2,
            stiffness: 0.45,
            break_chance: 0.08,
            split_enabled: true,
            split_probability: 0.45,
            split_angle: 40.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.7,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.45, 0.4, 0.35), // Gray-brown bark
            foliage_color: Color::from_rgb(0.4, 0.5, 0.35), // Silver-green
            foliage: Some(FoliagePresetValues::olive()),
            growth: Some(GrowthPresetValues::gnarled()),
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 2: New Species Presets
    // ═══════════════════════════════════════════════════════════════

    /// Cherry Blossom (Sakura): Graceful ornamental with distinctive spreading form
    pub fn cherry_blossom() -> Self {
        Self {
            trunk_height: 5.0,
            trunk_radius: 0.3,
            radial_segments: 8,
            height_segments: 4,
            trunk_taper: 0.35,
            trunk_taper_curve: 0.45,
            trunk_flare: 1.15,
            trunk_randomness: 0.1,
            root_flare_count: 3,
            root_flare_spread: 0.4,
            root_flare_height: 0.12,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.1,
            leader_taper: 0.12,
            leader_has_branches: false,
            branch_start: 0.3,
            branch_end: 0.9,
            branch_density: 2.0,
            branch_length: 0.55,
            branch_angle: 50.0, // Graceful spreading
            branch_radius_ratio: 0.28,
            branch_taper: 0.72,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.25,
            up_attraction: 0.05,
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.2,
            sub_branch_position_bias: 0.15,
            apical_dominance: 0.4,
            branch_flatness: 0.35, // Horizontal layered appearance
            branch_angle_curve: -0.2,
            crown_angle_variation: -0.2,
            trunk_twist: 12.0,
            branch_twist: 8.0,
            gravity_strength: 0.18,
            stiffness: 0.5,
            break_chance: 0.03,
            split_enabled: true,
            split_probability: 0.25,
            split_angle: 32.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.4,
            crown_shape: CrownShape::Spreading,
            crown_influence: 0.85,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.35, 0.25, 0.2), // Dark bark
            foliage_color: Color::from_rgb(1.0, 0.85, 0.88), // Pink-white blossoms
            foliage: Some(FoliagePresetValues::cherry_blossom()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Acacia (Umbrella Tree): Iconic flat-topped African savanna silhouette
    pub fn acacia() -> Self {
        Self {
            trunk_height: 7.0,
            trunk_radius: 0.35,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.25,
            trunk_taper_curve: 0.6,
            trunk_flare: 1.2,
            trunk_randomness: 0.08,
            root_flare_count: 4,
            root_flare_spread: 0.5,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.08,
            leader_taper: 0.15,
            leader_has_branches: false,
            branch_start: 0.65, // Branches only in crown
            branch_end: 0.95,
            branch_density: 1.8,
            branch_length: 0.7, // Long horizontal branches
            branch_angle: 88.0, // Near horizontal (85-95°)
            branch_radius_ratio: 0.3,
            branch_taper: 0.65,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.2,
            up_attraction: -0.1, // Slight droop at tips
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.15,
            sub_branch_position_bias: 0.2,
            apical_dominance: 0.1, // Very low - co-dominant flat crown
            branch_flatness: 0.9,  // Extreme horizontal spread
            branch_angle_curve: -0.1,
            crown_angle_variation: -0.1,
            trunk_twist: 8.0,
            branch_twist: 5.0,
            gravity_strength: 0.12,
            stiffness: 0.55,
            break_chance: 0.05,
            split_enabled: true,
            split_probability: 0.4,
            split_angle: 35.0,
            split_position: 0.45,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Umbrella, // Flat-topped
            crown_influence: 1.0,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.4, 0.35, 0.28), // Brown-gray bark
            foliage_color: Color::from_rgb(0.35, 0.5, 0.25), // Olive green
            foliage: Some(FoliagePresetValues::acacia()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Beech: Smooth gray bark, dense spherical crown
    pub fn beech() -> Self {
        Self {
            trunk_height: 10.0,
            trunk_radius: 0.55,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.28,
            trunk_taper_curve: 0.55,
            trunk_flare: 1.18,
            trunk_randomness: 0.03,
            root_flare_count: 5,
            root_flare_spread: 0.55,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.12,
            leader_taper: 0.1,
            leader_has_branches: true,
            branch_start: 0.35,
            branch_end: 0.9,
            branch_density: 3.0, // Dense branching
            branch_length: 0.6,
            branch_angle: 50.0,
            branch_radius_ratio: 0.35,
            branch_taper: 0.7,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.18,
            up_attraction: 0.15,
            branch_recursion: 2,
            sub_branch_count: 4,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.18,
            sub_branch_position_bias: 0.15,
            apical_dominance: 0.4,
            branch_flatness: 0.25,
            branch_angle_curve: 0.0,
            crown_angle_variation: -0.15,
            trunk_twist: 5.0,
            branch_twist: 6.0,
            gravity_strength: 0.22,
            stiffness: 0.6,
            break_chance: 0.04,
            split_enabled: true,
            split_probability: 0.45,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical, // Dense dome
            crown_influence: 0.95,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.55, 0.53, 0.5), // Smooth gray bark
            foliage_color: Color::from_rgb(0.2, 0.4, 0.15), // Rich green
            foliage: Some(FoliagePresetValues::beech()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Ginkgo: Unique fan-shaped leaves, angular branches
    pub fn ginkgo() -> Self {
        Self {
            trunk_height: 9.0,
            trunk_radius: 0.45,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.22,
            trunk_taper_curve: 0.6,
            trunk_flare: 1.12,
            trunk_randomness: 0.05,
            root_flare_count: 3,
            root_flare_spread: 0.4,
            root_flare_height: 0.12,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.15,
            leader_taper: 0.08,
            leader_has_branches: true,
            branch_start: 0.35,
            branch_end: 0.92,
            branch_density: 1.8,
            branch_length: 0.5,
            branch_angle: 48.0,
            branch_radius_ratio: 0.28,
            branch_taper: 0.72,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.38, // High randomness for angular look
            up_attraction: 0.25,     // Reaches upward
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.52,
            branch_length_variation: 0.22,
            sub_branch_position_bias: 0.1,
            apical_dominance: 0.55,
            branch_flatness: 0.15,
            branch_angle_curve: 0.1,
            crown_angle_variation: 0.1,
            trunk_twist: 3.0,
            branch_twist: 5.0,
            gravity_strength: 0.1,
            stiffness: 0.7,
            break_chance: 0.02,
            split_enabled: true,
            split_probability: 0.3,
            split_angle: 35.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.4,
            crown_shape: CrownShape::Conical, // Young ginkgo - matures to spreading
            crown_influence: 0.8,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.45, 0.42, 0.38), // Gray-brown bark
            foliage_color: Color::from_rgb(0.6, 0.7, 0.3),  // Light yellow-green
            foliage: Some(FoliagePresetValues::ginkgo()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Weeping Cherry: Cascade/waterfall effect variant
    pub fn weeping_cherry() -> Self {
        Self {
            trunk_height: 4.5,
            trunk_radius: 0.28,
            radial_segments: 8,
            height_segments: 4,
            trunk_taper: 0.38,
            trunk_taper_curve: 0.4,
            trunk_flare: 1.15,
            trunk_randomness: 0.1,
            root_flare_count: 3,
            root_flare_spread: 0.4,
            root_flare_height: 0.12,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.1,
            leader_taper: 0.12,
            leader_has_branches: false,
            branch_start: 0.35,
            branch_end: 0.85,
            branch_density: 2.2,
            branch_length: 0.65,
            branch_angle: 55.0,
            branch_radius_ratio: 0.25,
            branch_taper: 0.8,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.28,
            up_attraction: -0.6, // Strong downward cascade
            branch_recursion: 3,
            sub_branch_count: 3,
            sub_branch_scale: 0.68,
            branch_length_variation: 0.22,
            sub_branch_position_bias: 0.25,
            apical_dominance: 0.25,
            branch_flatness: 0.3,
            branch_angle_curve: -0.4,
            crown_angle_variation: -0.3,
            trunk_twist: 10.0,
            branch_twist: 12.0,
            gravity_strength: 0.75, // Strong weeping
            stiffness: 0.25,        // Flexible branches
            break_chance: 0.02,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.05,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.4,
            crown_shape: CrownShape::Hemispherical, // Umbrella-like weeping form
            crown_influence: 0.85,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.38, 0.28, 0.22), // Dark cherry bark
            foliage_color: Color::from_rgb(1.0, 0.8, 0.85), // Pink blossoms
            foliage: Some(FoliagePresetValues::weeping_cherry()),
            growth: Some(GrowthPresetValues::weeping()),
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 3: Fantasy/Stylized Presets
    // ═══════════════════════════════════════════════════════════════

    /// World Tree (Yggdrasil): Massive landmark-scale tree
    pub fn world_tree() -> Self {
        Self {
            trunk_height: 50.0, // Massive scale
            trunk_radius: 10.0, // Giant trunk
            radial_segments: 12,
            height_segments: 12,
            trunk_taper: 0.15,
            trunk_taper_curve: 0.6,
            trunk_flare: 1.8, // Large root flare
            trunk_randomness: 0.06,
            root_flare_count: 8, // Many visible roots
            root_flare_spread: 1.2,
            root_flare_height: 0.3, // Tall root buttresses
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.08,
            leader_taper: 0.1,
            leader_has_branches: true,
            branch_start: 0.4,
            branch_end: 0.9,
            branch_density: 0.5, // Few massive branches
            branch_length: 0.6,
            branch_angle: 50.0,
            branch_radius_ratio: 0.4, // Thick primary branches
            branch_taper: 0.65,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.2,
            up_attraction: 0.15,
            branch_recursion: 3,
            sub_branch_count: 4,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.2,
            sub_branch_position_bias: 0.15,
            apical_dominance: 0.35,
            branch_flatness: 0.25,
            branch_angle_curve: -0.1,
            crown_angle_variation: -0.15,
            trunk_twist: 20.0,
            branch_twist: 15.0,
            gravity_strength: 0.25,
            stiffness: 0.5,
            break_chance: 0.03,
            split_enabled: true,
            split_probability: 0.5,
            split_angle: 35.0,
            split_position: 0.5,
            split_radius_threshold: 0.15,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 2.0,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.85,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.3, 0.22, 0.15), // Ancient dark bark
            foliage_color: Color::from_rgb(0.15, 0.35, 0.12), // Deep forest green
            foliage: Some(FoliagePresetValues::world_tree()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Crystal Tree: Faceted geometric, no organic curves
    pub fn crystal_tree() -> Self {
        Self {
            trunk_height: 6.0,
            trunk_radius: 0.4,
            radial_segments: 6, // Hexagonal facets
            height_segments: 4,
            trunk_taper: 0.2,
            trunk_taper_curve: 0.5, // Linear taper
            trunk_flare: 1.0,       // No flare
            trunk_randomness: 0.0,  // No randomness - pure geometry
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.0,
            trunk_termination: TrunkTermination::PointedTip,
            leader_length: 0.15,
            leader_taper: 0.05,
            leader_has_branches: false,
            branch_start: 0.25,
            branch_end: 0.9,
            branch_density: 1.5,
            branch_length: 0.45,
            branch_angle: 60.0, // Geometric 60° angles
            branch_radius_ratio: 0.25,
            branch_taper: 0.75,
            phyllotaxis_angle: 60.0, // 60° intervals for hexagonal pattern
            branch_randomness: 0.0,  // No randomness
            up_attraction: 0.3,
            branch_recursion: 2,
            sub_branch_count: 2,
            sub_branch_scale: 0.6,
            branch_length_variation: 0.0, // Uniform
            sub_branch_position_bias: 0.0,
            apical_dominance: 0.6,
            branch_flatness: 0.0,
            branch_angle_curve: 0.0,
            crown_angle_variation: 0.0,
            trunk_twist: 0.0, // No twist
            branch_twist: 0.0,
            gravity_strength: 0.0, // No gravity
            stiffness: 1.0,        // Perfectly rigid
            break_chance: 0.0,
            split_enabled: true,
            split_probability: 0.4,
            split_angle: 60.0, // 60° split angle
            split_position: 0.5,
            split_radius_threshold: 0.08,
            split_radius_multiplier: 0.9,
            floor_avoidance: false,
            floor_level: 0.0,
            branch_collar_enabled: false, // No organic collars
            branch_collar_length: 1.0,
            crown_shape: CrownShape::Conical,
            crown_influence: 0.8,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.7, 0.85, 0.95), // Ice blue crystal
            foliage_color: Color::from_rgb(0.8, 0.9, 1.0), // Lighter crystal tips
            foliage: None, // No foliage - crystal formations instead
            growth: Some(GrowthPresetValues::structured()),
        }
    }

    /// Corrupted Tree: Twisted, diseased, dark horror appearance
    pub fn corrupted_tree() -> Self {
        Self {
            trunk_height: 6.0,
            trunk_radius: 0.5,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.35,
            trunk_taper_curve: 0.4,
            trunk_flare: 1.3,
            trunk_randomness: 0.35, // High randomness for twisted look
            root_flare_count: 5,
            root_flare_spread: 0.7,
            root_flare_height: 0.2,
            trunk_termination: TrunkTermination::FlatCap, // Broken top
            leader_length: 0.05,
            leader_taper: 0.2,
            leader_has_branches: false,
            branch_start: 0.2,
            branch_end: 0.75, // Branches end early (diseased top)
            branch_density: 1.0,
            branch_length: 0.45,
            branch_angle: 55.0,
            branch_radius_ratio: 0.35,
            branch_taper: 0.55, // Irregular taper
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.5, // Very irregular
            up_attraction: -0.15,   // Tends downward
            branch_recursion: 2,
            sub_branch_count: 2,
            sub_branch_scale: 0.5,
            branch_length_variation: 0.35,
            sub_branch_position_bias: 0.25,
            apical_dominance: 0.25,
            branch_flatness: 0.2,
            branch_angle_curve: 0.0,
            crown_angle_variation: 0.0,
            trunk_twist: 65.0, // Extreme twist
            branch_twist: 35.0,
            gravity_strength: 0.35,
            stiffness: 0.3,
            break_chance: 0.4, // Many broken branches
            split_enabled: true,
            split_probability: 0.3,
            split_angle: 45.0,
            split_position: 0.4,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: false, // Branches can touch ground
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.6,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.4,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.15, 0.1, 0.12), // Dark purple-black bark
            foliage_color: Color::from_rgb(0.25, 0.1, 0.3), // Dark purple foliage
            foliage: Some(FoliagePresetValues::corrupted()),
            growth: Some(GrowthPresetValues::gnarled()),
        }
    }

    /// Glowing Tree: Magical tree base (oak-like) with emission-ready foliage
    pub fn glowing_tree() -> Self {
        Self {
            trunk_height: 7.0,
            trunk_radius: 0.5,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.3,
            trunk_taper_curve: 0.5,
            trunk_flare: 1.2,
            trunk_randomness: 0.08,
            root_flare_count: 4,
            root_flare_spread: 0.5,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.12,
            leader_taper: 0.1,
            leader_has_branches: true,
            branch_start: 0.35,
            branch_end: 0.9,
            branch_density: 2.2,
            branch_length: 0.6,
            branch_angle: 50.0,
            branch_radius_ratio: 0.35,
            branch_taper: 0.7,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.22,
            up_attraction: 0.12,
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.18,
            sub_branch_position_bias: 0.15,
            apical_dominance: 0.35,
            branch_flatness: 0.25,
            branch_angle_curve: 0.0,
            crown_angle_variation: -0.15,
            trunk_twist: 12.0,
            branch_twist: 8.0,
            gravity_strength: 0.2,
            stiffness: 0.55,
            break_chance: 0.03,
            split_enabled: true,
            split_probability: 0.4,
            split_angle: 32.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.9,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.25, 0.2, 0.25), // Dark mystical bark
            foliage_color: Color::from_rgb(0.4, 0.9, 0.6), // Bright magical green (emission-ready)
            foliage: Some(FoliagePresetValues::glowing()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 4: New Species from Research
    // ═══════════════════════════════════════════════════════════════

    /// Blue Spruce: Blue-gray foliage, compact conical form
    pub fn blue_spruce() -> Self {
        Self {
            trunk_height: 7.0,
            trunk_radius: 0.35,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.12,
            trunk_taper_curve: 0.8,
            trunk_flare: 1.05,
            trunk_randomness: 0.0,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.2,
            leader_taper: 0.04,
            leader_has_branches: false,
            branch_start: 0.1,
            branch_end: 0.95,
            branch_density: 2.5,
            branch_length: 0.4,
            branch_angle: 80.0, // Near horizontal
            branch_radius_ratio: 0.22,
            branch_taper: 0.8,
            phyllotaxis_angle: 72.0, // Whorled
            branch_randomness: 0.08,
            up_attraction: -0.1,
            branch_recursion: 1,
            sub_branch_count: 2,
            sub_branch_scale: 0.35,
            branch_length_variation: 0.08,
            sub_branch_position_bias: 0.0,
            apical_dominance: 0.9,
            branch_flatness: 0.0,
            branch_angle_curve: 0.35,
            crown_angle_variation: 0.3,
            trunk_twist: 0.0,
            branch_twist: 3.0,
            gravity_strength: 0.05,
            stiffness: 0.8,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Conical,
            crown_influence: 1.0,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.38, 0.24, 0.14), // Reddish-brown bark
            foliage_color: Color::from_rgb(0.45, 0.55, 0.65), // Blue-gray foliage
            foliage: Some(FoliagePresetValues::blue_spruce()),
            growth: Some(GrowthPresetValues::structured()),
        }
    }

    /// Douglas Fir: Tiered branches, flat sprays
    pub fn douglas_fir() -> Self {
        Self {
            trunk_height: 15.0,
            trunk_radius: 0.6,
            radial_segments: 8,
            height_segments: 8,
            trunk_taper: 0.08,
            trunk_taper_curve: 0.85,
            trunk_flare: 1.1,
            trunk_randomness: 0.02,
            root_flare_count: 4,
            root_flare_spread: 0.4,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.2,
            leader_taper: 0.03,
            leader_has_branches: false,
            branch_start: 0.3,
            branch_end: 0.95,
            branch_density: 2.0,
            branch_length: 0.5,
            branch_angle: 80.0,
            branch_radius_ratio: 0.24,
            branch_taper: 0.78,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.1,
            up_attraction: -0.15,
            branch_recursion: 2,
            sub_branch_count: 2,
            sub_branch_scale: 0.45,
            branch_length_variation: 0.12,
            sub_branch_position_bias: 0.0,
            apical_dominance: 0.85,
            branch_flatness: 0.6, // Tiered flat sprays
            branch_angle_curve: 0.3,
            crown_angle_variation: 0.25,
            trunk_twist: 0.0,
            branch_twist: 4.0,
            gravity_strength: 0.1,
            stiffness: 0.75,
            break_chance: 0.02,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Conical,
            crown_influence: 0.95,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.4, 0.3, 0.2), // Gray-brown bark
            foliage_color: Color::from_rgb(0.12, 0.32, 0.18), // Dark green
            foliage: Some(FoliagePresetValues::douglas_fir()),
            growth: Some(GrowthPresetValues::structured()),
        }
    }

    /// Ponderosa Pine: Cinnamon bark, needle clusters
    pub fn ponderosa_pine() -> Self {
        Self {
            trunk_height: 12.0,
            trunk_radius: 0.5,
            radial_segments: 8,
            height_segments: 6,
            trunk_taper: 0.1,
            trunk_taper_curve: 0.75,
            trunk_flare: 1.1,
            trunk_randomness: 0.02,
            root_flare_count: 3,
            root_flare_spread: 0.35,
            root_flare_height: 0.12,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.18,
            leader_taper: 0.05,
            leader_has_branches: false,
            branch_start: 0.55, // High crown clearance
            branch_end: 0.95,
            branch_density: 1.5,
            branch_length: 0.45,
            branch_angle: 70.0,
            branch_radius_ratio: 0.25,
            branch_taper: 0.75,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.15,
            up_attraction: -0.05,
            branch_recursion: 1,
            sub_branch_count: 3, // Cluster of 3 needles
            sub_branch_scale: 0.4,
            branch_length_variation: 0.1,
            sub_branch_position_bias: 0.1,
            apical_dominance: 0.8,
            branch_flatness: 0.0,
            branch_angle_curve: 0.25,
            crown_angle_variation: 0.2,
            trunk_twist: 0.0,
            branch_twist: 5.0,
            gravity_strength: 0.08,
            stiffness: 0.75,
            break_chance: 0.02,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.08,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Conical,
            crown_influence: 0.85,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.65, 0.4, 0.2), // Cinnamon bark
            foliage_color: Color::from_rgb(0.15, 0.35, 0.2), // Yellow-green needles
            foliage: Some(FoliagePresetValues::ponderosa_pine()),
            growth: Some(GrowthPresetValues::structured()),
        }
    }

    /// Bristlecone Pine: Ancient gnarled, twisted trunk
    pub fn bristlecone_pine() -> Self {
        Self {
            trunk_height: 6.0,
            trunk_radius: 0.3,
            radial_segments: 8,
            height_segments: 4,
            trunk_taper: 0.25,
            trunk_taper_curve: 0.45,
            trunk_flare: 1.2,
            trunk_randomness: 0.3, // Very gnarled
            root_flare_count: 3,
            root_flare_spread: 0.5,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.05,
            leader_taper: 0.15,
            leader_has_branches: false,
            branch_start: 0.25,
            branch_end: 0.85,
            branch_density: 0.8, // Sparse
            branch_length: 0.35,
            branch_angle: 55.0,
            branch_radius_ratio: 0.35,
            branch_taper: 0.6,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.45,
            up_attraction: 0.0,
            branch_recursion: 2,
            sub_branch_count: 2,
            sub_branch_scale: 0.5,
            branch_length_variation: 0.3,
            sub_branch_position_bias: 0.25,
            apical_dominance: 0.35,
            branch_flatness: 0.2,
            branch_angle_curve: 0.0,
            crown_angle_variation: 0.0,
            trunk_twist: 90.0, // Extreme twist for ancient look
            branch_twist: 30.0,
            gravity_strength: 0.15,
            stiffness: 0.4,
            break_chance: 0.3, // Many dead/broken branches
            split_enabled: true,
            split_probability: 0.3,
            split_angle: 40.0,
            split_position: 0.45,
            split_radius_threshold: 0.08,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.5,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.5, 0.45, 0.4), // Weathered gray bark
            foliage_color: Color::from_rgb(0.2, 0.35, 0.25), // Dark green needles
            foliage: Some(FoliagePresetValues::bristlecone_pine()),
            growth: Some(GrowthPresetValues::gnarled()),
        }
    }

    /// Ash: Opposite branching, open crown
    pub fn ash() -> Self {
        Self {
            trunk_height: 10.0,
            trunk_radius: 0.4,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.28,
            trunk_taper_curve: 0.55,
            trunk_flare: 1.15,
            trunk_randomness: 0.05,
            root_flare_count: 4,
            root_flare_spread: 0.5,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.12,
            leader_taper: 0.1,
            leader_has_branches: true,
            branch_start: 0.4,
            branch_end: 0.9,
            branch_density: 2.0,
            branch_length: 0.55,
            branch_angle: 55.0,
            branch_radius_ratio: 0.3,
            branch_taper: 0.7,
            phyllotaxis_angle: 180.0, // Opposite branching
            branch_randomness: 0.2,
            up_attraction: 0.15,
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.18,
            sub_branch_position_bias: 0.15,
            apical_dominance: 0.5,
            branch_flatness: 0.2,
            branch_angle_curve: -0.1,
            crown_angle_variation: -0.15,
            trunk_twist: 5.0,
            branch_twist: 8.0,
            gravity_strength: 0.2,
            stiffness: 0.55,
            break_chance: 0.04,
            split_enabled: true,
            split_probability: 0.35,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical, // Open rounded crown
            crown_influence: 0.8,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.4, 0.38, 0.35), // Gray bark
            foliage_color: Color::from_rgb(0.2, 0.42, 0.18), // Medium green
            foliage: Some(FoliagePresetValues::ash()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Linden: Dense heart leaves, conical to spherical crown
    pub fn linden() -> Self {
        Self {
            trunk_height: 8.0,
            trunk_radius: 0.45,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.3,
            trunk_taper_curve: 0.55,
            trunk_flare: 1.2,
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
            branch_density: 3.0, // Dense
            branch_length: 0.5,
            branch_angle: 50.0,
            branch_radius_ratio: 0.32,
            branch_taper: 0.7,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.18,
            up_attraction: 0.1,
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.18,
            sub_branch_position_bias: 0.15,
            apical_dominance: 0.55,
            branch_flatness: 0.25,
            branch_angle_curve: 0.0,
            crown_angle_variation: -0.1,
            trunk_twist: 5.0,
            branch_twist: 6.0,
            gravity_strength: 0.2,
            stiffness: 0.6,
            break_chance: 0.03,
            split_enabled: true,
            split_probability: 0.4,
            split_angle: 28.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical, // Dense dome
            crown_influence: 0.9,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.35, 0.32, 0.28), // Gray-brown bark
            foliage_color: Color::from_rgb(0.22, 0.45, 0.18), // Rich green
            foliage: Some(FoliagePresetValues::linden()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Sycamore: Buttressed trunk, irregular crown
    pub fn sycamore() -> Self {
        Self {
            trunk_height: 12.0,
            trunk_radius: 0.6,
            radial_segments: 10,
            height_segments: 6,
            trunk_taper: 0.3,
            trunk_taper_curve: 0.5,
            trunk_flare: 1.6, // Buttressed trunk
            trunk_randomness: 0.1,
            root_flare_count: 6,
            root_flare_spread: 0.7,
            root_flare_height: 0.2,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.08,
            leader_taper: 0.12,
            leader_has_branches: false,
            branch_start: 0.35,
            branch_end: 0.9,
            branch_density: 2.2,
            branch_length: 0.6,
            branch_angle: 55.0,
            branch_radius_ratio: 0.35,
            branch_taper: 0.68,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.28, // Irregular
            up_attraction: 0.1,
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.55,
            branch_length_variation: 0.22,
            sub_branch_position_bias: 0.2,
            apical_dominance: 0.35, // Co-dominant
            branch_flatness: 0.2,
            branch_angle_curve: -0.15,
            crown_angle_variation: -0.2,
            trunk_twist: 10.0,
            branch_twist: 12.0,
            gravity_strength: 0.25,
            stiffness: 0.5,
            break_chance: 0.05,
            split_enabled: true,
            split_probability: 0.45,
            split_angle: 35.0,
            split_position: 0.5,
            split_radius_threshold: 0.12,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.6,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.75,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.7, 0.68, 0.6), // Pale mottled bark
            foliage_color: Color::from_rgb(0.25, 0.45, 0.2), // Medium green
            foliage: Some(FoliagePresetValues::sycamore()),
            growth: Some(GrowthPresetValues::spreading()),
        }
    }

    /// Aspen: Slender, trembling leaves
    pub fn aspen() -> Self {
        Self {
            trunk_height: 7.0,
            trunk_radius: 0.2, // Slender
            radial_segments: 6,
            height_segments: 5,
            trunk_taper: 0.15,
            trunk_taper_curve: 0.6,
            trunk_flare: 1.0,
            trunk_randomness: 0.08,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.15,
            leader_taper: 0.08,
            leader_has_branches: false,
            branch_start: 0.45,
            branch_end: 0.95,
            branch_density: 1.8,
            branch_length: 0.35,
            branch_angle: 45.0,
            branch_radius_ratio: 0.2,
            branch_taper: 0.75,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.15,
            up_attraction: 0.35, // Upward reaching
            branch_recursion: 1,
            sub_branch_count: 2,
            sub_branch_scale: 0.5,
            branch_length_variation: 0.15,
            sub_branch_position_bias: 0.1,
            apical_dominance: 0.7,
            branch_flatness: 0.1,
            branch_angle_curve: 0.15,
            crown_angle_variation: 0.1,
            trunk_twist: 3.0,
            branch_twist: 5.0,
            gravity_strength: 0.1,
            stiffness: 0.7,
            break_chance: 0.05,
            split_enabled: true,
            split_probability: 0.2,
            split_angle: 25.0,
            split_position: 0.5,
            split_radius_threshold: 0.05,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.4,
            crown_shape: CrownShape::TaperedCylindrical,
            crown_influence: 0.8,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.85, 0.85, 0.75), // White-green bark
            foliage_color: Color::from_rgb(0.4, 0.55, 0.3), // Light green
            foliage: Some(FoliagePresetValues::aspen()),
            growth: Some(GrowthPresetValues::columnar()),
        }
    }

    /// Royal Palm: Perfectly straight, gray-white trunk
    pub fn royal_palm() -> Self {
        Self {
            trunk_height: 10.0,
            trunk_radius: 0.25,
            radial_segments: 8,
            height_segments: 6,
            trunk_taper: 0.99, // Almost no taper (cylindrical)
            trunk_taper_curve: 0.5,
            trunk_flare: 1.0,
            trunk_randomness: 0.0, // Perfectly straight
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.15,
            leader_taper: 0.1,
            leader_has_branches: false,
            branch_start: 0.9, // Only at crown
            branch_end: 0.98,
            branch_density: 2.0,
            branch_length: 0.7,
            branch_angle: 55.0,
            branch_radius_ratio: 0.2,
            branch_taper: 0.9,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.05,
            up_attraction: 0.0,
            branch_recursion: 0,
            sub_branch_count: 0,
            sub_branch_scale: 0.5,
            branch_length_variation: 0.1,
            sub_branch_position_bias: 0.0,
            apical_dominance: 0.95,
            branch_flatness: 0.0,
            branch_angle_curve: 0.0,
            crown_angle_variation: 0.0,
            trunk_twist: 0.0,
            branch_twist: 0.0,
            gravity_strength: 0.1,
            stiffness: 0.9,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: false,
            branch_collar_length: 1.0,
            crown_shape: CrownShape::Cylindrical,
            crown_influence: 0.5,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.75, 0.75, 0.7), // Gray-white
            foliage_color: Color::from_rgb(0.15, 0.5, 0.2), // Dark green fronds
            foliage: Some(FoliagePresetValues::royal_palm()),
            growth: None,
        }
    }

    /// Fan Palm: Palmate fronds, petticoat skirt
    pub fn fan_palm() -> Self {
        Self {
            trunk_height: 6.0,
            trunk_radius: 0.3,
            radial_segments: 8,
            height_segments: 5,
            trunk_taper: 0.4,
            trunk_taper_curve: 0.3,
            trunk_flare: 1.2,
            trunk_randomness: 0.02,
            root_flare_count: 0,
            root_flare_spread: 0.0,
            root_flare_height: 0.15,
            trunk_termination: TrunkTermination::FlatCap,
            leader_length: 0.1,
            leader_taper: 0.1,
            leader_has_branches: false,
            branch_start: 0.75, // Petticoat skirt of dead fronds starts lower
            branch_end: 0.98,
            branch_density: 3.0,
            branch_length: 0.55,
            branch_angle: 70.0,
            branch_radius_ratio: 0.22,
            branch_taper: 0.85,
            phyllotaxis_angle: 45.0,
            branch_randomness: 0.08,
            up_attraction: -0.15,
            branch_recursion: 0,
            sub_branch_count: 0,
            sub_branch_scale: 0.5,
            branch_length_variation: 0.12,
            sub_branch_position_bias: 0.0,
            apical_dominance: 0.9,
            branch_flatness: 0.0,
            branch_angle_curve: 0.0,
            crown_angle_variation: 0.0,
            trunk_twist: 0.0,
            branch_twist: 0.0,
            gravity_strength: 0.2,
            stiffness: 0.6,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: false,
            branch_collar_length: 1.0,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.6,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.5, 0.45, 0.35), // Tan-brown
            foliage_color: Color::from_rgb(0.2, 0.55, 0.25), // Bright green fronds
            foliage: Some(FoliagePresetValues::fan_palm()),
            growth: None,
        }
    }

    /// Eucalyptus: Tall, pendulous, hanging leaves
    pub fn eucalyptus() -> Self {
        Self {
            trunk_height: 15.0,
            trunk_radius: 0.4,
            radial_segments: 8,
            height_segments: 7,
            trunk_taper: 0.18,
            trunk_taper_curve: 0.65,
            trunk_flare: 1.1,
            trunk_randomness: 0.08,
            root_flare_count: 3,
            root_flare_spread: 0.4,
            root_flare_height: 0.12,
            trunk_termination: TrunkTermination::LeaderBranch,
            leader_length: 0.12,
            leader_taper: 0.08,
            leader_has_branches: true,
            branch_start: 0.45,
            branch_end: 0.95,
            branch_density: 1.8,
            branch_length: 0.55,
            branch_angle: 50.0,
            branch_radius_ratio: 0.25,
            branch_taper: 0.75,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.25,
            up_attraction: -0.2, // Pendulous
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.6,
            branch_length_variation: 0.2,
            sub_branch_position_bias: 0.2,
            apical_dominance: 0.6,
            branch_flatness: 0.1,
            branch_angle_curve: -0.25,
            crown_angle_variation: -0.2,
            trunk_twist: 8.0,
            branch_twist: 10.0,
            gravity_strength: 0.4, // Strong droop for hanging leaves
            stiffness: 0.45,
            break_chance: 0.08,
            split_enabled: true,
            split_probability: 0.3,
            split_angle: 35.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            split_radius_multiplier: 0.9,
            floor_avoidance: true,
            floor_level: 0.0,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spreading,
            crown_influence: 0.75,
            crown_base_size: 0.0,
            crown_height: -1.0,
            trunk_color: Color::from_rgb(0.6, 0.55, 0.45), // Smooth pale bark
            foliage_color: Color::from_rgb(0.35, 0.5, 0.4), // Blue-green leaves
            foliage: Some(FoliagePresetValues::eucalyptus()),
            growth: Some(GrowthPresetValues::spreading()),
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
        assert!(TreePreset::Maple.get_values().is_some());
        assert!(TreePreset::Spruce.get_values().is_some());
        assert!(TreePreset::Poplar.get_values().is_some());
        assert!(TreePreset::Baobab.get_values().is_some());
        assert!(TreePreset::DragonTree.get_values().is_some());
        assert!(TreePreset::JapaneseMaple.get_values().is_some());
        assert!(TreePreset::DeadTree.get_values().is_some());
        assert!(TreePreset::Redwood.get_values().is_some());
        // Existing species (added earlier)
        assert!(TreePreset::Elm.get_values().is_some());
        assert!(TreePreset::Fir.get_values().is_some());
        assert!(TreePreset::Cedar.get_values().is_some());
        assert!(TreePreset::JoshuaTree.get_values().is_some());
        assert!(TreePreset::Olive.get_values().is_some());
        // New species (Phase 2)
        assert!(TreePreset::CherryBlossom.get_values().is_some());
        assert!(TreePreset::Acacia.get_values().is_some());
        assert!(TreePreset::Beech.get_values().is_some());
        assert!(TreePreset::Ginkgo.get_values().is_some());
        assert!(TreePreset::WeepingCherry.get_values().is_some());
        // Fantasy presets (Phase 3)
        assert!(TreePreset::WorldTree.get_values().is_some());
        assert!(TreePreset::CrystalTree.get_values().is_some());
        assert!(TreePreset::CorruptedTree.get_values().is_some());
        assert!(TreePreset::GlowingTree.get_values().is_some());
        // Phase 4: New species
        assert!(TreePreset::BlueSpruce.get_values().is_some());
        assert!(TreePreset::DouglasFir.get_values().is_some());
        assert!(TreePreset::PonderosaPine.get_values().is_some());
        assert!(TreePreset::BristleconePine.get_values().is_some());
        assert!(TreePreset::Ash.get_values().is_some());
        assert!(TreePreset::Linden.get_values().is_some());
        assert!(TreePreset::Sycamore.get_values().is_some());
        assert!(TreePreset::Aspen.get_values().is_some());
        assert!(TreePreset::RoyalPalm.get_values().is_some());
        assert!(TreePreset::FanPalm.get_values().is_some());
        assert!(TreePreset::Eucalyptus.get_values().is_some());
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

    // New species presets tests
    #[test]
    fn test_elm_vase_shape() {
        let elm = TreePresetValues::elm();
        assert_eq!(elm.crown_shape, CrownShape::Hemispherical);
        assert!(elm.branch_angle_curve < 0.0); // Arches outward
        assert!(elm.apical_dominance < 0.4); // Low - vase-shaped
    }

    #[test]
    fn test_fir_symmetric_pyramid() {
        let fir = TreePresetValues::fir();
        assert_eq!(fir.crown_shape, CrownShape::Conical);
        assert_eq!(fir.phyllotaxis_angle, 72.0); // Whorled branching
        assert!(fir.apical_dominance > 0.8); // Strong central leader
    }

    #[test]
    fn test_cedar_flat_sprays() {
        let cedar = TreePresetValues::cedar();
        assert!(cedar.trunk_flare >= 1.5); // Buttressed base
        assert!(cedar.branch_flatness > 0.6); // Flat spray effect
    }

    #[test]
    fn test_joshua_tree_chaotic() {
        let joshua = TreePresetValues::joshua_tree();
        assert!(joshua.split_probability > 0.8); // High dichotomous forking
        assert!(joshua.branch_randomness > 0.6); // High randomness
        assert_eq!(joshua.sub_branch_count, 2); // Binary forking
    }

    #[test]
    fn test_olive_gnarled() {
        let olive = TreePresetValues::olive();
        assert!(olive.trunk_twist >= 50.0); // High twist
        assert!(olive.trunk_randomness > 0.3); // Gnarled appearance
    }

    // Style modifier tests
    #[test]
    fn test_style_modifier_none_returns_none() {
        assert!(StyleModifier::None.get_values().is_none());
    }

    #[test]
    fn test_style_modifiers_return_values() {
        assert!(StyleModifier::StylizedCartoon.get_values().is_some());
        assert!(StyleModifier::LowPoly.get_values().is_some());
        assert!(StyleModifier::Horror.get_values().is_some());
        assert!(StyleModifier::Realistic.get_values().is_some());
        assert!(StyleModifier::MobileOptimized.get_values().is_some());
        assert!(StyleModifier::AAARealistic.get_values().is_some());
    }

    #[test]
    fn test_stylized_cartoon_modifier() {
        let cartoon = StyleModifierValues::stylized_cartoon();
        assert!(cartoon.trunk_radius_mult > 1.0); // Thicker trunk
        assert!(cartoon.randomness_add < 0.0); // Smoother
        assert_eq!(cartoon.radial_segments, Some(6));
    }

    #[test]
    fn test_horror_modifier() {
        let horror = StyleModifierValues::horror();
        assert!(horror.randomness_add > 0.2);
        assert!(horror.twist_add > 20.0);
        assert!(horror.break_chance_add > 0.1);
    }

    #[test]
    fn test_mobile_optimized_modifier() {
        let mobile = StyleModifierValues::mobile_optimized();
        assert_eq!(mobile.radial_segments, Some(4)); // Minimum
        assert!(mobile.recursion_mult <= 0.5); // Half recursion
        assert!(mobile.foliage_density_mult <= 0.5); // Half foliage
        assert_eq!(mobile.smooth_iterations, 0); // No smoothing
    }

    #[test]
    fn test_aaa_realistic_modifier() {
        let aaa = StyleModifierValues::aaa_realistic();
        assert_eq!(aaa.radial_segments, Some(16)); // High poly
        assert!(aaa.recursion_mult >= 1.25); // More recursion
        assert!(aaa.foliage_density_mult >= 1.5); // Dense
        assert!(aaa.smooth_iterations >= 3); // Good smoothing
    }

    // Scale modifier tests
    #[test]
    fn test_scale_modifier_mature_returns_none() {
        assert!(ScaleModifier::Mature.get_values().is_none());
    }

    #[test]
    fn test_scale_modifiers_return_values() {
        assert!(ScaleModifier::Sapling.get_values().is_some());
        assert!(ScaleModifier::Ancient.get_values().is_some());
        assert!(ScaleModifier::Dead.get_values().is_some());
    }

    #[test]
    fn test_sapling_modifier() {
        let sapling = ScaleModifierValues::sapling();
        assert!(sapling.height_mult < 0.3); // Small
        assert!(sapling.radius_mult < 0.2); // Thin
        assert!(sapling.foliage_enabled);
    }

    #[test]
    fn test_ancient_modifier() {
        let ancient = ScaleModifierValues::ancient();
        assert!(ancient.height_mult > 1.0);
        assert!(ancient.radius_mult > 1.5);
        assert!(ancient.foliage_enabled);
    }

    #[test]
    fn test_dead_modifier() {
        let dead = ScaleModifierValues::dead();
        assert!(!dead.foliage_enabled); // No foliage
        assert!(dead.break_chance_add > 0.2); // Many broken branches
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 5: Season Modifier Tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_season_modifier_none_returns_none() {
        assert!(SeasonModifier::None.get_values().is_none());
    }

    #[test]
    fn test_season_modifiers_return_values() {
        assert!(SeasonModifier::Spring.get_values().is_some());
        assert!(SeasonModifier::Summer.get_values().is_some());
        assert!(SeasonModifier::Autumn.get_values().is_some());
        assert!(SeasonModifier::Winter.get_values().is_some());
    }

    #[test]
    fn test_spring_modifier() {
        let spring = SeasonModifierValues::spring();
        assert!(spring.foliage_density_mult < 1.0); // Less than summer
        assert!(spring.leaf_scale_mult < 1.0); // Smaller leaves
        assert!(spring.foliage_color.is_some()); // Custom color
        assert!(!spring.foliage_disabled);
    }

    #[test]
    fn test_summer_modifier() {
        let summer = SeasonModifierValues::summer();
        assert_eq!(summer.foliage_density_mult, 1.0); // Full foliage
        assert_eq!(summer.leaf_scale_mult, 1.0); // Normal leaves
        assert!(summer.foliage_color.is_none()); // Use preset color
        assert!(!summer.foliage_disabled);
    }

    #[test]
    fn test_autumn_modifier() {
        let autumn = SeasonModifierValues::autumn();
        assert!(autumn.foliage_density_mult < 1.0); // Less foliage
        assert!(autumn.foliage_color.is_some()); // Warm colors
        assert!(!autumn.foliage_disabled);
    }

    #[test]
    fn test_winter_modifier() {
        let winter = SeasonModifierValues::winter();
        assert_eq!(winter.foliage_density_mult, 0.0); // No foliage
        assert!(winter.snow_enabled); // Snow enabled
        assert!(winter.foliage_disabled); // Deciduous have no foliage
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 2: New Species Tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_cherry_blossom_ornamental() {
        let cherry = TreePresetValues::cherry_blossom();
        assert_eq!(cherry.crown_shape, CrownShape::Spreading);
        assert!(cherry.branch_flatness > 0.3); // Horizontal layered
        assert!(cherry.foliage.is_some());
    }

    #[test]
    fn test_acacia_umbrella_shape() {
        let acacia = TreePresetValues::acacia();
        assert_eq!(acacia.crown_shape, CrownShape::Umbrella);
        assert!(acacia.branch_angle > 85.0); // Near horizontal
        assert!(acacia.branch_flatness > 0.8); // Extreme horizontal
        assert!(acacia.apical_dominance < 0.2); // Very low
        assert!(acacia.branch_start > 0.6); // Crown only
    }

    #[test]
    fn test_beech_dense_crown() {
        let beech = TreePresetValues::beech();
        assert_eq!(beech.crown_shape, CrownShape::Spherical);
        assert!(beech.branch_density >= 3.0); // Dense
        assert!(beech.sub_branch_count >= 4);
    }

    #[test]
    fn test_ginkgo_angular() {
        let ginkgo = TreePresetValues::ginkgo();
        assert!(ginkgo.branch_randomness > 0.35); // High randomness
        assert!(ginkgo.up_attraction > 0.2); // Reaches upward
    }

    #[test]
    fn test_weeping_cherry_cascade() {
        let weeping = TreePresetValues::weeping_cherry();
        assert!(weeping.up_attraction < -0.5); // Strong downward
        assert!(weeping.gravity_strength > 0.7); // Strong weeping
        assert!(weeping.stiffness < 0.3); // Flexible
        assert!(weeping.branch_recursion >= 3); // Deep cascade
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 3: Fantasy Preset Tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_world_tree_massive_scale() {
        let world = TreePresetValues::world_tree();
        assert!(world.trunk_height >= 50.0); // Massive
        assert!(world.trunk_radius >= 10.0); // Giant trunk
        assert!(world.root_flare_height >= 0.3); // Tall root buttresses
        assert!(world.branch_density < 1.0); // Few massive branches
    }

    #[test]
    fn test_crystal_tree_geometric() {
        let crystal = TreePresetValues::crystal_tree();
        assert_eq!(crystal.radial_segments, 6); // Hexagonal
        assert_eq!(crystal.trunk_randomness, 0.0); // No randomness
        assert_eq!(crystal.branch_randomness, 0.0); // Pure geometry
        assert_eq!(crystal.gravity_strength, 0.0); // No gravity
        assert!(!crystal.branch_collar_enabled); // No organic collars
        assert!(crystal.foliage.is_none()); // No foliage
    }

    #[test]
    fn test_corrupted_tree_twisted() {
        let corrupted = TreePresetValues::corrupted_tree();
        assert!(corrupted.trunk_twist >= 60.0); // Extreme twist
        assert!(corrupted.break_chance >= 0.4); // Many broken branches
        assert!(corrupted.trunk_randomness > 0.3); // Irregular
        assert!(corrupted.foliage.is_some()); // Has dark foliage
    }

    #[test]
    fn test_glowing_tree_magical() {
        let glowing = TreePresetValues::glowing_tree();
        assert_eq!(glowing.crown_shape, CrownShape::Spherical);
        assert!(glowing.foliage.is_some()); // Has glowing foliage
                                            // Check foliage color is bright (emission-ready)
        if let Some(f) = &glowing.foliage {
            assert!(f.foliage_color.g > 0.8); // Bright green
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 4: New Species Tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_blue_spruce_conical() {
        let spruce = TreePresetValues::blue_spruce();
        assert_eq!(spruce.crown_shape, CrownShape::Conical);
        assert_eq!(spruce.phyllotaxis_angle, 72.0); // Whorled
        assert!(spruce.apical_dominance > 0.85);
    }

    #[test]
    fn test_douglas_fir_tiered() {
        let fir = TreePresetValues::douglas_fir();
        assert_eq!(fir.crown_shape, CrownShape::Conical);
        assert!(fir.branch_flatness > 0.5); // Tiered flat sprays
    }

    #[test]
    fn test_bristlecone_pine_ancient() {
        let pine = TreePresetValues::bristlecone_pine();
        assert!(pine.trunk_twist >= 90.0); // Extreme twist
        assert!(pine.break_chance >= 0.3); // Many dead branches
        assert!(pine.trunk_randomness > 0.25); // Gnarled
    }

    #[test]
    fn test_ash_opposite_branching() {
        let ash = TreePresetValues::ash();
        assert_eq!(ash.phyllotaxis_angle, 180.0); // Opposite
    }

    #[test]
    fn test_sycamore_buttressed() {
        let sycamore = TreePresetValues::sycamore();
        assert!(sycamore.trunk_flare >= 1.6); // Buttressed trunk
    }

    #[test]
    fn test_aspen_slender() {
        let aspen = TreePresetValues::aspen();
        assert!(aspen.trunk_radius <= 0.2); // Slender
        assert!(aspen.up_attraction > 0.3); // Upward reaching
    }

    #[test]
    fn test_royal_palm_straight() {
        let palm = TreePresetValues::royal_palm();
        assert!(palm.trunk_taper >= 0.99); // Almost no taper
        assert_eq!(palm.trunk_randomness, 0.0); // Perfectly straight
        assert!(palm.branch_start >= 0.9); // Only at crown
    }

    #[test]
    fn test_eucalyptus_pendulous() {
        let euc = TreePresetValues::eucalyptus();
        assert!(euc.up_attraction < 0.0); // Pendulous
        assert!(euc.gravity_strength >= 0.4); // Strong droop
    }

    // ═══════════════════════════════════════════════════════════════
    // Bonsai Style Tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_bonsai_style_none_returns_none() {
        assert!(BonsaiStyle::None.get_values().is_none());
    }

    #[test]
    fn test_bonsai_styles_return_values() {
        assert!(BonsaiStyle::Chokkan.get_values().is_some());
        assert!(BonsaiStyle::Moyogi.get_values().is_some());
        assert!(BonsaiStyle::Shakan.get_values().is_some());
        assert!(BonsaiStyle::Kengai.get_values().is_some());
        assert!(BonsaiStyle::Fukinagashi.get_values().is_some());
        assert!(BonsaiStyle::Bunjingi.get_values().is_some());
    }

    #[test]
    fn test_chokkan_formal_upright() {
        let chokkan = BonsaiStyleValues::chokkan();
        assert_eq!(chokkan.trunk_twist, 0.0); // Straight trunk
        assert!(chokkan.up_attraction > 0.8); // Strong upward
        assert!(chokkan.stiffness > 0.85); // Rigid
        assert!(chokkan.branch_randomness < 0.1); // Minimal
    }

    #[test]
    fn test_moyogi_informal_upright() {
        let moyogi = BonsaiStyleValues::moyogi();
        assert!(moyogi.trunk_twist > 30.0); // S-curved
        assert!(moyogi.branch_randomness > 0.15); // Natural
    }

    #[test]
    fn test_kengai_cascade() {
        let kengai = BonsaiStyleValues::kengai();
        assert!(kengai.up_attraction < 0.0); // Downward
        assert!(kengai.gravity_strength >= 1.0); // Strong cascade
        assert!(kengai.stiffness < 0.3); // Very flexible
    }

    #[test]
    fn test_fukinagashi_windswept() {
        let fuki = BonsaiStyleValues::fukinagashi();
        assert!(fuki.branch_direction_bias >= 1.0); // One-sided
    }

    #[test]
    fn test_bunjingi_literati() {
        let bunj = BonsaiStyleValues::bunjingi();
        assert!(bunj.branch_start >= 0.8); // Top branches only
        assert!(bunj.branch_density_mult < 0.4); // Very sparse
        assert!(bunj.trunk_taper > 0.6); // Strong taper
    }
}
