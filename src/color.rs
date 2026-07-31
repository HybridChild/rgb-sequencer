//! Fixed-point RGB color type.

use crate::fixed::{lerp_channel, scale_channel};

/// An RGB color with 16 bits per channel.
///
/// Channels are normalized values, not hardware values. Convert them to the native
/// format in [`RgbLed::set_color`](crate::RgbLed::set_color).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Rgb {
    /// Red channel, `0..=65535`.
    pub r: u16,

    /// Green channel, `0..=65535`.
    pub g: u16,

    /// Blue channel, `0..=65535`.
    pub b: u16,
}

impl Rgb {
    /// Creates a color from 16-bit channel values.
    #[inline]
    pub const fn new(r: u16, g: u16, b: u16) -> Self {
        Self { r, g, b }
    }

    /// Creates a color from 8-bit channel values, expanding 255 to exactly 65535.
    #[inline]
    pub const fn from_u8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as u16 * 257,
            g: g as u16 * 257,
            b: b as u16 * 257,
        }
    }

    /// Reduces the color to 8 bits per channel, for drivers such as WS2812.
    #[inline]
    pub const fn to_u8(self) -> (u8, u8, u8) {
        (
            (self.r >> 8) as u8,
            (self.g >> 8) as u8,
            (self.b >> 8) as u8,
        )
    }

    /// Interpolates toward `other` by a Q0.15 fraction, where 32768 lands on `other`.
    ///
    /// Values above 32768 are not meaningful and are not clamped.
    #[inline]
    pub const fn lerp(self, other: Self, t: u16) -> Self {
        Self {
            r: lerp_channel(self.r, other.r, t),
            g: lerp_channel(self.g, other.g, t),
            b: lerp_channel(self.b, other.b, t),
        }
    }

    /// Scales every channel by an 8-bit factor, where 255 leaves the color unchanged.
    #[inline]
    pub const fn scale(self, factor: u8) -> Self {
        Self {
            r: scale_channel(self.r, factor),
            g: scale_channel(self.g, factor),
            b: scale_channel(self.b, factor),
        }
    }

    /// Returns true if every channel is within `epsilon` of `other`'s.
    #[inline]
    pub const fn approx_eq(self, other: Self, epsilon: u16) -> bool {
        self.r.abs_diff(other.r) < epsilon
            && self.g.abs_diff(other.g) < epsilon
            && self.b.abs_diff(other.b) < epsilon
    }
}
