//! Time abstraction traits for platform-agnostic timing.

/// Trait for abstracting time sources.
pub trait TimeSource<I: TimeInstant> {
    /// Returns the current time instant.
    fn now(&self) -> I;
}

/// Trait abstraction for duration types.
pub trait TimeDuration: Copy + PartialEq {
    /// Zero duration constant.
    const ZERO: Self;

    /// Converts duration to milliseconds.
    fn as_millis(&self) -> u64;

    /// Creates duration from milliseconds.
    fn from_millis(millis: u64) -> Self;

    /// Saturating subtraction (returns ZERO on underflow).
    fn saturating_sub(self, other: Self) -> Self;
}

/// Trait abstraction for instant types.
pub trait TimeInstant: Copy {
    /// Duration type for this instant.
    type Duration: TimeDuration;

    /// Calculates duration since an earlier instant.
    fn duration_since(&self, earlier: Self) -> Self::Duration;

    /// Adds duration to instant, returns None on overflow.
    fn checked_add(self, duration: Self::Duration) -> Option<Self>;

    /// Subtracts duration from instant, returns None on underflow.
    fn checked_sub(self, duration: Self::Duration) -> Option<Self>;
}
