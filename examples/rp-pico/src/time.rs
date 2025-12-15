//! Hardware timer wrapper for rgb-sequencer time traits.
//!
//! This module provides wrappers around the RP2040 hardware timer (using fugit types)
//! to implement the rgb-sequencer time traits.

use fugit::{MicrosDurationU64, TimerInstantU64};
use rgb_sequencer::{TimeDuration, TimeInstant, TimeSource};

/// Duration type backed by fugit microsecond duration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(MicrosDurationU64);

impl TimeDuration for Duration {
    const ZERO: Self = Duration(MicrosDurationU64::from_ticks(0));

    fn as_millis(&self) -> u64 {
        self.0.to_millis()
    }

    fn from_millis(millis: u64) -> Self {
        Duration(MicrosDurationU64::millis(millis))
    }

    fn saturating_sub(self, other: Self) -> Self {
        let result = self.0.to_micros().saturating_sub(other.0.to_micros());
        Duration(MicrosDurationU64::micros(result))
    }
}

/// Instant type backed by fugit timer instant
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(TimerInstantU64<1_000_000>);

impl TimeInstant for Instant {
    type Duration = Duration;

    fn duration_since(&self, earlier: Self) -> Self::Duration {
        let ticks = self.0.ticks().saturating_sub(earlier.0.ticks());
        Duration(MicrosDurationU64::from_ticks(ticks))
    }

    fn checked_add(self, duration: Self::Duration) -> Option<Self> {
        let new_ticks = self.0.ticks().checked_add(duration.0.to_micros())?;
        Some(Instant(TimerInstantU64::from_ticks(new_ticks)))
    }

    fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
        let new_ticks = self.0.ticks().checked_sub(duration.0.to_micros())?;
        Some(Instant(TimerInstantU64::from_ticks(new_ticks)))
    }
}

impl From<TimerInstantU64<1_000_000>> for Instant {
    fn from(instant: TimerInstantU64<1_000_000>) -> Self {
        Instant(instant)
    }
}

/// Time source wrapper around RP2040 Timer
pub struct HardwareTimer {
    timer: rp_pico::hal::Timer,
}

impl HardwareTimer {
    /// Create a new hardware timer wrapper
    pub fn new(timer: rp_pico::hal::Timer) -> Self {
        Self { timer }
    }
}

impl TimeSource<Instant> for HardwareTimer {
    fn now(&self) -> Instant {
        Instant(self.timer.get_counter())
    }
}
