//! Time abstraction traits for platform-agnostic duration and instant types.
//!
//! These traits allow the sequencer to work with different timing systems
//! (Embassy, std::time, custom hardware timers, etc.) without being tied
//! to a specific implementation.
//!
//! Implementers should ensure their types handle time wrapping appropriately
//! for their platform's timer width (e.g., 32-bit vs 64-bit counters).

/// Trait for abstracting time sources.
///
/// Allows the sequencer to query current time from different systems
/// (Embassy, std, custom timers, etc.).
pub trait TimeSource<I: TimeInstant> {
    /// Returns the current time instant.
    fn now(&self) -> I;
}

/// Abstracts over duration types from different embedded time libraries.
pub trait TimeDuration: Copy + PartialEq {
    /// A duration of zero length, used for initialization.
    const ZERO: Self;

    /// Returns this duration in milliseconds.
    fn as_millis(&self) -> u64;

    /// Creates a duration from a millisecond count.
    fn from_millis(millis: u64) -> Self;

    /// Subtracts another duration from this one, used for calculating remaining time in steps.
    ///
    /// If `other` is greater than `self`, implementations should return `Self::ZERO` rather
    /// than wrapping or panicking (saturating behavior).
    fn saturating_sub(self, other: Self) -> Self;
}

/// Abstracts over instant types from different embedded time libraries.
pub trait TimeInstant: Copy {
    /// The duration type produced when calculating time differences.
    type Duration: TimeDuration;

    /// Returns the duration elapsed from `earlier` to `self`.
    ///
    /// # Panics
    /// May panic if `earlier` is after `self`. Implementations should either panic
    /// or handle time wrapping appropriately for their use case.
    fn duration_since(&self, earlier: Self) -> Self::Duration;

    /// Adds a duration to this instant.
    ///
    /// Returns `None` if the resulting instant would overflow.
    fn checked_add(self, duration: Self::Duration) -> Option<Self>;

    /// Subtracts a duration from this instant.
    ///
    /// Returns `None` if the subtraction would underflow (i.e., the duration
    /// is larger than the time elapsed since the instant's reference point).
    fn checked_sub(self, duration: Self::Duration) -> Option<Self>;
}
