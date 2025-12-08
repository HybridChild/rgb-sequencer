#![no_std]
#![no_main]

use cortex_m_rt::entry;
use palette::Srgb;
use panic_halt as _;
use rgb_sequencer::{
    LoopCount, RgbLed, RgbSequence, RgbSequencer, TimeDuration, TimeInstant, TimeSource,
    TransitionStyle,
};

// ============================================================================
// Minimal Time Types
// ============================================================================

/// Minimal 32-bit millisecond duration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration32(u32);

impl Duration32 {
    const fn new(millis: u32) -> Self {
        Duration32(millis)
    }
}

impl TimeDuration for Duration32 {
    const ZERO: Self = Duration32(0);

    fn as_millis(&self) -> u64 {
        self.0 as u64
    }

    fn from_millis(millis: u64) -> Self {
        Duration32(millis as u32)
    }

    fn saturating_sub(self, other: Self) -> Self {
        Duration32(self.0.saturating_sub(other.0))
    }
}

/// Minimal 32-bit millisecond instant
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant32(u32);

impl TimeInstant for Instant32 {
    type Duration = Duration32;

    fn duration_since(&self, earlier: Self) -> Self::Duration {
        Duration32(self.0.saturating_sub(earlier.0))
    }

    fn checked_add(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_add(duration.0).map(Instant32)
    }

    fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_sub(duration.0).map(Instant32)
    }
}

// ============================================================================
// Minimal LED Implementation
// ============================================================================

/// Zero-size LED implementation for measuring library overhead
pub struct MinimalLed;

impl RgbLed for MinimalLed {
    fn set_color(&mut self, _color: Srgb) {
        // Minimal no-op implementation
        core::hint::black_box(());
    }
}

// ============================================================================
// Minimal TimeSource Implementation
// ============================================================================

pub struct MinimalTimeSource;

impl TimeSource<Instant32> for MinimalTimeSource {
    fn now(&self) -> Instant32 {
        Instant32(0)
    }
}

// ============================================================================
// Test Sequence
// ============================================================================

// This function uses the library to prevent optimizer from removing code
#[inline(never)]
fn test_sequence() {
    let time_source = MinimalTimeSource;
    let led = MinimalLed;

    // Single 4-step sequence with all features:
    // - Linear transitions
    // - start_color (smooth entry from black)
    // - landing_color (smooth exit to black on completion)
    // - Finite loop count
    let sequence = RgbSequence::<Duration32, 4>::builder()
        .step(
            Srgb::new(1.0, 0.0, 0.0),  // Red
            Duration32::new(1000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 1.0, 0.0),  // Green
            Duration32::new(1000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 0.0, 1.0),  // Blue
            Duration32::new(1000),
            TransitionStyle::Step,
        )
        .step(
            Srgb::new(1.0, 1.0, 1.0),  // White
            Duration32::new(1000),
            TransitionStyle::Step,
        )
        .start_color(Srgb::new(0.0, 0.0, 0.0))      // Start from black
        .landing_color(Srgb::new(0.0, 0.0, 0.0))    // End on black
        .loop_count(LoopCount::Finite(3))
        .build();

    // Exercise all API methods on a single sequencer
    if let Ok(seq) = sequence {
        let mut sequencer = RgbSequencer::new(led, &time_source);

        // Load and start
        sequencer.load(seq);
        let _ = sequencer.start();

        // Service (updates LED)
        let _ = sequencer.service();

        // Pause and resume
        let _ = sequencer.pause();
        let _ = sequencer.resume();

        // Restart
        let _ = sequencer.restart();

        // Query state
        let _ = sequencer.get_state();

        // Clear
        sequencer.clear();

        core::hint::black_box(sequencer);
    }
}

#[entry]
fn main() -> ! {
    // Call test function to ensure all code is included
    test_sequence();

    // Halt - this is a size analysis binary, not meant to run
    loop {
        cortex_m::asm::nop();
    }
}
