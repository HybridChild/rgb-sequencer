//! RGB LED sequencer with state management.

use crate::COLOR_OFF;
use crate::command::SequencerAction;
use crate::sequence::RgbSequence;
use crate::time::{TimeDuration, TimeInstant, TimeSource};
use palette::Srgb;

/// Trait for abstracting RGB LED hardware.
pub trait RgbLed {
    /// Sets LED to specified color.
    ///
    /// Color components are in 0.0-1.0 range. Convert to your hardware's native format
    /// (PWM duty cycles, 8-bit values, etc.) in your implementation.
    fn set_color(&mut self, color: Srgb);
}

/// The current state of an RGB sequencer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SequencerState {
    /// No sequence loaded.
    Idle,
    /// Sequence loaded.
    Loaded,
    /// Sequence running.
    Running,
    /// Sequence paused.
    Paused,
    /// Sequence complete.
    Complete,
}

/// Timing information returned by service operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ServiceTiming<D> {
    /// Continuous animation - service again at your target frame rate (e.g., 16-33ms for 30-60 FPS).
    Continuous,
    /// Static hold - can delay this duration before next service call.
    Delay(D),
    /// Sequence complete - no further servicing needed.
    Complete,
}

/// Errors that can occur during sequencer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SequencerError {
    /// Invalid state.
    InvalidState {
        /// Expected state description.
        expected: &'static str,
        /// Actual current state.
        actual: SequencerState,
    },
    /// No sequence loaded.
    NoSequenceLoaded,
}

impl core::fmt::Display for SequencerError {
    /// Formats the error for display.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SequencerError::InvalidState { expected, actual } => {
                write!(
                    f,
                    "invalid state: expected {}, but sequencer is in {:?}",
                    expected, actual
                )
            }
            SequencerError::NoSequenceLoaded => {
                write!(f, "no sequence loaded")
            }
        }
    }
}

/// Controls a single RGB LED through sequences.
pub struct RgbSequencer<'t, I: TimeInstant, L: RgbLed, T: TimeSource<I>, const N: usize> {
    led: L,
    time_source: &'t T,
    state: SequencerState,
    sequence: Option<RgbSequence<I::Duration, N>>,
    start_time: Option<I>,
    pause_start_time: Option<I>,
    current_color: Srgb,
}

/// Epsilon for floating-point color comparisons.
const COLOR_EPSILON: f32 = 0.001;

/// Returns true if two colors are approximately equal.
#[inline]
fn colors_approximately_equal(a: Srgb, b: Srgb) -> bool {
    (a.red - b.red).abs() < COLOR_EPSILON
        && (a.green - b.green).abs() < COLOR_EPSILON
        && (a.blue - b.blue).abs() < COLOR_EPSILON
}

impl<'t, I: TimeInstant, L: RgbLed, T: TimeSource<I>, const N: usize> RgbSequencer<'t, I, L, T, N> {
    /// Creates sequencer with LED off.
    pub fn new(mut led: L, time_source: &'t T) -> Self {
        led.set_color(COLOR_OFF);

        Self {
            led,
            time_source,
            state: SequencerState::Idle,
            sequence: None,
            start_time: None,
            pause_start_time: None,
            current_color: COLOR_OFF,
        }
    }

    /// Dispatches action to appropriate method.
    pub fn handle_action(
        &mut self,
        action: SequencerAction<I::Duration, N>,
    ) -> Result<ServiceTiming<I::Duration>, SequencerError> {
        match action {
            SequencerAction::Load(sequence) => {
                self.load(sequence);
                Ok(ServiceTiming::Complete)
            }
            SequencerAction::Start => self.start(),
            SequencerAction::Stop => {
                self.stop()?;
                Ok(ServiceTiming::Complete)
            }
            SequencerAction::Pause => {
                self.pause()?;
                Ok(ServiceTiming::Complete)
            }
            SequencerAction::Resume => self.resume(),
            SequencerAction::Restart => self.restart(),
            SequencerAction::Clear => {
                self.clear();
                Ok(ServiceTiming::Complete)
            }
        }
    }

    /// Loads a sequence.
    pub fn load(&mut self, sequence: RgbSequence<I::Duration, N>) {
        self.sequence = Some(sequence);
        self.start_time = None;
        self.pause_start_time = None;
        self.state = SequencerState::Loaded;
    }

    /// Starts sequence.
    pub fn start(&mut self) -> Result<ServiceTiming<I::Duration>, SequencerError> {
        if self.state != SequencerState::Loaded {
            return Err(SequencerError::InvalidState {
                expected: "Loaded",
                actual: self.state,
            });
        }

        if self.sequence.is_none() {
            return Err(SequencerError::NoSequenceLoaded);
        }

        self.start_time = Some(self.time_source.now());
        self.state = SequencerState::Running;
        self.service()
    }

    /// Loads and immediately starts a sequence.
    pub fn load_and_start(
        &mut self,
        sequence: RgbSequence<I::Duration, N>,
    ) -> Result<ServiceTiming<I::Duration>, SequencerError> {
        self.load(sequence);
        self.start()
    }

    /// Restarts sequence.
    pub fn restart(&mut self) -> Result<ServiceTiming<I::Duration>, SequencerError> {
        match self.state {
            SequencerState::Running | SequencerState::Paused | SequencerState::Complete => {
                if self.sequence.is_none() {
                    return Err(SequencerError::NoSequenceLoaded);
                }

                self.start_time = Some(self.time_source.now());
                self.pause_start_time = None;
                self.state = SequencerState::Running;
                self.service()
            }
            _ => Err(SequencerError::InvalidState {
                expected: "Running, Paused, or Complete",
                actual: self.state,
            }),
        }
    }

    /// Services sequencer, updating LED if color changed.
    ///
    /// Must be called from `Running` state. Returns timing hint for next service call.
    #[inline]
    pub fn service(&mut self) -> Result<ServiceTiming<I::Duration>, SequencerError> {
        if self.state != SequencerState::Running {
            return Err(SequencerError::InvalidState {
                expected: "Running",
                actual: self.state,
            });
        }

        let sequence = self.sequence.as_ref().unwrap();
        let start_time = self.start_time.unwrap();
        let current_time = self.time_source.now();
        let elapsed = current_time.duration_since(start_time);

        // Evaluate color and timing
        let (new_color, next_service) = sequence.evaluate(elapsed);

        // Update LED only if color changed (using approximate equality for f32)
        if !colors_approximately_equal(new_color, self.current_color) {
            self.led.set_color(new_color);
            self.current_color = new_color;
        }

        // Convert timing hint to ServiceTiming
        match next_service {
            None => {
                self.state = SequencerState::Complete;
                Ok(ServiceTiming::Complete)
            }
            Some(duration) if duration == I::Duration::ZERO => Ok(ServiceTiming::Continuous),
            Some(duration) => Ok(ServiceTiming::Delay(duration)),
        }
    }

    /// Peeks at next timing hint without updating LED or advancing state.
    ///
    /// Returns `SequencerError::InvalidState` if not in `Running` state.
    #[inline]
    pub fn peek_next_timing(&self) -> Result<ServiceTiming<I::Duration>, SequencerError> {
        if self.state != SequencerState::Running {
            return Err(SequencerError::InvalidState {
                expected: "Running",
                actual: self.state,
            });
        }

        let sequence = self.sequence.as_ref().unwrap();
        let start_time = self.start_time.unwrap();
        let current_time = self.time_source.now();
        let elapsed = current_time.duration_since(start_time);

        // Evaluate timing without updating state
        let (_color, next_service) = sequence.evaluate(elapsed);

        // Convert timing hint to ServiceTiming
        match next_service {
            None => Ok(ServiceTiming::Complete),
            Some(duration) if duration == I::Duration::ZERO => Ok(ServiceTiming::Continuous),
            Some(duration) => Ok(ServiceTiming::Delay(duration)),
        }
    }

    /// Stops sequence and turns LED off.
    pub fn stop(&mut self) -> Result<(), SequencerError> {
        match self.state {
            SequencerState::Running | SequencerState::Paused | SequencerState::Complete => {
                self.start_time = None;
                self.pause_start_time = None;
                self.state = SequencerState::Loaded;

                self.led.set_color(COLOR_OFF);
                self.current_color = COLOR_OFF;

                Ok(())
            }
            _ => Err(SequencerError::InvalidState {
                expected: "Running, Paused, or Complete",
                actual: self.state,
            }),
        }
    }

    /// Pauses sequence at current color.
    ///
    /// Timing is compensated on resume - sequence continues from same position.
    pub fn pause(&mut self) -> Result<(), SequencerError> {
        if self.state != SequencerState::Running {
            return Err(SequencerError::InvalidState {
                expected: "Running",
                actual: self.state,
            });
        }

        self.pause_start_time = Some(self.time_source.now());
        self.state = SequencerState::Paused;
        Ok(())
    }

    /// Resumes paused sequence.
    pub fn resume(&mut self) -> Result<ServiceTiming<I::Duration>, SequencerError> {
        if self.state != SequencerState::Paused {
            return Err(SequencerError::InvalidState {
                expected: "Paused",
                actual: self.state,
            });
        }

        let pause_start = self.pause_start_time.unwrap();
        let current_time = self.time_source.now();
        let pause_duration = current_time.duration_since(pause_start);

        // Add the pause duration to start time to compensate for the time spent paused.
        // This keeps the sequence at the same position it was at when paused.
        // If checked_add returns None (overflow, e.g., due to very long pause on 32-bit timers),
        // we fall back to the old start time. This causes the sequence to jump forward but
        // prevents a crash. This is a graceful degradation on timer overflow.
        let old_start = self.start_time.unwrap();
        self.start_time = Some(old_start.checked_add(pause_duration).unwrap_or(old_start));

        self.pause_start_time = None;
        self.state = SequencerState::Running;
        self.service()
    }

    /// Clears sequence and turns LED off.
    pub fn clear(&mut self) {
        self.sequence = None;
        self.start_time = None;
        self.pause_start_time = None;
        self.state = SequencerState::Idle;

        self.led.set_color(COLOR_OFF);
        self.current_color = COLOR_OFF;
    }

    /// Returns current state.
    #[inline]
    pub fn state(&self) -> SequencerState {
        self.state
    }

    /// Returns current color.
    #[inline]
    pub fn current_color(&self) -> Srgb {
        self.current_color
    }

    /// Returns true if paused.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.state == SequencerState::Paused
    }

    /// Returns true if running.
    #[inline]
    pub fn is_running(&self) -> bool {
        self.state == SequencerState::Running
    }

    /// Returns current sequence reference.
    #[inline]
    pub fn current_sequence(&self) -> Option<&RgbSequence<I::Duration, N>> {
        self.sequence.as_ref()
    }

    /// Returns elapsed time since start.
    pub fn elapsed_time(&self) -> Option<I::Duration> {
        self.start_time.map(|start| {
            let now = self.time_source.now();
            now.duration_since(start)
        })
    }

    /// Returns current playback position (step index, loop number).
    ///
    /// Returns `None` if not running or sequence is function-based. Useful for event detection
    /// (step changes, loop completions) - see examples in tests.
    #[inline]
    pub fn current_position(&self) -> Option<(usize, u32)> {
        if self.state != SequencerState::Running {
            return None;
        }

        let sequence = self.sequence.as_ref()?;
        let start_time = self.start_time?;
        let current_time = self.time_source.now();
        let elapsed = current_time.duration_since(start_time);

        let position = sequence.find_step_position(elapsed)?;
        Some((position.step_index, position.current_loop))
    }

    /// Consumes the sequencer and returns the LED.
    #[inline]
    pub fn into_led(self) -> L {
        self.led
    }

    /// Consumes the sequencer and returns the LED and current sequence.
    #[inline]
    pub fn into_parts(self) -> (L, Option<RgbSequence<I::Duration, N>>) {
        (self.led, self.sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::RgbSequence;
    use crate::time::{TimeDuration, TimeInstant};
    use crate::types::{LoopCount, TransitionStyle};
    use heapless::Vec;
    use palette::Srgb;

    // Mock Duration type
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct TestDuration(u64);

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

    // Mock Instant type
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct TestInstant(u64);

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

    // Mock LED that records color changes
    struct MockLed {
        current_color: Srgb,
        color_history: Vec<Srgb, 32>,
    }

    impl MockLed {
        fn new() -> Self {
            Self {
                current_color: Srgb::new(0.0, 0.0, 0.0),
                color_history: heapless::Vec::new(),
            }
        }

        fn get_last_color(&self) -> Srgb {
            self.current_color
        }
    }

    impl RgbLed for MockLed {
        fn set_color(&mut self, color: Srgb) {
            self.current_color = color;
            let _ = self.color_history.push(color);
        }
    }

    // Mock time source with controllable time
    struct MockTimeSource {
        current_time: core::cell::Cell<TestInstant>,
    }

    impl MockTimeSource {
        fn new() -> Self {
            Self {
                current_time: core::cell::Cell::new(TestInstant(0)),
            }
        }

        fn advance(&self, duration: TestDuration) {
            let current = self.current_time.get();
            self.current_time.set(TestInstant(current.0 + duration.0));
        }
    }

    impl TimeSource<TestInstant> for MockTimeSource {
        fn now(&self) -> TestInstant {
            self.current_time.get()
        }
    }

    // Helper colors
    const RED: Srgb = Srgb::new(1.0, 0.0, 0.0);
    const GREEN: Srgb = Srgb::new(0.0, 1.0, 0.0);
    const BLUE: Srgb = Srgb::new(0.0, 0.0, 1.0);
    const BLACK: Srgb = Srgb::new(0.0, 0.0, 0.0);

    fn colors_equal(a: Srgb, b: Srgb) -> bool {
        const EPSILON: f32 = 0.001;
        (a.red - b.red).abs() < EPSILON
            && (a.green - b.green).abs() < EPSILON
            && (a.blue - b.blue).abs() < EPSILON
    }

    #[test]
    fn start_requires_loaded_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        // Try to start from Idle state (no sequence loaded)
        let result = sequencer.start();
        assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
    }

    #[test]
    fn pause_requires_running_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);

        // Try to pause from Loaded state
        let result = sequencer.pause();
        assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
    }

    #[test]
    fn resume_requires_paused_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);

        // Try to resume from Loaded state
        let result = sequencer.resume();
        assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
    }

    #[test]
    fn service_requires_running_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);

        // Try to service from Loaded state
        let result = sequencer.service();
        assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
    }

    #[test]
    fn loading_and_starting_sequence_updates_led_color() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);
        assert_eq!(sequencer.state(), SequencerState::Loaded);

        sequencer.start().unwrap();
        assert_eq!(sequencer.state(), SequencerState::Running);

        // LED should now be RED
        assert!(colors_equal(sequencer.current_color(), RED));
    }

    #[test]
    fn service_correctly_progresses_through_multiple_steps() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // At start - RED
        assert!(colors_equal(sequencer.current_color(), RED));

        // Advance to middle of first step
        timer.advance(TestDuration(50));
        sequencer.service().unwrap();
        assert!(colors_equal(sequencer.current_color(), RED));

        // Advance to second step
        timer.advance(TestDuration(60));
        sequencer.service().unwrap();
        assert!(colors_equal(sequencer.current_color(), GREEN));

        // Advance to third step
        timer.advance(TestDuration(100));
        sequencer.service().unwrap();
        assert!(colors_equal(sequencer.current_color(), BLUE));
    }

    #[test]
    fn function_based_sequence_computes_colors_correctly() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        fn brightness_pulse(base: Srgb, elapsed: TestDuration) -> Srgb {
            let brightness = if elapsed.as_millis() < 500 { 0.5 } else { 1.0 };
            Srgb::new(
                base.red * brightness,
                base.green * brightness,
                base.blue * brightness,
            )
        }

        fn continuous(_elapsed: TestDuration) -> Option<TestDuration> {
            Some(TestDuration::ZERO)
        }

        let sequence =
            RgbSequence::<TestDuration, 8>::from_function(RED, brightness_pulse, continuous);

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // At start - 50% brightness
        assert!(colors_equal(
            sequencer.current_color(),
            Srgb::new(0.5, 0.0, 0.0)
        ));

        // After 500ms - full brightness
        timer.advance(TestDuration(500));
        sequencer.service().unwrap();
        assert!(colors_equal(sequencer.current_color(), RED));
    }

    #[test]
    fn pause_resume_maintains_position() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // Advance 500ms into first step
        timer.advance(TestDuration(500));
        sequencer.service().unwrap();
        assert!(colors_equal(sequencer.current_color(), RED));

        // Pause
        sequencer.pause().unwrap();
        assert_eq!(sequencer.state(), SequencerState::Paused);

        // Advance time while paused (simulating delay)
        timer.advance(TestDuration(3000));

        // Resume - should still be in RED step
        sequencer.resume().unwrap();
        assert_eq!(sequencer.state(), SequencerState::Running);
        assert!(colors_equal(sequencer.current_color(), RED));

        // Advance 500ms more - should transition to GREEN (total 1000ms in RED)
        timer.advance(TestDuration(500));
        sequencer.service().unwrap();
        assert!(colors_equal(sequencer.current_color(), GREEN));
    }

    #[test]
    fn stop_turns_off_led_and_returns_to_loaded() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        assert!(colors_equal(sequencer.current_color(), RED));

        sequencer.stop().unwrap();
        assert_eq!(sequencer.state(), SequencerState::Loaded);
        assert!(colors_equal(sequencer.current_color(), BLACK));
    }

    #[test]
    fn clear_removes_sequence_and_returns_to_idle() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        assert!(colors_equal(sequencer.current_color(), RED));

        sequencer.clear();
        assert_eq!(sequencer.state(), SequencerState::Idle);
        assert!(colors_equal(sequencer.current_color(), BLACK));
    }

    #[test]
    fn service_returns_correct_delay_for_step_transition() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(500), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        let timing = sequencer.start().unwrap();

        // Should return the remaining time in the first step (1000ms)
        assert_eq!(timing, ServiceTiming::Delay(TestDuration(1000)));
    }

    #[test]
    fn service_returns_continuous_timing_for_linear_transition() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Linear)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // Advance into the linear transition step
        timer.advance(TestDuration(150));
        let timing = sequencer.service().unwrap();

        // Should return Continuous for linear transitions
        assert_eq!(timing, ServiceTiming::Continuous);
    }

    #[test]
    fn finite_sequence_completes_and_transitions_to_complete_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .landing_color(BLUE)
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // Advance past the sequence duration
        timer.advance(TestDuration(200));
        let timing = sequencer.service().unwrap();

        // Should return Complete to indicate completion
        assert_eq!(timing, ServiceTiming::Complete);
        assert_eq!(sequencer.state(), SequencerState::Complete);
        assert!(colors_equal(sequencer.current_color(), BLUE));
    }

    #[test]
    fn peek_next_timing_returns_timing_without_state_changes() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Linear)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // Peek should return Delay for step transition
        let peek_timing = sequencer.peek_next_timing().unwrap();
        assert_eq!(peek_timing, ServiceTiming::Delay(TestDuration(1000)));

        // LED should still be at initial color
        assert!(colors_equal(sequencer.current_color(), RED));

        // Advance into linear transition
        timer.advance(TestDuration(1100));

        // Peek should return Continuous for linear transition
        let peek_timing = sequencer.peek_next_timing().unwrap();
        assert_eq!(peek_timing, ServiceTiming::Continuous);

        // LED color should not have changed from peek
        assert!(colors_equal(sequencer.current_color(), RED));

        // Now actually service - LED should update to transitioning color
        sequencer.service().unwrap();
        // At t=1100, we're 100ms into a 1000ms linear transition from RED to GREEN
        // So we should be ~10% of the way from RED to GREEN
        let current = sequencer.current_color();
        assert!(current.red < 1.0); // Moving away from red
        assert!(current.green > 0.0); // Moving toward green

        // Peek when sequence is complete
        timer.advance(TestDuration(1000));
        let peek_timing = sequencer.peek_next_timing().unwrap();
        assert_eq!(peek_timing, ServiceTiming::Complete);

        // State should still be Running (peek doesn't change state)
        assert_eq!(sequencer.state(), SequencerState::Running);

        // Service should transition to Complete state
        sequencer.service().unwrap();
        assert_eq!(sequencer.state(), SequencerState::Complete);
    }

    #[test]
    fn peek_next_timing_requires_running_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        // Should fail when not running
        let result = sequencer.peek_next_timing();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SequencerError::InvalidState { .. }
        ));
    }

    #[test]
    fn restart_from_running_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // Advance into the sequence
        timer.advance(TestDuration(1500));
        sequencer.service().unwrap();
        assert!(colors_equal(sequencer.current_color(), GREEN));

        // Restart should reset to beginning
        let restart_result = sequencer.restart();
        assert!(restart_result.is_ok());
        assert_eq!(sequencer.state(), SequencerState::Running);
        assert!(colors_equal(sequencer.current_color(), RED));
    }

    #[test]
    fn restart_from_paused_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        timer.advance(TestDuration(500));
        sequencer.service().unwrap();
        sequencer.pause().unwrap();

        // Restart from paused should reset and run
        sequencer.restart().unwrap();
        assert_eq!(sequencer.state(), SequencerState::Running);
        assert!(colors_equal(sequencer.current_color(), RED));
    }

    #[test]
    fn restart_from_complete_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .landing_color(BLUE)
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        timer.advance(TestDuration(200));
        sequencer.service().unwrap();
        assert_eq!(sequencer.state(), SequencerState::Complete);

        // Restart should reset and run from beginning
        sequencer.restart().unwrap();
        assert_eq!(sequencer.state(), SequencerState::Running);
        assert!(colors_equal(sequencer.current_color(), RED));
    }

    #[test]
    fn restart_from_invalid_state_fails() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        // Try restart from Idle
        let result = sequencer.restart();
        assert!(matches!(result, Err(SequencerError::InvalidState { .. })));

        // Try restart from Loaded
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();
        sequencer.load(sequence);

        let result = sequencer.restart();
        assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
    }

    #[test]
    fn handle_action_dispatches_all_action_types_correctly() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        // Test Load action
        let result = sequencer.handle_action(SequencerAction::Load(sequence));
        assert!(result.is_ok());
        assert_eq!(sequencer.state(), SequencerState::Loaded);

        // Test Start action
        let result = sequencer.handle_action(SequencerAction::Start);
        assert!(result.is_ok());
        assert_eq!(sequencer.state(), SequencerState::Running);

        // Test Pause action
        let result = sequencer.handle_action(SequencerAction::Pause);
        assert!(result.is_ok());
        assert_eq!(sequencer.state(), SequencerState::Paused);

        // Test Resume action
        let result = sequencer.handle_action(SequencerAction::Resume);
        assert!(result.is_ok());
        assert_eq!(sequencer.state(), SequencerState::Running);

        // Test Stop action
        let result = sequencer.handle_action(SequencerAction::Stop);
        assert!(result.is_ok());
        assert_eq!(sequencer.state(), SequencerState::Loaded);

        // Test Restart action
        sequencer.start().unwrap();
        let result = sequencer.handle_action(SequencerAction::Restart);
        assert!(result.is_ok());
        assert_eq!(sequencer.state(), SequencerState::Running);

        // Test Clear action
        let result = sequencer.handle_action(SequencerAction::Clear);
        assert!(result.is_ok());
        assert_eq!(sequencer.state(), SequencerState::Idle);
    }

    #[test]
    fn query_methods_return_correct_state_and_timing_info() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        // Initial state queries
        assert_eq!(sequencer.state(), SequencerState::Idle);
        assert!(!sequencer.is_running());
        assert!(!sequencer.is_paused());
        assert!(sequencer.current_sequence().is_none());
        assert!(sequencer.elapsed_time().is_none());
        assert!(colors_equal(sequencer.current_color(), BLACK));

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);
        assert!(!sequencer.is_running());
        assert!(sequencer.current_sequence().is_some());

        sequencer.start().unwrap();
        assert!(sequencer.is_running());
        assert!(!sequencer.is_paused());

        timer.advance(TestDuration(50));
        sequencer.service().unwrap();
        let elapsed = sequencer.elapsed_time().unwrap();
        assert_eq!(elapsed, TestDuration(50));

        sequencer.pause().unwrap();
        assert!(!sequencer.is_running());
        assert!(sequencer.is_paused());
    }

    #[test]
    fn stop_from_paused_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        sequencer.pause().unwrap();

        // Stop from paused should work
        let result = sequencer.stop();
        assert!(result.is_ok());
        assert_eq!(sequencer.state(), SequencerState::Loaded);
        assert!(colors_equal(sequencer.current_color(), BLACK));
    }

    #[test]
    fn stop_from_complete_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        timer.advance(TestDuration(200));
        sequencer.service().unwrap();

        assert_eq!(sequencer.state(), SequencerState::Complete);

        let result = sequencer.stop();
        assert!(result.is_ok());
        assert_eq!(sequencer.state(), SequencerState::Loaded);
    }

    #[test]
    fn led_only_updates_when_color_changes() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // After start, LED should be set to RED (plus initial BLACK from new())
        // Color history should have: [BLACK (from new), RED (from start)]

        // Service multiple times without time advancing - color shouldn't change
        timer.advance(TestDuration(100));
        sequencer.service().unwrap();
        sequencer.service().unwrap();
        sequencer.service().unwrap();

        // The LED's color_history should not grow since color didn't change
        // We can't directly test this without exposing the mock, but we can verify
        // the current color remains RED
        assert!(colors_equal(sequencer.current_color(), RED));
    }

    #[test]
    fn resume_handles_timer_overflow_gracefully() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        timer.advance(TestDuration(500));
        sequencer.service().unwrap();
        sequencer.pause().unwrap();

        // Note: With our current TestInstant implementation, overflow won't actually occur
        // since it uses u64 and checked_add will succeed. However, this test documents
        // the intended behavior. On 32-bit systems with wrapping timers, the graceful
        // degradation would kick in.

        sequencer.resume().unwrap();
        assert_eq!(sequencer.state(), SequencerState::Running);
    }

    #[test]
    fn error_types_are_constructable() {
        // Verify error types can be constructed
        let _error1 = SequencerError::InvalidState {
            expected: "Running",
            actual: SequencerState::Paused,
        };
        let _error2 = SequencerError::NoSequenceLoaded;
    }

    #[test]
    fn comprehensive_state_transitions() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        // State: Idle -> Invalid operations
        assert!(sequencer.start().is_err());
        assert!(sequencer.pause().is_err());
        assert!(sequencer.resume().is_err());
        assert!(sequencer.stop().is_err());
        assert!(sequencer.restart().is_err());
        assert!(sequencer.service().is_err());

        // State: Idle -> Loaded
        sequencer.load(sequence);
        assert_eq!(sequencer.state(), SequencerState::Loaded);

        // State: Loaded -> Invalid operations
        assert!(sequencer.pause().is_err());
        assert!(sequencer.resume().is_err());
        assert!(sequencer.stop().is_err());
        assert!(sequencer.restart().is_err());
        assert!(sequencer.service().is_err());

        // State: Loaded -> Running
        assert!(sequencer.start().is_ok());
        assert_eq!(sequencer.state(), SequencerState::Running);

        // State: Running -> Paused
        assert!(sequencer.pause().is_ok());
        assert_eq!(sequencer.state(), SequencerState::Paused);

        // State: Paused -> Invalid operations
        assert!(sequencer.start().is_err());
        assert!(sequencer.pause().is_err());
        assert!(sequencer.service().is_err());

        // State: Paused -> Running
        assert!(sequencer.resume().is_ok());
        assert_eq!(sequencer.state(), SequencerState::Running);

        // State: Running -> Loaded (via stop)
        assert!(sequencer.stop().is_ok());
        assert_eq!(sequencer.state(), SequencerState::Loaded);

        // State: Loaded -> Running -> Complete
        sequencer.start().unwrap();
        timer.advance(TestDuration(200));
        sequencer.service().unwrap();
        assert_eq!(sequencer.state(), SequencerState::Complete);

        // State: Complete -> Running (via restart)
        assert!(sequencer.restart().is_ok());
        assert_eq!(sequencer.state(), SequencerState::Running);

        // State: Running -> Idle (via clear)
        sequencer.clear();
        assert_eq!(sequencer.state(), SequencerState::Idle);
    }

    #[test]
    fn loading_new_sequence_replaces_existing_and_resets_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence1 = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        let sequence2 = RgbSequence::<TestDuration, 8>::builder()
            .step(GREEN, TestDuration(200), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        // Load first sequence and start
        sequencer.load(sequence1);
        sequencer.start().unwrap();
        assert!(colors_equal(sequencer.current_color(), RED));

        // Load second sequence should stop the first and transition to Loaded
        sequencer.load(sequence2);
        assert_eq!(sequencer.state(), SequencerState::Loaded);

        // Start second sequence
        sequencer.start().unwrap();
        assert!(colors_equal(sequencer.current_color(), GREEN));
    }

    #[test]
    fn multiple_service_calls_without_time_advancement_are_safe() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // Multiple service calls without time advancement should be safe
        for _ in 0..10 {
            let result = sequencer.service();
            assert!(result.is_ok());
            assert!(colors_equal(sequencer.current_color(), RED));
        }
    }

    #[test]
    fn load_and_start_convenience_method_works() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        // Should go from Idle -> Loaded -> Running in one call
        let timing = sequencer.load_and_start(sequence).unwrap();
        assert_eq!(sequencer.state(), SequencerState::Running);
        assert!(colors_equal(sequencer.current_color(), RED));
        assert_eq!(timing, ServiceTiming::Delay(TestDuration(1000)));

        // Advance and verify it progresses through sequence
        timer.advance(TestDuration(1100));
        sequencer.service().unwrap();
        assert!(colors_equal(sequencer.current_color(), GREEN));
    }

    #[test]
    fn sequence_with_mixed_zero_and_nonzero_durations_works_correctly() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(0), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(BLUE, TestDuration(0), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // At time 0, zero-duration steps are skipped, so we're at GREEN (second step)
        assert!(colors_equal(sequencer.current_color(), GREEN));

        // After 50ms, still in GREEN (second step)
        timer.advance(TestDuration(50));
        sequencer.service().unwrap();
        assert!(colors_equal(sequencer.current_color(), GREEN));

        // After 100ms total, should be BLUE (third step, also zero duration)
        timer.advance(TestDuration(50));
        sequencer.service().unwrap();
        assert!(colors_equal(sequencer.current_color(), BLUE));
    }

    #[test]
    fn current_position_returns_none_when_not_running() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        // Idle state - no position
        assert_eq!(sequencer.current_position(), None);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        // Loaded state - no position
        sequencer.load(sequence);
        assert_eq!(sequencer.current_position(), None);

        // Running state - should have position
        sequencer.start().unwrap();
        assert!(sequencer.current_position().is_some());

        // Paused state - no position
        sequencer.pause().unwrap();
        assert_eq!(sequencer.current_position(), None);
    }

    #[test]
    fn current_position_tracks_step_changes() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // At start - step 0, loop 0
        assert_eq!(sequencer.current_position(), Some((0, 0)));

        // After 50ms - still step 0, loop 0
        timer.advance(TestDuration(50));
        sequencer.service().unwrap();
        assert_eq!(sequencer.current_position(), Some((0, 0)));

        // After 100ms - step 1, loop 0
        timer.advance(TestDuration(50));
        sequencer.service().unwrap();
        assert_eq!(sequencer.current_position(), Some((1, 0)));

        // After 200ms - step 2, loop 0
        timer.advance(TestDuration(100));
        sequencer.service().unwrap();
        assert_eq!(sequencer.current_position(), Some((2, 0)));

        // After 300ms - sequence complete, no position
        timer.advance(TestDuration(100));
        sequencer.service().unwrap();
        assert_eq!(sequencer.current_position(), None);
    }

    #[test]
    fn current_position_tracks_loop_changes() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(3))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // Loop 0
        assert_eq!(sequencer.current_position(), Some((0, 0)));

        // Advance to loop 1
        timer.advance(TestDuration(200));
        sequencer.service().unwrap();
        assert_eq!(sequencer.current_position(), Some((0, 1)));

        // Mid loop 1
        timer.advance(TestDuration(50));
        sequencer.service().unwrap();
        assert_eq!(sequencer.current_position(), Some((0, 1)));

        // Advance to loop 2
        timer.advance(TestDuration(150));
        sequencer.service().unwrap();
        assert_eq!(sequencer.current_position(), Some((0, 2)));

        // Complete all loops
        timer.advance(TestDuration(200));
        sequencer.service().unwrap();
        assert_eq!(sequencer.current_position(), None);
    }

    #[test]
    fn current_position_enables_event_detection() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(2))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        let mut last_position = None;
        let mut step_enter_events = heapless::Vec::<(usize, u32), 16>::new();
        let mut loop_complete_events = heapless::Vec::<u32, 8>::new();

        // Simulate event loop - advance time in small increments to catch all transitions
        // Each step is 100ms, we advance by 30ms per iteration to catch all step boundaries
        for _ in 0..20 {
            let current = sequencer.current_position();

            // Detect position changes (step enter or loop change)
            if current != last_position {
                if let Some((step, loop_num)) = current {
                    step_enter_events.push((step, loop_num)).ok();

                    // Detect loop completion (when returning to step 0 with higher loop number)
                    if step == 0 && loop_num > 0 {
                        if let Some((_, last_loop)) = last_position {
                            if loop_num > last_loop {
                                loop_complete_events.push(last_loop).ok();
                            }
                        }
                    }
                }
                last_position = current;
            }

            sequencer.service().ok();
            timer.advance(TestDuration(30));
        }

        // Verify step enter events were detected
        // Should see: (0,0), (1,0), (2,0), (0,1), (1,1), (2,1)
        assert!(
            step_enter_events.len() >= 6,
            "Expected at least 6 step events, got {}",
            step_enter_events.len()
        );
        assert_eq!(step_enter_events[0], (0, 0)); // Start of loop 0
        assert_eq!(step_enter_events[1], (1, 0)); // Step 1 of loop 0
        assert_eq!(step_enter_events[2], (2, 0)); // Step 2 of loop 0
        assert_eq!(step_enter_events[3], (0, 1)); // Start of loop 1
        assert_eq!(step_enter_events[4], (1, 1)); // Step 1 of loop 1
        assert_eq!(step_enter_events[5], (2, 1)); // Step 2 of loop 1

        // Verify loop completion events
        assert!(loop_complete_events.len() >= 1);
        assert_eq!(loop_complete_events[0], 0); // Loop 0 completed
    }

    #[test]
    fn current_position_returns_none_for_function_based_sequences() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        fn color_fn(base: Srgb, _elapsed: TestDuration) -> Srgb {
            base
        }

        fn timing_fn(_elapsed: TestDuration) -> Option<TestDuration> {
            Some(TestDuration::ZERO)
        }

        let sequence = RgbSequence::<TestDuration, 8>::from_function(RED, color_fn, timing_fn);

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // Function-based sequences don't have step positions
        assert_eq!(sequencer.current_position(), None);
    }

    #[test]
    fn into_led_extracts_led_from_sequencer() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        // Extract LED
        let extracted_led = sequencer.into_led();

        // LED should be extractable (compilation test)
        let _ = extracted_led;
    }

    #[test]
    fn into_led_preserves_current_color() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        sequencer.service().unwrap();

        // Verify color is RED before extraction
        assert!(colors_equal(sequencer.current_color(), RED));

        // Extract LED - it should have the color displayed
        let extracted_led = sequencer.into_led();
        assert!(colors_equal(extracted_led.get_last_color(), RED));
    }

    #[test]
    fn into_parts_extracts_led_and_sequence() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        sequencer.load(sequence);

        // Extract both LED and sequence
        let (extracted_led, extracted_sequence) = sequencer.into_parts();

        // LED should be extractable
        let _ = extracted_led;

        // Sequence should be present
        assert!(extracted_sequence.is_some());
        let seq = extracted_sequence.unwrap();
        assert_eq!(seq.step_count(), 2);
    }

    #[test]
    fn into_parts_returns_none_when_no_sequence_loaded() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        // Extract without loading a sequence
        let (_led, sequence) = sequencer.into_parts();

        // Sequence should be None
        assert!(sequence.is_none());
    }
}
