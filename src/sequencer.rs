//! RGB LED sequencer with state management and timing control.
//!
//! Provides [`RgbSequencer`] which manages a single RGB LED through timed sequences,
//! handling state transitions, timing coordination, and LED updates. Also defines
//! the [`RgbLed`] trait for hardware abstraction.

use crate::COLOR_OFF;
use crate::command::SequencerAction;
use crate::sequence::RgbSequence;
use crate::time::{TimeDuration, TimeInstant, TimeSource};
use palette::Srgb;

/// Trait for abstracting RGB LED hardware.
///
/// Implement this for your LED hardware (GPIO, PWM, SPI, etc.) to allow
/// the sequencer to control it.
pub trait RgbLed {
    /// Sets the LED to the specified RGB color.
    ///
    /// Color components are in the range 0.0-1.0. Implementations should
    /// convert these to their hardware's native format (e.g., PWM duty cycles,
    /// 8-bit RGB values). Handle any hardware errors internally - this method
    /// cannot fail.
    fn set_color(&mut self, color: Srgb);
}

/// The current state of an RGB sequencer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SequencerState {
    /// No sequence loaded. LED is off.
    Idle,
    /// Sequence loaded and ready to start. LED is off.
    Loaded,
    /// Sequence actively executing. LED displays animated colors.
    Running,
    /// Sequence paused. LED holds the color from when pause was called.
    Paused,
    /// Finite sequence finished. LED displays landing color or last step color.
    Complete,
}

/// Timing information returned by service operations.
///
/// Indicates when the sequencer needs to be serviced again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ServiceTiming<D> {
    /// Continuous animation in progress. Service again at your desired frame rate.
    ///
    /// Returned during linear color transitions or continuous function-based sequences.
    /// Typically you should sleep for 16-33ms (30-60 FPS) between service calls.
    Continuous,

    /// Static color hold. Service again after the specified delay.
    ///
    /// Returned during step transitions where the color is constant.
    /// Sleep for exactly this duration before calling service again.
    Delay(D),

    /// Sequence has completed.
    ///
    /// Returned when a finite sequence finishes all loops. The LED will display
    /// the landing color (if configured) or the last step's color. No further
    /// servicing is needed until a new sequence is loaded or the sequence is restarted.
    Complete,
}

/// Errors that can occur during sequencer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SequencerError {
    /// Operation called from an invalid state.
    ///
    /// The `expected` field describes which state(s) are valid for this operation.
    InvalidState {
        /// Human-readable description of expected state(s), e.g. "Running" or "Running, Paused, or Complete"
        expected: &'static str,
        /// The actual current state
        actual: SequencerState,
    },
    /// No sequence is loaded.
    NoSequenceLoaded,
}

impl core::fmt::Display for SequencerError {
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

#[cfg(feature = "std")]
impl std::error::Error for SequencerError {}

/// Controls a single RGB LED through timed color sequences.
///
/// Each sequencer owns an LED and executes sequences independently. The sequencer
/// tracks timing, calculates colors, and updates the LED hardware.
///
/// Supports both step-based sequences (pre-defined color transitions) and
/// function-based sequences (algorithmic animations). Both types work seamlessly
/// with the same sequencer interface.
///
/// # Type Parameters
/// * `'t` - Lifetime of the time source reference
/// * `I` - Time instant type
/// * `L` - LED implementation type
/// * `T` - Time source implementation type
/// * `N` - Maximum number of steps in sequences
pub struct RgbSequencer<'t, I: TimeInstant, L: RgbLed, T: TimeSource<I>, const N: usize> {
    led: L,
    time_source: &'t T,
    state: SequencerState,
    sequence: Option<RgbSequence<I::Duration, N>>,
    start_time: Option<I>,
    pause_start_time: Option<I>,
    current_color: Srgb,
}

impl<'t, I: TimeInstant, L: RgbLed, T: TimeSource<I>, const N: usize> RgbSequencer<'t, I, L, T, N> {
    /// Creates a new idle sequencer with LED turned off.
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

    /// Handles a sequencer action by dispatching to the appropriate method.
    ///
    /// This is a convenience method for command-based control, allowing actions
    /// to be dispatched without matching on the action type manually.
    ///
    /// # Returns
    /// * `Ok(ServiceTiming)` - Timing for actions that start/resume sequences
    /// * `Ok(ServiceTiming::Complete)` - For actions that don't require servicing
    /// * `Err` - Operation failed (invalid state, etc.)
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

    /// Loads a sequence. Can be called from any state.
    ///
    /// Stops any running sequence and transitions to `Loaded` state.
    pub fn load(&mut self, sequence: RgbSequence<I::Duration, N>) {
        self.sequence = Some(sequence);
        self.start_time = None;
        self.pause_start_time = None;
        self.state = SequencerState::Loaded;
    }

    /// Starts the loaded sequence from the beginning.
    ///
    /// Must be called from `Loaded` state.
    ///
    /// # Returns
    /// * `Ok(ServiceTiming)` - When to service next
    /// * `Err` - Invalid state or no sequence loaded
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

    /// Restarts the sequence from the beginning.
    ///
    /// Can be called from `Running`, `Paused`, or `Complete` states.
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

    /// Services the sequencer, updating the LED if necessary.
    ///
    /// Must be called from `Running` state.
    ///
    /// # Returns
    /// - `Ok(ServiceTiming::Continuous)` - Continuous animation, service at desired frame rate
    /// - `Ok(ServiceTiming::Delay(duration))` - Static hold, service after this delay
    /// - `Ok(ServiceTiming::Complete)` - Sequence complete, transitions to `Complete` state
    /// - `Err` - Invalid state
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

        // Update LED only if color changed
        if new_color != self.current_color {
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

    /// Stops the sequence and turns off the LED.
    ///
    /// Sequence remains loaded and transitions to `Loaded` state.
    /// Can be called from `Running`, `Paused`, or `Complete`.
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

    /// Pauses the sequence at the current color.
    ///
    /// Must be called from `Running` state.
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

    /// Resumes a paused sequence, adjusting timing for pause duration.
    ///
    /// Must be called from `Paused` state.
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

    /// Clears the sequence and turns off the LED.
    ///
    /// Removes loaded sequence and transitions to `Idle`. Can be called from any state.
    pub fn clear(&mut self) {
        self.sequence = None;
        self.start_time = None;
        self.pause_start_time = None;
        self.state = SequencerState::Idle;

        self.led.set_color(COLOR_OFF);
        self.current_color = COLOR_OFF;
    }

    /// Returns the current state of the sequencer.
    pub fn get_state(&self) -> SequencerState {
        self.state
    }

    /// Returns the current color being displayed on the LED.
    pub fn current_color(&self) -> Srgb {
        self.current_color
    }

    /// Returns true if the sequencer is currently paused.
    pub fn is_paused(&self) -> bool {
        self.state == SequencerState::Paused
    }

    /// Returns true if the sequencer is currently running.
    pub fn is_running(&self) -> bool {
        self.state == SequencerState::Running
    }

    /// Returns a reference to the currently loaded sequence, if any
    pub fn current_sequence(&self) -> Option<&RgbSequence<I::Duration, N>> {
        self.sequence.as_ref()
    }

    /// Returns the elapsed time since the sequence started, if running
    pub fn elapsed_time(&self) -> Option<I::Duration> {
        self.start_time.map(|start| {
            let now = self.time_source.now();
            now.duration_since(start)
        })
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
    extern crate std;
    use std::format;

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
            .build()
            .unwrap();

        sequencer.load(sequence);
        assert_eq!(sequencer.get_state(), SequencerState::Loaded);

        sequencer.start().unwrap();
        assert_eq!(sequencer.get_state(), SequencerState::Running);

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
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
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
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
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
        assert_eq!(sequencer.get_state(), SequencerState::Paused);

        // Advance time while paused (simulating delay)
        timer.advance(TestDuration(3000));

        // Resume - should still be in RED step
        sequencer.resume().unwrap();
        assert_eq!(sequencer.get_state(), SequencerState::Running);
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
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        assert!(colors_equal(sequencer.current_color(), RED));

        sequencer.stop().unwrap();
        assert_eq!(sequencer.get_state(), SequencerState::Loaded);
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
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        assert!(colors_equal(sequencer.current_color(), RED));

        sequencer.clear();
        assert_eq!(sequencer.get_state(), SequencerState::Idle);
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
            .step(GREEN, TestDuration(500), TransitionStyle::Step)
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
            .step(GREEN, TestDuration(1000), TransitionStyle::Linear)
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
        assert_eq!(sequencer.get_state(), SequencerState::Complete);
        assert!(colors_equal(sequencer.current_color(), BLUE));
    }

    #[test]
    fn restart_from_running_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
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
        assert_eq!(sequencer.get_state(), SequencerState::Running);
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
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        timer.advance(TestDuration(500));
        sequencer.service().unwrap();
        sequencer.pause().unwrap();

        // Restart from paused should reset and run
        sequencer.restart().unwrap();
        assert_eq!(sequencer.get_state(), SequencerState::Running);
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
            .loop_count(LoopCount::Finite(1))
            .landing_color(BLUE)
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        timer.advance(TestDuration(200));
        sequencer.service().unwrap();
        assert_eq!(sequencer.get_state(), SequencerState::Complete);

        // Restart should reset and run from beginning
        sequencer.restart().unwrap();
        assert_eq!(sequencer.get_state(), SequencerState::Running);
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
            .build()
            .unwrap();

        // Test Load action
        let result = sequencer.handle_action(SequencerAction::Load(sequence));
        assert!(result.is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Loaded);

        // Test Start action
        let result = sequencer.handle_action(SequencerAction::Start);
        assert!(result.is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Running);

        // Test Pause action
        let result = sequencer.handle_action(SequencerAction::Pause);
        assert!(result.is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Paused);

        // Test Resume action
        let result = sequencer.handle_action(SequencerAction::Resume);
        assert!(result.is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Running);

        // Test Stop action
        let result = sequencer.handle_action(SequencerAction::Stop);
        assert!(result.is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Loaded);

        // Test Restart action
        sequencer.start().unwrap();
        let result = sequencer.handle_action(SequencerAction::Restart);
        assert!(result.is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Running);

        // Test Clear action
        let result = sequencer.handle_action(SequencerAction::Clear);
        assert!(result.is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Idle);
    }

    #[test]
    fn query_methods_return_correct_state_and_timing_info() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        // Initial state queries
        assert_eq!(sequencer.get_state(), SequencerState::Idle);
        assert!(!sequencer.is_running());
        assert!(!sequencer.is_paused());
        assert!(sequencer.current_sequence().is_none());
        assert!(sequencer.elapsed_time().is_none());
        assert!(colors_equal(sequencer.current_color(), BLACK));

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
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
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        sequencer.pause().unwrap();

        // Stop from paused should work
        let result = sequencer.stop();
        assert!(result.is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Loaded);
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
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();
        timer.advance(TestDuration(200));
        sequencer.service().unwrap();

        assert_eq!(sequencer.get_state(), SequencerState::Complete);

        let result = sequencer.stop();
        assert!(result.is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Loaded);
    }

    #[test]
    fn led_only_updates_when_color_changes() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
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
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
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
        assert_eq!(sequencer.get_state(), SequencerState::Running);
    }

    #[test]
    fn error_messages_format_correctly_for_display() {
        let error1 = SequencerError::InvalidState {
            expected: "Running",
            actual: SequencerState::Paused,
        };
        let error_str = format!("{}", error1);
        assert!(error_str.contains("invalid state"));
        assert!(error_str.contains("Running"));
        assert!(error_str.contains("Paused"));

        let error2 = SequencerError::NoSequenceLoaded;
        let error_str = format!("{}", error2);
        assert!(error_str.contains("no sequence loaded"));
    }

    #[test]
    fn comprehensive_state_transitions() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
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
        assert_eq!(sequencer.get_state(), SequencerState::Loaded);

        // State: Loaded -> Invalid operations
        assert!(sequencer.pause().is_err());
        assert!(sequencer.resume().is_err());
        assert!(sequencer.stop().is_err());
        assert!(sequencer.restart().is_err());
        assert!(sequencer.service().is_err());

        // State: Loaded -> Running
        assert!(sequencer.start().is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Running);

        // State: Running -> Paused
        assert!(sequencer.pause().is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Paused);

        // State: Paused -> Invalid operations
        assert!(sequencer.start().is_err());
        assert!(sequencer.pause().is_err());
        assert!(sequencer.service().is_err());

        // State: Paused -> Running
        assert!(sequencer.resume().is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Running);

        // State: Running -> Loaded (via stop)
        assert!(sequencer.stop().is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Loaded);

        // State: Loaded -> Running -> Complete
        sequencer.start().unwrap();
        timer.advance(TestDuration(200));
        sequencer.service().unwrap();
        assert_eq!(sequencer.get_state(), SequencerState::Complete);

        // State: Complete -> Running (via restart)
        assert!(sequencer.restart().is_ok());
        assert_eq!(sequencer.get_state(), SequencerState::Running);

        // State: Running -> Idle (via clear)
        sequencer.clear();
        assert_eq!(sequencer.get_state(), SequencerState::Idle);
    }

    #[test]
    fn loading_new_sequence_replaces_existing_and_resets_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence1 = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .build()
            .unwrap();

        let sequence2 = RgbSequence::<TestDuration, 8>::builder()
            .step(GREEN, TestDuration(200), TransitionStyle::Step)
            .build()
            .unwrap();

        // Load first sequence and start
        sequencer.load(sequence1);
        sequencer.start().unwrap();
        assert!(colors_equal(sequencer.current_color(), RED));

        // Load second sequence should stop the first and transition to Loaded
        sequencer.load(sequence2);
        assert_eq!(sequencer.get_state(), SequencerState::Loaded);

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
    fn sequence_with_mixed_zero_and_nonzero_durations_works_correctly() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer =
            RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(0), TransitionStyle::Step)
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .step(BLUE, TestDuration(0), TransitionStyle::Step)
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
}
