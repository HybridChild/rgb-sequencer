//! Color space conversion helpers.
//!
//! Provides convenience functions for working with different color spaces,
//! particularly HSV (Hue, Saturation, Value) which is more intuitive for
//! many LED animations like color wheels and hue rotations.
//!
//! All functions return `palette::Srgb` for direct use with RGB sequences.

use palette::{FromColor, Hsv, Srgb};

/// Creates an RGB color from HSV (Hue, Saturation, Value) components.
#[inline]
pub fn hsv(hue: f32, saturation: f32, value: f32) -> Srgb {
    let hsv = Hsv::new(hue, saturation, value);
    Srgb::from_color(hsv)
}

/// Creates an RGB color from hue only (full saturation and value).
#[inline]
pub fn hue(hue: f32) -> Srgb {
    hsv(hue, 1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colors_equal(a: Srgb, b: Srgb) -> bool {
        const EPSILON: f32 = 0.01;
        (a.red - b.red).abs() < EPSILON
            && (a.green - b.green).abs() < EPSILON
            && (a.blue - b.blue).abs() < EPSILON
    }

    #[test]
    fn hsv_creates_primary_colors() {
        // Red (hue = 0)
        let red = hsv(0.0, 1.0, 1.0);
        assert!(colors_equal(red, Srgb::new(1.0, 0.0, 0.0)));

        // Green (hue = 120)
        let green = hsv(120.0, 1.0, 1.0);
        assert!(colors_equal(green, Srgb::new(0.0, 1.0, 0.0)));

        // Blue (hue = 240)
        let blue = hsv(240.0, 1.0, 1.0);
        assert!(colors_equal(blue, Srgb::new(0.0, 0.0, 1.0)));
    }

    #[test]
    fn hsv_handles_saturation() {
        // Full saturation
        let full = hsv(0.0, 1.0, 1.0);
        assert!(full.red > 0.99);

        // Zero saturation (gray)
        let gray = hsv(0.0, 0.0, 0.5);
        assert!(colors_equal(gray, Srgb::new(0.5, 0.5, 0.5)));
    }

    #[test]
    fn hsv_handles_value() {
        // Full value
        let bright = hsv(0.0, 1.0, 1.0);
        assert!(bright.red > 0.99);

        // Half value
        let dim = hsv(0.0, 1.0, 0.5);
        assert!(dim.red > 0.49 && dim.red < 0.51);

        // Zero value (black)
        let black = hsv(0.0, 1.0, 0.0);
        assert!(colors_equal(black, Srgb::new(0.0, 0.0, 0.0)));
    }

    #[test]
    fn hue_creates_fully_saturated_colors() {
        let red = hue(0.0);
        assert!(colors_equal(red, Srgb::new(1.0, 0.0, 0.0)));

        let cyan = hue(180.0);
        assert!(colors_equal(cyan, Srgb::new(0.0, 1.0, 1.0)));

        let yellow = hue(60.0);
        assert!(colors_equal(yellow, Srgb::new(1.0, 1.0, 0.0)));
    }

    #[test]
    fn hue_wraps_around_360() {
        // Hue should wrap, so 360 == 0
        let red1 = hue(0.0);
        let red2 = hue(360.0);
        assert!(colors_equal(red1, red2));
    }
}
