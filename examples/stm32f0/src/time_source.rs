use core::cell::Cell;
use critical_section::Mutex;
use rgb_sequencer::{TimeDuration, TimeInstant, TimeSource};

/// Global millisecond counter incremented by SysTick interrupt
/// 
/// This counter is automatically incremented every millisecond by the SysTick
/// interrupt handler. It wraps after ~49.7 days of continuous operation.
static MILLIS_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

/// Increments the global millisecond counter.
/// 
/// This function should be called from the SysTick interrupt handler every 1ms.
/// It's marked as `pub` so it can be accessed from the interrupt handler in main.rs.
pub fn tick() {
    critical_section::with(|cs| {
        let counter = MILLIS_COUNTER.borrow(cs);
        let current = counter.get();
        counter.set(current.wrapping_add(1));
    });
}

/// Duration type using milliseconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HalDuration(pub u64);

impl TimeDuration for HalDuration {
    const ZERO: Self = HalDuration(0);

    fn as_millis(&self) -> u64 {
        self.0
    }

    fn from_millis(millis: u64) -> Self {
        HalDuration(millis)
    }

    fn saturating_sub(self, other: Self) -> Self {
        HalDuration(self.0.saturating_sub(other.0))
    }
}

/// Instant type representing a point in time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HalInstant(u32);

impl HalInstant {
    /// Returns the instant as milliseconds since startup
    pub fn as_millis(&self) -> u32 {
        self.0
    }
}

impl TimeInstant for HalInstant {
    type Duration = HalDuration;

    fn duration_since(&self, earlier: Self) -> Self::Duration {
        // Handle wrapping correctly using wrapping_sub
        let diff = self.0.wrapping_sub(earlier.0);
        HalDuration(diff as u64)
    }

    fn checked_add(self, duration: Self::Duration) -> Option<Self> {
        // Truncate duration to u32 and use wrapping add to handle overflow
        let millis = duration.0 as u32;
        Some(HalInstant(self.0.wrapping_add(millis)))
    }

    fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
        // Truncate duration to u32 and use wrapping sub to handle underflow
        let millis = duration.0 as u32;
        Some(HalInstant(self.0.wrapping_sub(millis)))
    }
}

/// Time source that provides current time instants based on SysTick
/// 
/// This time source reads from a global counter that's automatically incremented
/// by the SysTick interrupt handler every millisecond. No manual tick() calls needed!
pub struct HalTimeSource;

impl HalTimeSource {
    pub fn new() -> Self {
        Self
    }
}

impl TimeSource<HalInstant> for HalTimeSource {
    fn now(&self) -> HalInstant {
        critical_section::with(|cs| {
            HalInstant(MILLIS_COUNTER.borrow(cs).get())
        })
    }
}
