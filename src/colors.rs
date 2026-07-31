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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BLACK, BLUE, GREEN, RED, WHITE};

    /// Channels agree to within roughly 0.2% of full scale.
    fn close(a: Rgb, b: Rgb) -> bool {
        a.approx_eq(b, 128)
    }

    #[test]
    fn red_is_exact_and_other_primaries_are_within_a_hair() {
        // Hue 0 is the one sector boundary that lands exactly on a representable
        // value; the rest are off by a part or two in 65535, which vanishes at 8 bits.
        assert_eq!(hue(0), RED);
        assert!(close(hue(degrees(120)), GREEN));
        assert!(close(hue(degrees(240)), BLUE));
        assert_eq!(hue(degrees(120)).to_u8(), (0, 255, 0));
        assert_eq!(hue(degrees(240)).to_u8(), (0, 0, 255));
    }

    #[test]
    fn secondary_hues_are_close() {
        assert!(close(hue(degrees(60)), Rgb::new(65535, 65535, 0)), "yellow");
        assert!(close(hue(degrees(180)), Rgb::new(0, 65535, 65535)), "cyan");
        assert!(
            close(hue(degrees(300)), Rgb::new(65535, 0, 65535)),
            "magenta"
        );
    }

    #[test]
    fn zero_saturation_is_greyscale() {
        assert_eq!(hsv(0, 0, 65535), WHITE);
        assert_eq!(hsv(30000, 0, 65535), WHITE);
        assert_eq!(hsv(12345, 0, 32768), Rgb::new(32768, 32768, 32768));
    }

    #[test]
    fn zero_value_is_black() {
        for h in [0u16, 10000, 30000, 50000, 65535] {
            assert_eq!(hsv(h, 65535, 0), BLACK, "hue {h}");
        }
    }

    #[test]
    fn value_scales_every_channel() {
        assert!(close(hsv(0, 65535, 32768), Rgb::new(32768, 0, 0)));
        assert!(close(
            hsv(degrees(180), 65535, 32768),
            Rgb::new(0, 32768, 32768)
        ));
    }

    #[test]
    fn degrees_wraps_at_full_circle() {
        assert_eq!(degrees(0), degrees(360));
        assert_eq!(degrees(90), degrees(450));
        assert_eq!(hue(degrees(360)), RED);
    }

    #[test]
    fn every_hue_keeps_one_channel_full_and_one_empty() {
        // A fully saturated, full-value color always has one channel at full scale
        // and one at zero. This catches sector-boundary arithmetic errors.
        let mut h = 0u32;
        while h <= 65535 {
            let c = hue(h as u16);
            let max = if c.r >= c.g && c.r >= c.b {
                c.r
            } else if c.g >= c.b {
                c.g
            } else {
                c.b
            };
            let min = if c.r <= c.g && c.r <= c.b {
                c.r
            } else if c.g <= c.b {
                c.g
            } else {
                c.b
            };
            assert!(max >= 65535 - 128, "hue {h} peaked at only {max}");
            assert!(min <= 128, "hue {h} bottomed out at {min}");
            h += 7;
        }
    }

    #[test]
    fn hue_rotation_wraps_without_modulo() {
        // Adding to a u16 hue wraps the wheel naturally, with no range check.
        let start = 60000u16;
        let rotated = start.wrapping_add(10000);
        assert!(rotated < start);
        assert_ne!(hue(rotated), BLACK);
    }
}
