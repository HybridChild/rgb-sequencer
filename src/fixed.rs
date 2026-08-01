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

/// Scales a channel by a 16-bit factor where [`FULL`](crate::FULL) is unity.
///
/// Divides by 65535 via the series `1/65535 = (1 + 2^-16 + 2^-32 + ...) / 65536`,
/// rounding to nearest. One correction term suffices: the next is worth less than a
/// ULP over a 32-bit product, and a factor of 65535 still lands exactly on `value`,
/// so full brightness is not lossy.
///
/// The widest intermediate is `65535 * 65535 + 32768 + 65534`, which is 32768 short
/// of `u32::MAX` - the operand types bound it, so it cannot overflow.
#[inline]
pub(crate) const fn scale_channel(value: u16, factor: u16) -> u16 {
    // 32768 is the round-to-nearest bias - half a divisor, rounded up.
    let x = value as u32 * factor as u32 + 32768;
    ((x + (x >> 16)) >> 16) as u16
}

/// Divides by 65535, truncating, for products of two values in `0..=65535`.
///
/// The same reciprocal series as [`scale_channel`] but without the rounding bias, so
/// it reproduces a plain `/ 65535` bit for bit while costing only shifts - `thumbv6m`
/// has no divide instruction, so the literal form calls `__udivsi3`.
///
/// The trailing `+ 1` is load-bearing rather than a rounding term: without it the
/// series falls one short at every exact multiple of 65535. Verified against
/// `x / 65535` for every `x` in `0..=65535 * 65535`.
#[inline]
pub(crate) const fn div_65535(x: u32) -> u32 {
    (x + (x >> 16) + 1) >> 16
}
