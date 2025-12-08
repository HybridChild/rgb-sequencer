//! Time abstraction traits for platform-agnostic timing.

/// Trait for abstracting time sources.
pub trait TimeSource<I: TimeInstant> {
    /// Returns the current time instant.
    fn now(&self) -> I;
}

/// Abstracts over duration types.
pub trait TimeDuration: Copy + PartialEq {
    const ZERO: Self;

    fn as_millis(&self) -> u64;

    fn from_millis(millis: u64) -> Self;

    /// Saturating subtraction (returns ZERO on underflow).
    fn saturating_sub(self, other: Self) -> Self;
}

/// Abstracts over instant types.
pub trait TimeInstant: Copy {
    type Duration: TimeDuration;

    fn duration_since(&self, earlier: Self) -> Self::Duration;

    fn checked_add(self, duration: Self::Duration) -> Option<Self>;

    fn checked_sub(self, duration: Self::Duration) -> Option<Self>;
}
