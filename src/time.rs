/// Abstracts over duration types from different embedded time libraries.
///
/// Allows the sequencer to work with Embassy's `Duration`, `std::time::Duration`,
/// fugit durations, or custom implementations. All implementations must be `Copy`
/// and provide millisecond conversions for internal calculations.
pub trait TimeDuration: Copy {
    /// A duration of zero length, used for initialization.
    const ZERO: Self;

    /// Returns this duration in milliseconds.
    fn as_millis(&self) -> u64;

    /// Creates a duration from a millisecond count.
    fn from_millis(millis: u64) -> Self;

    /// Subtracts another duration from this one, used for calculating remaining time in steps.
    ///
    /// Overflow behavior when `other > self` is implementation-defined.
    fn saturating_sub(self, other: Self) -> Self;
}

/// Abstracts over instant types from different embedded time libraries.
///
/// Allows the sequencer to work with Embassy's `Instant`, `std::time::Instant`,
/// fugit instants, or custom implementations. All implementations must be `Copy`.
pub trait TimeInstant: Copy {
    /// The duration type produced when calculating time differences.
    type Duration: TimeDuration;

    /// Returns the duration elapsed from `earlier` to `self`.
    ///
    /// Panics if `earlier` is after `self` (implementation-defined).
    fn duration_since(&self, earlier: Self) -> Self::Duration;
}
