//! Fixed-point arithmetic primitives.
//!
//! Progress and easing use Q0.15: an unsigned 15-bit fraction in a `u16`, where
//! [`ONE`] represents 1.0. Q0.15 rather than Q0.16 so that squaring stays inside a
//! `u32` (`32768^2` is 2^30; `65536^2` is not). Every intermediate here is 32-bit,
//! so no 64-bit helper from `compiler_builtins` is ever linked in.

/// 1.0 in Q0.15 format.
pub(crate) const ONE: u16 = 32768;

/// 0.5 in Q0.15 format.
pub(crate) const HALF: u16 = ONE / 2;

/// Fractional bit count of the Q0.15 format.
pub(crate) const SHIFT: u32 = 15;

/// Bit width the operands of [`progress_q15`] are reduced to before dividing.
///
/// The widest numerator whose 15-bit shift still fits a `u32`.
const OPERAND_BITS: u32 = 17;

/// Multiplies two Q0.15 values. Both operands must be in `0..=ONE`; the result is too.
#[inline]
pub(crate) const fn mul_q15(a: u16, b: u16) -> u16 {
    ((a as u32 * b as u32) >> SHIFT) as u16
}

/// Computes elapsed-within-step progress as a Q0.15 fraction, saturating at [`ONE`].
///
/// A zero duration reports fully complete.
///
/// Both operands are reduced by a shared right shift before dividing, so a 32-bit
/// divide always suffices however long the step is. The shift preserves the ratio and
/// only engages beyond ~131 seconds, where 17 bits still resolve finer than Q0.15.
#[inline]
pub(crate) const fn progress_q15(time_ms: u64, duration_ms: u64) -> u16 {
    if duration_ms == 0 || time_ms >= duration_ms {
        return ONE;
    }

    let significant_bits = u64::BITS - duration_ms.leading_zeros();
    let shift = significant_bits.saturating_sub(OPERAND_BITS);

    let numerator = (time_ms >> shift) as u32;
    let denominator = (duration_ms >> shift) as u32;

    // The shared shift keeps `numerator <= denominator`, so this cannot exceed ONE.
    ((numerator << SHIFT) / denominator) as u16
}

/// Linearly interpolates one color channel by a Q0.15 factor.
///
/// Branching on direction keeps the product unsigned. `t == ONE` lands exactly on
/// `to`, so transitions finish on their target color.
#[inline]
pub(crate) const fn lerp_channel(from: u16, to: u16, t: u16) -> u16 {
    if to >= from {
        from + (((to - from) as u32 * t as u32) >> SHIFT) as u16
    } else {
        from - (((from - to) as u32 * t as u32) >> SHIFT) as u16
    }
}

/// Scales a channel by an 8-bit factor where 255 is unity.
///
/// Divides by 255 via the series `1/255 = (1 + 2^-8 + 2^-16 + ...) / 256`. Both
/// correction terms are needed over this 24-bit product; the single-term form used
/// for 8-bit blending is one ULP short at the top and would make 255 lossy.
#[inline]
pub(crate) const fn scale_channel(value: u16, factor: u8) -> u16 {
    let x = value as u32 * factor as u32 + 128;
    ((x + (x >> 8) + (x >> 16)) >> 8) as u16
}
