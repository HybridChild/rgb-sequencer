//! Shared test infrastructure for rgb-sequencer integration tests

#![allow(dead_code)] // Items used across multiple test files; Rust analyzes per-file

use palette::Srgb;
use rgb_sequencer::{RgbLed, TimeDuration, TimeInstant, TimeSource};

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
    current_color: Srgb,
    color_history: heapless::Vec<Srgb, 32>,
}

impl MockLed {
    pub fn new() -> Self {
        Self {
            current_color: Srgb::new(0.0, 0.0, 0.0),
            color_history: heapless::Vec::new(),
        }
    }

    pub fn get_last_color(&self) -> Srgb {
        self.current_color
    }

    pub fn color_history(&self) -> &[Srgb] {
        &self.color_history
    }
}

impl RgbLed for MockLed {
    fn set_color(&mut self, color: Srgb) {
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

/// Compare two colors with floating-point tolerance
pub fn colors_equal(a: Srgb, b: Srgb) -> bool {
    const EPSILON: f32 = 0.001;
    (a.red - b.red).abs() < EPSILON
        && (a.green - b.green).abs() < EPSILON
        && (a.blue - b.blue).abs() < EPSILON
}

/// Compare two colors with custom epsilon
pub fn colors_equal_epsilon(a: Srgb, b: Srgb, epsilon: f32) -> bool {
    (a.red - b.red).abs() < epsilon
        && (a.green - b.green).abs() < epsilon
        && (a.blue - b.blue).abs() < epsilon
}
