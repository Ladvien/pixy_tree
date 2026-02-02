use godot::prelude::*;
use std::f32::consts::PI;

/// Crown shape envelope that modulates branch length based on height position
#[derive(GodotConvert, Var, Export, Default, Clone, Copy, Debug, PartialEq)]
#[godot(via = i64)]
pub enum CrownShape {
    #[default]
    Cylindrical = 0,
    Conical = 1,
    Spherical = 2,
    Hemispherical = 3,
    TaperedCylindrical = 4,
    Flame = 5,
    InverseConical = 6,
    TendFlame = 7,
    Spreading = 8, // Widest at ~40% height, gradual taper - for JapaneseMaple, Oak variants
}

impl CrownShape {
    /// Get the branch length multiplier based on normalized height (0.0 = bottom, 1.0 = top)
    pub fn get_length_multiplier(&self, height_ratio: f32) -> f32 {
        let t = height_ratio.clamp(0.0, 1.0);

        match self {
            // Uniform length throughout (current behavior)
            CrownShape::Cylindrical => 1.0,

            // Christmas tree / pine - longer at top, shorter at bottom
            CrownShape::Conical => 0.2 + 0.8 * t,

            // Oak/maple - longest at middle, shorter at top and bottom
            CrownShape::Spherical => 0.2 + 0.8 * (PI * t).sin(),

            // Like spherical but only the top half of the sine curve
            CrownShape::Hemispherical => 0.2 + 0.8 * (PI * 0.5 * t).sin(),

            // Gradual taper from bottom to top
            CrownShape::TaperedCylindrical => 1.0 - 0.5 * t,

            // Cypress - peaked at 70% height, then falloff
            CrownShape::Flame => {
                let peak = 0.7;
                if t < peak {
                    0.3 + 0.7 * (t / peak)
                } else {
                    1.0 - 0.8 * ((t - peak) / (1.0 - peak))
                }
            }

            // Inverse of conical - longer at bottom, shorter at top
            CrownShape::InverseConical => 1.0 - 0.8 * t,

            // Similar to flame but more gradual falloff
            CrownShape::TendFlame => {
                let peak = 0.6;
                if t < peak {
                    0.4 + 0.6 * (t / peak)
                } else {
                    let falloff = (t - peak) / (1.0 - peak);
                    1.0 - 0.6 * falloff * falloff
                }
            }

            // Spreading - widest at 40% height, gradual taper above and below
            // Good for Japanese Maple, ornamental trees with layered horizontal branches
            CrownShape::Spreading => {
                let peak = 0.4;
                if t < peak {
                    // Gradual increase to peak
                    0.5 + 0.5 * (t / peak)
                } else {
                    // Gentle taper after peak
                    let falloff = (t - peak) / (1.0 - peak);
                    1.0 - 0.4 * falloff
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cylindrical_uniform() {
        let shape = CrownShape::Cylindrical;
        assert_eq!(shape.get_length_multiplier(0.0), 1.0);
        assert_eq!(shape.get_length_multiplier(0.5), 1.0);
        assert_eq!(shape.get_length_multiplier(1.0), 1.0);
    }

    #[test]
    fn test_conical_increases_with_height() {
        let shape = CrownShape::Conical;
        let bottom = shape.get_length_multiplier(0.0);
        let middle = shape.get_length_multiplier(0.5);
        let top = shape.get_length_multiplier(1.0);

        assert!(bottom < middle);
        assert!(middle < top);
        assert!((bottom - 0.2).abs() < 0.001);
        assert!((top - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_spherical_peaks_in_middle() {
        let shape = CrownShape::Spherical;
        let bottom = shape.get_length_multiplier(0.0);
        let middle = shape.get_length_multiplier(0.5);
        let top = shape.get_length_multiplier(1.0);

        assert!(middle > bottom);
        assert!(middle > top);
    }

    #[test]
    fn test_flame_peaks_at_70_percent() {
        let shape = CrownShape::Flame;
        let at_peak = shape.get_length_multiplier(0.7);
        let before_peak = shape.get_length_multiplier(0.5);
        let after_peak = shape.get_length_multiplier(0.9);

        assert!(at_peak > before_peak);
        assert!(at_peak > after_peak);
    }

    #[test]
    fn test_inverse_conical_decreases_with_height() {
        let shape = CrownShape::InverseConical;
        let bottom = shape.get_length_multiplier(0.0);
        let top = shape.get_length_multiplier(1.0);

        assert!(bottom > top);
        assert!((bottom - 1.0).abs() < 0.001);
        assert!((top - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_clamping() {
        let shape = CrownShape::Conical;
        // Values outside 0-1 should be clamped
        assert_eq!(
            shape.get_length_multiplier(-0.5),
            shape.get_length_multiplier(0.0)
        );
        assert_eq!(
            shape.get_length_multiplier(1.5),
            shape.get_length_multiplier(1.0)
        );
    }

    #[test]
    fn test_spreading_peaks_at_40_percent() {
        let shape = CrownShape::Spreading;
        let at_peak = shape.get_length_multiplier(0.4);
        let before_peak = shape.get_length_multiplier(0.2);
        let after_peak = shape.get_length_multiplier(0.7);
        let at_top = shape.get_length_multiplier(1.0);

        // Peak should be at 0.4
        assert!(at_peak > before_peak);
        assert!(at_peak > after_peak);
        // Should taper toward top
        assert!(after_peak > at_top);
        // Peak value should be 1.0
        assert!((at_peak - 1.0).abs() < 0.001);
    }
}
