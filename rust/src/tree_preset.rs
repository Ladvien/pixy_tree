use godot::prelude::*;

use crate::crown_shape::CrownShape;
use crate::foliage::FoliagePresetValues;
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

    // Branch Collar
    pub branch_collar_enabled: bool,
    pub branch_collar_length: f32,

    // Crown
    pub crown_shape: CrownShape,
    pub crown_influence: f32,

    // Materials
    pub trunk_color: Color,
    pub foliage_color: Color,

    // Foliage
    pub foliage: Option<FoliagePresetValues>,
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
            branch_density: 1.2,
            branch_length: 0.5,
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
            gravity_strength: 0.1,
            stiffness: 0.6,
            break_chance: 0.05,
            split_enabled: true,
            split_probability: 0.3,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.1,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.9,
            trunk_color: Color::from_rgb(0.365, 0.227, 0.102), // #5D3A1A Dark Brown
            foliage_color: Color::from_rgb(0.176, 0.314, 0.086), // #2D5016 Dark Green
            foliage: Some(FoliagePresetValues::oak()),
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
            branch_start: 0.15,
            branch_end: 0.95,
            branch_density: 1.8,
            branch_length: 0.45,
            branch_angle: 70.0,
            branch_radius_ratio: 0.25,
            branch_taper: 0.8,
            phyllotaxis_angle: 137.5,
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
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Conical,
            crown_influence: 1.0,
            trunk_color: Color::from_rgb(0.420, 0.267, 0.137), // #6B4423 Reddish Brown
            foliage_color: Color::from_rgb(0.106, 0.302, 0.243), // #1B4D3E Pine Green
            foliage: Some(FoliagePresetValues::pine()),
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
            branch_density: 1.5,
            branch_length: 0.7,
            branch_angle: 50.0,
            branch_radius_ratio: 0.2,
            branch_taper: 0.85,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.3,
            up_attraction: -0.5,
            branch_recursion: 2,
            sub_branch_count: 3,
            sub_branch_scale: 0.6,
            branch_length_variation: 0.25,
            sub_branch_position_bias: 0.3,
            apical_dominance: 0.2,
            branch_flatness: 0.2,
            branch_angle_curve: -0.4,
            crown_angle_variation: -0.3, // Strong horizontal spread at bottom
            trunk_twist: 10.0,
            branch_twist: 15.0,
            gravity_strength: 0.4,
            stiffness: 0.2,
            break_chance: 0.0,
            split_enabled: false,
            split_probability: 0.0,
            split_angle: 30.0,
            split_position: 0.5,
            split_radius_threshold: 0.05,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Hemispherical,
            crown_influence: 0.8,
            trunk_color: Color::from_rgb(0.478, 0.361, 0.239), // #7A5C3D Grayish Brown
            foliage_color: Color::from_rgb(0.565, 0.690, 0.376), // #90B060 Yellow-Green
            foliage: Some(FoliagePresetValues::willow()),
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
            branch_density: 1.0,
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
            apical_dominance: 0.6,
            branch_flatness: 0.1,
            branch_angle_curve: 0.2,
            crown_angle_variation: 0.1, // Slight vertical bias at bottom
            trunk_twist: 5.0,
            branch_twist: 8.0,
            gravity_strength: 0.05,
            stiffness: 0.7,
            break_chance: 0.1,
            split_enabled: true,
            split_probability: 0.2,
            split_angle: 25.0,
            split_position: 0.5,
            split_radius_threshold: 0.05,
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::TaperedCylindrical,
            crown_influence: 0.7,
            trunk_color: Color::from_rgb(0.961, 0.961, 0.863), // #F5F5DC Beige/White
            foliage_color: Color::from_rgb(0.486, 0.804, 0.486), // #7CCD7C Light Green
            foliage: Some(FoliagePresetValues::birch()),
        }
    }

    /// Palm: Branches only at top, no sub-branches
    pub fn palm() -> Self {
        Self {
            trunk_height: 6.0,
            trunk_radius: 0.35,
            radial_segments: 8,
            height_segments: 6,
            trunk_taper: 0.5,
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
            branch_radius_ratio: 0.15,
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
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Cylindrical,
            crown_influence: 0.5,
            trunk_color: Color::from_rgb(0.545, 0.451, 0.333), // #8B7355 Tan Brown
            foliage_color: Color::from_rgb(0.133, 0.545, 0.133), // #228B22 Forest Green
            foliage: Some(FoliagePresetValues::palm()),
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
            branch_density: 2.0,
            branch_length: 0.25,
            branch_angle: 25.0,
            branch_radius_ratio: 0.2,
            branch_taper: 0.7,
            phyllotaxis_angle: 137.5,
            branch_randomness: 0.1,
            up_attraction: 0.6,
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
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Flame,
            crown_influence: 1.0,
            trunk_color: Color::from_rgb(0.290, 0.235, 0.165), // #4A3C2A Dark Olive
            foliage_color: Color::from_rgb(0.208, 0.369, 0.231), // #355E3B Hunter Green
            foliage: Some(FoliagePresetValues::cypress()),
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
            branch_radius_ratio: 0.4,
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
            branch_collar_enabled: true,
            branch_collar_length: 1.5,
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.6,
            trunk_color: Color::from_rgb(0.361, 0.251, 0.200), // #5C4033 Dark Brown
            foliage_color: Color::from_rgb(0.208, 0.369, 0.231), // #355E3B Hunter Green
            foliage: Some(FoliagePresetValues::bonsai()),
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
}
