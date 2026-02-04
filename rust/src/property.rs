//! PropertyWrapper System
//!
//! Polymorphic property evaluation for branch parameters.
//! Supports Constant, Random, and Curve evaluation modes.
//! Evaluated with position along parent branch (0.0 = base, 1.0 = tip).

use crate::branch::SeededRng;
use godot::prelude::*;

/// Property evaluation mode
#[derive(Clone, Debug)]
pub enum PropertyValue {
    /// Fixed constant value
    Constant(f32),
    /// Random value between min and max (evaluated fresh each call)
    Random { min: f32, max: f32 },
    /// Curve mapping: maps input x ∈ [x_min,x_max] to output ∈ [y_min, y_max] with power curve
    /// C3 fix: Added x_min/x_max for input range remapping (matching C++ SimpleCurveProperty)
    Curve {
        y_min: f32,
        y_max: f32,
        power: f32,
        x_min: f32,
        x_max: f32,
    },
}

impl PropertyValue {
    /// Evaluate the property at position x
    ///
    /// # Arguments
    /// * `x` - Position along parent branch (0.0 = base, 1.0 = tip for default range)
    /// * `rng` - Random number generator (used only for Random mode)
    ///
    /// C3 fix: For Curve mode, x is remapped from [x_min, x_max] to [0, 1] before evaluation.
    pub fn evaluate(&self, x: f32, rng: &mut SeededRng) -> f32 {
        match self {
            PropertyValue::Constant(value) => *value,
            PropertyValue::Random { min, max } => rng.range(*min, *max),
            PropertyValue::Curve {
                y_min,
                y_max,
                power,
                x_min,
                x_max,
            } => {
                // C3 fix: Remap input from [x_min, x_max] to [0, 1]
                let range = (*x_max - *x_min).max(0.001);
                let normalized_x = ((x - *x_min) / range).clamp(0.0, 1.0);
                let factor = normalized_x.powf(*power);
                y_min + (y_max - y_min) * factor
            }
        }
    }

    /// Evaluate without randomness (uses midpoint for Random mode)
    pub fn evaluate_deterministic(&self, x: f32) -> f32 {
        match self {
            PropertyValue::Constant(value) => *value,
            PropertyValue::Random { min, max } => (min + max) * 0.5,
            PropertyValue::Curve {
                y_min,
                y_max,
                power,
                x_min,
                x_max,
            } => {
                // C3 fix: Remap input from [x_min, x_max] to [0, 1]
                let range = (*x_max - *x_min).max(0.001);
                let normalized_x = ((x - *x_min) / range).clamp(0.0, 1.0);
                let factor = normalized_x.powf(*power);
                y_min + (y_max - y_min) * factor
            }
        }
    }
}

impl Default for PropertyValue {
    fn default() -> Self {
        PropertyValue::Constant(1.0)
    }
}

/// Property wrapper that can be used for branch parameters.
/// Combines a base value with an evaluation mode.
#[derive(Clone, Debug)]
pub struct BranchProperty {
    /// The evaluation mode
    pub mode: PropertyMode,
    /// Base value (used in Constant mode, or as scale in Curve mode)
    pub base_value: f32,
    /// Curve power (only used in Curve mode, 1.0 = linear)
    pub curve_power: f32,
    /// End value for curve (fraction of base_value at tip)
    pub curve_end_ratio: f32,
    /// Variation range for random mode (as fraction of base_value)
    pub random_variation: f32,
    /// C3 fix: Input range minimum (only used in Curve mode, default 0.0)
    pub x_min: f32,
    /// C3 fix: Input range maximum (only used in Curve mode, default 1.0)
    pub x_max: f32,
}

/// H7: Property evaluation mode selector - exportable to Godot UI
#[derive(GodotConvert, Var, Export, Clone, Copy, Debug, PartialEq, Default)]
#[godot(via = i64)]
pub enum PropertyMode {
    /// Use base_value directly
    #[default]
    Constant = 0,
    /// Vary randomly around base_value
    Random = 1,
    /// Curve from base_value to base_value * curve_end_ratio along parent
    Curve = 2,
}

impl BranchProperty {
    /// Create a constant property
    pub fn constant(value: f32) -> Self {
        Self {
            mode: PropertyMode::Constant,
            base_value: value,
            curve_power: 1.0,
            curve_end_ratio: 1.0,
            random_variation: 0.0,
            x_min: 0.0,
            x_max: 1.0,
        }
    }

    /// Create a curve property
    pub fn curve(base_value: f32, end_ratio: f32, power: f32) -> Self {
        Self {
            mode: PropertyMode::Curve,
            base_value,
            curve_power: power,
            curve_end_ratio: end_ratio,
            random_variation: 0.0,
            x_min: 0.0,
            x_max: 1.0,
        }
    }

    /// Create a curve property with custom input range (C3 fix)
    pub fn curve_with_range(
        base_value: f32,
        end_ratio: f32,
        power: f32,
        x_min: f32,
        x_max: f32,
    ) -> Self {
        Self {
            mode: PropertyMode::Curve,
            base_value,
            curve_power: power,
            curve_end_ratio: end_ratio,
            random_variation: 0.0,
            x_min,
            x_max,
        }
    }

    /// Evaluate the property at position x along parent branch
    /// C3 fix: For Curve mode, x is remapped from [x_min, x_max] to [0, 1]
    pub fn evaluate(&self, x: f32, rng: &mut SeededRng) -> f32 {
        match self.mode {
            PropertyMode::Constant => self.base_value,
            PropertyMode::Random => {
                let variation = self.base_value * self.random_variation;
                rng.range(self.base_value - variation, self.base_value + variation)
            }
            PropertyMode::Curve => {
                // C3 fix: Remap input from [x_min, x_max] to [0, 1]
                let range = (self.x_max - self.x_min).max(0.001);
                let normalized_x = ((x - self.x_min) / range).clamp(0.0, 1.0);
                let factor = normalized_x.powf(self.curve_power);
                let end_value = self.base_value * self.curve_end_ratio;
                self.base_value + (end_value - self.base_value) * factor
            }
        }
    }
}

impl Default for BranchProperty {
    fn default() -> Self {
        Self {
            mode: PropertyMode::Constant,
            base_value: 1.0,
            curve_power: 1.0,
            curve_end_ratio: 1.0,
            random_variation: 0.0,
            x_min: 0.0,
            x_max: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_property() {
        let mut rng = SeededRng::new(42);
        let prop = PropertyValue::Constant(5.0);
        assert_eq!(prop.evaluate(0.0, &mut rng), 5.0);
        assert_eq!(prop.evaluate(0.5, &mut rng), 5.0);
        assert_eq!(prop.evaluate(1.0, &mut rng), 5.0);
    }

    #[test]
    fn test_random_property() {
        let mut rng = SeededRng::new(42);
        let prop = PropertyValue::Random { min: 0.0, max: 1.0 };
        let val = prop.evaluate(0.5, &mut rng);
        assert!(val >= 0.0 && val <= 1.0);
    }

    #[test]
    fn test_curve_property_linear() {
        let mut rng = SeededRng::new(42);
        let prop = PropertyValue::Curve {
            y_min: 1.0,
            y_max: 0.0,
            power: 1.0,
            x_min: 0.0,
            x_max: 1.0,
        };
        assert!((prop.evaluate(0.0, &mut rng) - 1.0).abs() < 0.001);
        assert!((prop.evaluate(0.5, &mut rng) - 0.5).abs() < 0.001);
        assert!((prop.evaluate(1.0, &mut rng) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_curve_property_power() {
        let mut rng = SeededRng::new(42);
        let prop = PropertyValue::Curve {
            y_min: 1.0,
            y_max: 0.0,
            power: 2.0,
            x_min: 0.0,
            x_max: 1.0,
        };
        // At x=0.5 with power 2: factor = 0.25, result = 1.0 + (0.0 - 1.0) * 0.25 = 0.75
        assert!((prop.evaluate(0.5, &mut rng) - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_branch_property_constant() {
        let mut rng = SeededRng::new(42);
        let prop = BranchProperty::constant(3.0);
        assert_eq!(prop.evaluate(0.0, &mut rng), 3.0);
        assert_eq!(prop.evaluate(1.0, &mut rng), 3.0);
    }

    #[test]
    fn test_branch_property_curve() {
        let mut rng = SeededRng::new(42);
        // Branch length starts at 2.0, ends at 0.2 (end_ratio=0.1), linear
        let prop = BranchProperty::curve(2.0, 0.1, 1.0);
        assert!((prop.evaluate(0.0, &mut rng) - 2.0).abs() < 0.001);
        // At x=1.0: 2.0 + (0.2 - 2.0) * 1.0 = 0.2
        assert!((prop.evaluate(1.0, &mut rng) - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_deterministic_random() {
        let prop = PropertyValue::Random { min: 2.0, max: 4.0 };
        assert!((prop.evaluate_deterministic(0.5) - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_curve_property_with_range() {
        // C3 test: x_min/x_max range remapping
        let mut rng = SeededRng::new(42);
        // Input range [0.3, 0.7] maps to internal [0, 1]
        let prop = PropertyValue::Curve {
            y_min: 0.0,
            y_max: 1.0,
            power: 1.0,
            x_min: 0.3,
            x_max: 0.7,
        };
        // At x=0.3 (x_min), should return y_min = 0.0
        assert!((prop.evaluate(0.3, &mut rng) - 0.0).abs() < 0.001);
        // At x=0.7 (x_max), should return y_max = 1.0
        assert!((prop.evaluate(0.7, &mut rng) - 1.0).abs() < 0.001);
        // At x=0.5 (midpoint), should return 0.5
        assert!((prop.evaluate(0.5, &mut rng) - 0.5).abs() < 0.001);
        // Values outside range should clamp
        assert!((prop.evaluate(0.0, &mut rng) - 0.0).abs() < 0.001);
        assert!((prop.evaluate(1.0, &mut rng) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_branch_property_curve_with_range() {
        // C3 test: BranchProperty with custom input range
        let mut rng = SeededRng::new(42);
        let prop = BranchProperty::curve_with_range(1.0, 0.5, 1.0, 0.2, 0.8);
        // At x=0.2 (x_min), should return base_value = 1.0
        assert!((prop.evaluate(0.2, &mut rng) - 1.0).abs() < 0.001);
        // At x=0.8 (x_max), should return base_value * end_ratio = 0.5
        assert!((prop.evaluate(0.8, &mut rng) - 0.5).abs() < 0.001);
        // At x=0.5 (midpoint), should return 0.75
        assert!((prop.evaluate(0.5, &mut rng) - 0.75).abs() < 0.001);
    }
}
