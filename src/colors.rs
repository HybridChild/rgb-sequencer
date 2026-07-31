//! Color space conversion helpers.
//!
//! Provides convenience functions for working with HSV (Hue, Saturation, Value),
//! which is more intuitive than RGB for many LED animations like color wheels and
//! hue rotations.
//!
//! All functions are `const` and use integer arithmetic only.

use crate::color::Rgb;

/// Converts degrees to the full-circle hue units used by [`hsv`] and [`hue`].
///
/// Values of 360 and above wrap around the wheel.
#[inline]
pub const fn degrees(deg: u16) -> u16 {
    ((deg as u32 % 360) * 65536 / 360) as u16
}

/// Creates an RGB color from HSV components.
///
/// `hue` spans the full wheel across the whole `u16` range, so rotation is plain
/// addition and wraps for free; see [`degrees`] to convert from degrees.
/// `saturation` and `value` run `0..=65535`.
///
/// Only red falls on a representable sector boundary, so hues such as 120° land
/// within a few parts per 65535 of the exact primary rather than on it.
#[inline]
pub const fn hsv(hue: u16, saturation: u16, value: u16) -> Rgb {
    let v = value as u32;

    // Chroma: the span between the brightest and dimmest channel.
    let c = v * saturation as u32 / 65535;
    // Minimum channel value, added back to every channel at the end.
    let m = v - c;

    // Multiplying by 6 puts the sector index in the high bits and the position within
    // the sector in the low 16, so both fall out with a shift and a mask - no division,
    // and no truncation error from a rounded-off sector width.
    let h6 = hue as u32 * 6;
    let sector = h6 >> 16;
    let rising = h6 & 0xFFFF;
    let falling = 65535 - rising;

    // The second-largest channel ramps linearly across each sector.
    let x_up = c * rising / 65535;
    let x_down = c * falling / 65535;

    let (r, g, b) = match sector {
        0 => (c, x_up, 0),
        1 => (x_down, c, 0),
        2 => (0, c, x_up),
        3 => (0, x_down, c),
        4 => (x_up, 0, c),
        _ => (c, 0, x_down),
    };

    Rgb::new((r + m) as u16, (g + m) as u16, (b + m) as u16)
}

/// Creates a fully saturated, full-brightness RGB color from hue alone.
#[inline]
pub const fn hue(hue: u16) -> Rgb {
    hsv(hue, 65535, 65535)
}
