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

/// Longest step duration that [`progress_q15`] can resolve with a 32-bit divide.
///
/// `131_071 << 15` is the largest left-shifted value that still fits a `u32`.
const MAX_FAST_DURATION_MS: u64 = (u32::MAX >> SHIFT) as u64;

/// Multiplies two Q0.15 values.
///
/// Both operands must be in `0..=ONE`; the result is too.
#[inline]
pub(crate) const fn mul_q15(a: u16, b: u16) -> u16 {
    ((a as u32 * b as u32) >> SHIFT) as u16
}

/// Computes elapsed-within-step progress as a Q0.15 fraction, saturating at [`ONE`].
///
/// A zero duration reports fully complete, matching the f32 implementation this
/// replaces (which short-circuited zero-duration steps before dividing).
#[inline]
pub(crate) const fn progress_q15(time_ms: u64, duration_ms: u64) -> u16 {
    if duration_ms == 0 || time_ms >= duration_ms {
        return ONE;
    }

    if duration_ms <= MAX_FAST_DURATION_MS {
        // Fast path: one 32-bit divide. Covers every step shorter than ~131 seconds,
        // which is every realistic LED animation step.
        (((time_ms as u32) << SHIFT) / duration_ms as u32) as u16
    } else if time_ms <= u64::MAX >> SHIFT {
        // Exact, but pays for a 64-bit divide.
        ((time_ms << SHIFT) / duration_ms) as u16
    } else {
        // Only reachable if elapsed time exceeds 2^49 ms (~17,800 years). Dividing
        // first keeps this total; `duration_ms >> SHIFT` is enormous here, so the
        // lost precision is far below one Q0.15 LSB.
        let scaled = time_ms / (duration_ms >> SHIFT);
        if scaled > ONE as u64 {
            ONE
        } else {
            scaled as u16
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Q0.15 value as an f32 in 0.0..=1.0.
    fn to_f32(q: u16) -> f32 {
        q as f32 / ONE as f32
    }

    #[test]
    fn one_and_half_are_consistent() {
        assert_eq!(ONE, 32768);
        assert_eq!(HALF, 16384);
        assert_eq!(to_f32(ONE), 1.0);
        assert_eq!(to_f32(HALF), 0.5);
    }

    #[test]
    fn mul_q15_endpoints_are_exact() {
        assert_eq!(mul_q15(ONE, ONE), ONE);
        assert_eq!(mul_q15(ONE, 0), 0);
        assert_eq!(mul_q15(0, ONE), 0);
        assert_eq!(mul_q15(ONE, HALF), HALF);
        assert_eq!(mul_q15(HALF, HALF), ONE / 4);
    }

    #[test]
    fn mul_q15_matches_f32_across_full_range() {
        let mut a = 0u32;
        while a <= ONE as u32 {
            let mut b = 0u32;
            while b <= ONE as u32 {
                let expected = (to_f32(a as u16) * to_f32(b as u16) * ONE as f32) as i32;
                let actual = mul_q15(a as u16, b as u16) as i32;
                assert!(
                    (expected - actual).abs() <= 1,
                    "mul_q15({a}, {b}) = {actual}, expected ~{expected}"
                );
                b += 97;
            }
            a += 97;
        }
    }

    #[test]
    fn progress_saturates_and_bounds() {
        assert_eq!(progress_q15(0, 1000), 0);
        assert_eq!(progress_q15(1000, 1000), ONE);
        assert_eq!(progress_q15(5000, 1000), ONE);
        // Zero duration reports complete rather than dividing by zero.
        assert_eq!(progress_q15(0, 0), ONE);
        // Progress is strictly below ONE while inside the step.
        assert!(progress_q15(999, 1000) < ONE);
    }

    #[test]
    fn progress_matches_f32_ratio() {
        let durations = [1u64, 2, 7, 100, 1000, 16_384, 131_071];
        for &d in &durations {
            for n in 0..=32u64 {
                let t = n * d / 32;
                if t >= d {
                    continue;
                }
                let expected = ((t as f32 / d as f32) * ONE as f32) as i32;
                let actual = progress_q15(t, d) as i32;
                assert!(
                    (expected - actual).abs() <= 1,
                    "progress_q15({t}, {d}) = {actual}, expected ~{expected}"
                );
            }
        }
    }

    #[test]
    fn progress_slow_path_matches_fast_path_semantics() {
        // Just past MAX_FAST_DURATION_MS, forcing the 64-bit branch.
        let d = MAX_FAST_DURATION_MS + 1;
        assert_eq!(progress_q15(0, d), 0);
        assert_eq!(progress_q15(d, d), ONE);
        let mid = progress_q15(d / 2, d) as i32;
        assert!((mid - HALF as i32).abs() <= 1, "midpoint was {mid}");
    }

    #[test]
    fn lerp_channel_endpoints_are_exact() {
        assert_eq!(lerp_channel(0, 65535, 0), 0);
        assert_eq!(lerp_channel(0, 65535, ONE), 65535);
        assert_eq!(lerp_channel(65535, 0, ONE), 0);
        assert_eq!(lerp_channel(1234, 1234, HALF), 1234);
        // Descending direction reaches its target too.
        assert_eq!(lerp_channel(40000, 100, ONE), 100);
    }

    #[test]
    fn lerp_channel_matches_f32_both_directions() {
        let pairs = [
            (0u16, 65535u16),
            (65535, 0),
            (0, 1),
            (30000, 40000),
            (40000, 30000),
            (12345, 54321),
        ];
        for &(from, to) in &pairs {
            let mut t = 0u32;
            while t <= ONE as u32 {
                let f = to_f32(t as u16);
                let expected = (from as f32 + (to as f32 - from as f32) * f) as i32;
                let actual = lerp_channel(from, to, t as u16) as i32;
                assert!(
                    (expected - actual).abs() <= 2,
                    "lerp_channel({from}, {to}, {t}) = {actual}, expected ~{expected}"
                );
                t += 313;
            }
        }
    }

    #[test]
    fn scale_channel_full_is_identity() {
        assert_eq!(scale_channel(0, 255), 0);
        assert_eq!(scale_channel(65535, 255), 65535);
        assert_eq!(scale_channel(12345, 255), 12345);
        assert_eq!(scale_channel(65535, 0), 0);
    }

    #[test]
    fn scale_channel_matches_f32() {
        let values = [0u16, 1, 255, 12345, 32768, 65534, 65535];
        for &v in &values {
            for factor in 0..=255u8 {
                let expected = (v as f32 * (factor as f32 / 255.0)) as i32;
                let actual = scale_channel(v, factor) as i32;
                assert!(
                    (expected - actual).abs() <= 1,
                    "scale_channel({v}, {factor}) = {actual}, expected ~{expected}"
                );
            }
        }
    }
}
