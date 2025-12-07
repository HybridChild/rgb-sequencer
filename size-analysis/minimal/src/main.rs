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
// Test Sequences
// ============================================================================

// This function uses the library to prevent optimizer from removing code
#[inline(never)]
fn test_sequences() {
    let time_source = MinimalTimeSource;
    let led = MinimalLed;

    // Test different sequence capacities

    // 4-step sequence
    let seq4 = RgbSequence::<Duration32, 4>::builder()
        .step(
            Srgb::new(1.0, 0.0, 0.0),
            Duration32::new(1000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 1.0, 0.0),
            Duration32::new(1000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 0.0, 1.0),
            Duration32::new(1000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(1.0, 1.0, 1.0),
            Duration32::new(1000),
            TransitionStyle::Step,
        )
        .loop_count(LoopCount::Infinite)
        .build();

    if let Ok(sequence) = seq4 {
        let mut sequencer = RgbSequencer::new(led, &time_source);
        sequencer.load(sequence);
        let _ = sequencer.start();
        let _ = sequencer.service();
        core::hint::black_box(sequencer);
    }

    // 8-step sequence
    let seq8 = RgbSequence::<Duration32, 8>::builder()
        .step(
            Srgb::new(1.0, 0.0, 0.0),
            Duration32::new(500),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 1.0, 0.0),
            Duration32::new(500),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 0.0, 1.0),
            Duration32::new(500),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(1.0, 1.0, 0.0),
            Duration32::new(500),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(1.0, 0.0, 1.0),
            Duration32::new(500),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 1.0, 1.0),
            Duration32::new(500),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(1.0, 1.0, 1.0),
            Duration32::new(500),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 0.0, 0.0),
            Duration32::new(500),
            TransitionStyle::Step,
        )
        .loop_count(LoopCount::Finite(10))
        .build();

    if let Ok(sequence) = seq8 {
        let led2 = MinimalLed;
        let mut sequencer = RgbSequencer::new(led2, &time_source);
        sequencer.load(sequence);
        let _ = sequencer.start();
        let _ = sequencer.pause();
        let _ = sequencer.resume();
        core::hint::black_box(sequencer);
    }

    // 16-step sequence
    let seq16 = RgbSequence::<Duration32, 16>::builder()
        .step(
            Srgb::new(1.0, 0.0, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.9, 0.1, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.8, 0.2, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.7, 0.3, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.6, 0.4, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.5, 0.5, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.4, 0.6, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.3, 0.7, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.2, 0.8, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.1, 0.9, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 1.0, 0.0),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 0.9, 0.1),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 0.8, 0.2),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 0.7, 0.3),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 0.6, 0.4),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::new(0.0, 0.5, 0.5),
            Duration32::new(250),
            TransitionStyle::Linear,
        )
        .loop_count(LoopCount::Infinite)
        .start_color(Srgb::new(0.0, 0.0, 0.0))
        .landing_color(Srgb::new(0.0, 0.0, 0.0))
        .build();

    if let Ok(sequence) = seq16 {
        let led3 = MinimalLed;
        let mut sequencer = RgbSequencer::new(led3, &time_source);
        sequencer.load(sequence);
        let _ = sequencer.restart();
        core::hint::black_box(sequencer);
    }

    // Function-based sequence (0 capacity)
    fn color_fn(_base: Srgb, _t: Duration32) -> Srgb {
        Srgb::new(0.5, 0.5, 0.5)
    }

    fn timing_fn(_t: Duration32) -> Option<Duration32> {
        Some(Duration32::new(16))
    }

    let seq_func =
        RgbSequence::<Duration32, 0>::from_function(Srgb::new(1.0, 1.0, 1.0), color_fn, timing_fn);

    let led4 = MinimalLed;
    let mut sequencer = RgbSequencer::new(led4, &time_source);
    sequencer.load(seq_func);
    let _ = sequencer.start();
    core::hint::black_box(sequencer);
}

#[entry]
fn main() -> ! {
    // Call test function to ensure all code is included
    test_sequences();

    // Halt - this is a size analysis binary, not meant to run
    loop {
        cortex_m::asm::nop();
    }
}
