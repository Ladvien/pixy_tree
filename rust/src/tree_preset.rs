use godot::prelude::*;

use crate::crown_shape::CrownShape;
use crate::foliage::FoliagePresetValues;

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

    // Crown
    pub crown_shape: CrownShape,
    pub crown_influence: f32,

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
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.9,
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
            crown_shape: CrownShape::Conical,
            crown_influence: 1.0,
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
            crown_shape: CrownShape::Hemispherical,
            crown_influence: 0.8,
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
            crown_shape: CrownShape::TaperedCylindrical,
            crown_influence: 0.7,
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
            crown_shape: CrownShape::Cylindrical,
            crown_influence: 0.5,
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
            crown_shape: CrownShape::Flame,
            crown_influence: 1.0,
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
            crown_shape: CrownShape::Spherical,
            crown_influence: 0.6,
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
