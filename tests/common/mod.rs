//! Shared test infrastructure for rgb-sequencer integration tests

#![allow(dead_code)] // Items used across multiple test files; Rust analyzes per-file

use rgb_sequencer::{Rgb, RgbLed, TimeDuration, TimeInstant, TimeSource};

// ============================================================================
// Mock Time Types
// ============================================================================

/// Mock duration type for testing (wraps milliseconds)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TestDuration(pub u64);

impl TimeDuration for TestDuration {
    const ZERO: Self = TestDuration(0);

    fn as_millis(&self) -> u64 {
        self.0
    }

    fn from_millis(millis: u64) -> Self {
        TestDuration(millis)
    }

    fn saturating_sub(self, other: Self) -> Self {
        TestDuration(self.0.saturating_sub(other.0))
    }
}

/// Mock instant type for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TestInstant(pub u64);

impl TimeInstant for TestInstant {
    type Duration = TestDuration;

    fn duration_since(&self, earlier: Self) -> Self::Duration {
        TestDuration(self.0 - earlier.0)
    }

    fn checked_add(self, duration: Self::Duration) -> Option<Self> {
        Some(TestInstant(self.0 + duration.0))
    }

    fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_sub(duration.0).map(TestInstant)
    }
}

// ============================================================================
// Mock LED
// ============================================================================

/// Mock LED that records all color changes for testing
pub struct MockLed {
    current_color: Rgb,
    color_history: heapless::Vec<Rgb, 32>,
}

impl MockLed {
    pub fn new() -> Self {
        Self {
            current_color: Rgb::new(0, 0, 0),
            color_history: heapless::Vec::new(),
        }
    }

    pub fn get_last_color(&self) -> Rgb {
        self.current_color
    }

    pub fn color_history(&self) -> &[Rgb] {
        &self.color_history
    }
}

impl RgbLed for MockLed {
    fn set_color(&mut self, color: Rgb) {
        self.current_color = color;
        let _ = self.color_history.push(color);
    }
}

// ============================================================================
// Mock Time Source
// ============================================================================

/// Mock time source with controllable time advancement
pub struct MockTimeSource {
    current_time: core::cell::Cell<TestInstant>,
}

impl MockTimeSource {
    pub fn new() -> Self {
        Self {
            current_time: core::cell::Cell::new(TestInstant(0)),
        }
    }

    /// Advance time by the given duration
    pub fn advance(&self, duration: TestDuration) {
        let current = self.current_time.get();
        self.current_time.set(TestInstant(current.0 + duration.0));
    }

    pub fn set_time(&self, time: TestInstant) {
        self.current_time.set(time);
    }
}

impl TimeSource<TestInstant> for MockTimeSource {
    fn now(&self) -> TestInstant {
        self.current_time.get()
    }
}

// ============================================================================
// Re-export color constants from library for test convenience
// ============================================================================

#[allow(unused_imports)]
pub use rgb_sequencer::{BLACK, BLUE, GREEN, RED};

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Compare two colors with the default tolerance.
pub fn colors_equal(a: Rgb, b: Rgb) -> bool {
    a.approx_eq(b, DEFAULT_TEST_EPSILON)
}

/// Default per-channel tolerance used by [`colors_equal`], ~0.2% of full scale.
///
/// Several tests sample a transition at 999/1000 of the way through and assert it
/// has essentially reached its target, which leaves a residual of ~0.1%. The
/// tolerance has to sit above that residual: the old f32 epsilon of 0.001 was
/// exactly on that boundary and passed only by rounding luck.
pub const DEFAULT_TEST_EPSILON: u16 = 128;

/// Compare two colors with custom epsilon
pub fn colors_equal_epsilon(a: Rgb, b: Rgb, epsilon: u16) -> bool {
    a.approx_eq(b, epsilon)
}

/// Converts a 0.0-1.0 fraction to a channel value, for readable test expectations.
///
/// Test cases were originally written against f32 colors; this keeps those
/// intentions legible (`channel(0.5)` rather than `32768`) without putting any
/// floating point into the library itself.
pub fn channel(fraction: f32) -> u16 {
    (fraction * 65535.0) as u16
}

/// Builds a color from 0.0-1.0 fractions, mirroring the old `Srgb::new`.
pub fn rgb_f(r: f32, g: f32, b: f32) -> Rgb {
    Rgb::new(channel(r), channel(g), channel(b))
}

/// Q0.15 interpolation factor from a 0.0-1.0 fraction, mirroring the old `mix`.
pub fn factor(fraction: f32) -> u16 {
    (fraction * 32768.0) as u16
}

/// 8-bit brightness value from a 0.0-1.0 fraction.
pub fn bright(fraction: f32) -> u8 {
    (fraction * 255.0).round() as u8
}

/// Tolerance for comparisons that pass through 8-bit brightness scaling.
///
/// A `u8` brightness cannot represent fractions like 50% exactly — 127/255 and
/// 128/255 straddle it — so an expectation written as a decimal fraction lands up
/// to ~0.2% of full scale away. This tolerance sits just above that.
pub const BRIGHTNESS_EPSILON: u16 = 256;
