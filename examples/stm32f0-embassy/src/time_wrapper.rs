use rgb_sequencer::{TimeDuration, TimeInstant, TimeSource};

/// Newtype wrapper for embassy_time::Duration to implement TimeDuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EmbassyDuration(pub embassy_time::Duration);

impl TimeDuration for EmbassyDuration {
    const ZERO: Self = EmbassyDuration(embassy_time::Duration::from_ticks(0));

    fn as_millis(&self) -> u64 {
        self.0.as_millis()
    }

    fn from_millis(millis: u64) -> Self {
        EmbassyDuration(embassy_time::Duration::from_millis(millis))
    }

    fn saturating_sub(self, other: Self) -> Self {
        EmbassyDuration(embassy_time::Duration::from_ticks(
            self.0.as_ticks().saturating_sub(other.0.as_ticks()),
        ))
    }
}

/// Newtype wrapper for embassy_time::Instant to implement TimeInstant
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EmbassyInstant(pub embassy_time::Instant);

impl TimeInstant for EmbassyInstant {
    type Duration = EmbassyDuration;

    fn duration_since(&self, earlier: Self) -> Self::Duration {
        EmbassyDuration(self.0 - earlier.0)
    }

    fn checked_add(self, duration: Self::Duration) -> Option<Self> {
        Some(EmbassyInstant(self.0 + duration.0))
    }

    fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_sub(duration.0).map(EmbassyInstant)
    }
}

/// Time source implementation for Embassy
pub struct EmbassyTimeSource;

impl EmbassyTimeSource {
    /// Creates a new Embassy time source
    pub fn new() -> Self {
        Self
    }
}

impl TimeSource<EmbassyInstant> for EmbassyTimeSource {
    fn now(&self) -> EmbassyInstant {
        EmbassyInstant(embassy_time::Instant::now())
    }
}
