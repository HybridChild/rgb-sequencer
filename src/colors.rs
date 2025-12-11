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
