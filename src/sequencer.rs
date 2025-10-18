use crate::sequence::RgbSequence;
use crate::time::{TimeDuration, TimeInstant};
use crate::{COLOR_OFF};
use palette::Srgb;

/// Trait for abstracting RGB LED hardware.
///
/// Implement this for your LED hardware (GPIO, PWM, SPI, etc.) to allow
/// the sequencer to control it.
pub trait RgbLed {
    /// Sets the LED to the specified RGB color.
    ///
    /// Should be infallible - handle errors internally if needed.
    fn set_color(&mut self, color: Srgb);
}

/// Trait for abstracting time sources.
///
/// Allows the sequencer to query current time from different systems
/// (Embassy, std, custom timers, etc.).
pub trait TimeSource<I: TimeInstant> {
    /// Returns the current time instant.
    fn now(&self) -> I;
}

/// The current state of an RGB sequencer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequencerState {
    /// No sequence loaded, LED off.
    Idle,
    /// Sequence loaded but not started.
    Loaded,
    /// Sequence actively running.
    Running,
    /// Sequence paused at current color.
    Paused,
    /// Finite sequence completed, displaying final color.
    Complete,
}

/// Errors that can occur during sequencer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequencerError {
    /// Operation called from an invalid state.
    InvalidState {
        expected: &'static str,
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

impl<'t, I: TimeInstant, L: RgbLed, T: TimeSource<I>, const N: usize>
    RgbSequencer<'t, I, L, T, N>
{
    /// Creates a new idle sequencer with LED turned off.
    pub fn new(led: L, time_source: &'t T) -> Self {
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
    /// * `Ok(Some(duration))` - Time until next service
    /// * `Ok(None)` - Sequence completed immediately
    /// * `Err` - Invalid state or no sequence loaded
    pub fn start(&mut self) -> Result<Option<I::Duration>, SequencerError> {
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
    pub fn restart(&mut self) -> Result<Option<I::Duration>, SequencerError> {
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
    /// * `Ok(Some(Duration::ZERO))` - Linear transition, service again at desired frame rate
    /// * `Ok(Some(duration))` - Step transition, service again after this duration
    /// * `Ok(None)` - Sequence complete, transitions to `Complete` state
    /// * `Err` - Invalid state
    pub fn service(&mut self) -> Result<Option<I::Duration>, SequencerError> {
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

        // Get the color (always returns Some now)
        let new_color = sequence.color_at(elapsed);
        
        // Update LED if color changed
        if new_color != self.current_color {
            self.led.set_color(new_color);
            self.current_color = new_color;
        }

        // Check if sequence is complete
        if sequence.is_complete(elapsed) {
            self.state = SequencerState::Complete;
            return Ok(None);
        }

        Ok(self.calculate_next_service_time(elapsed))
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
    pub fn resume(&mut self) -> Result<Option<I::Duration>, SequencerError> {
        if self.state != SequencerState::Paused {
            return Err(SequencerError::InvalidState {
                expected: "Paused",
                actual: self.state,
            });
        }

        let pause_start = self.pause_start_time.unwrap();
        let current_time = self.time_source.now();
        let pause_duration = current_time.duration_since(pause_start);

        // Add the pause duration to start time
        // This keeps the sequence at the same position it was at when paused
        let old_start = self.start_time.unwrap();
        self.start_time = old_start.checked_add(pause_duration);

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

    /// Calculates when to service next based on current position.
    fn calculate_next_service_time(&self, elapsed: I::Duration) -> Option<I::Duration> {
        if self.is_in_linear_transition(elapsed) {
            Some(I::Duration::ZERO)
        } else {
            self.time_until_next_step(elapsed)
        }
    }

    /// Checks if currently in a linear transition.
    fn is_in_linear_transition(&self, elapsed: I::Duration) -> bool {
        let sequence = self.sequence.as_ref().unwrap();
        
        let loop_duration = self.loop_duration();
        if loop_duration.as_millis() == 0 {
            return false;
        }

        // Check if sequence is complete
        if let crate::types::LoopCount::Finite(count) = sequence.loop_count() {
            let total_duration_millis = loop_duration.as_millis() * (count as u64);
            if elapsed.as_millis() >= total_duration_millis {
                return false;
            }
        }

        // Find current step
        let time_in_loop_millis = elapsed.as_millis() % loop_duration.as_millis();
        let time_in_loop = I::Duration::from_millis(time_in_loop_millis);

        let mut accumulated_time = I::Duration::ZERO;
        for i in 0..sequence.step_count() {
            let step_duration = self.get_step_duration(i);
            let step_end_time = I::Duration::from_millis(
                accumulated_time.as_millis() + step_duration.as_millis(),
            );

            if time_in_loop.as_millis() < step_end_time.as_millis() {
                return self.is_step_linear(i);
            }

            accumulated_time = step_end_time;
        }

        false
    }

    /// Calculates time until the next step begins.
    fn time_until_next_step(&self, elapsed: I::Duration) -> Option<I::Duration> {
        let sequence = self.sequence.as_ref().unwrap();
        let loop_duration = self.loop_duration();
        
        if loop_duration.as_millis() == 0 {
            return None;
        }

        // Check if finite sequence is complete
        if let crate::types::LoopCount::Finite(count) = sequence.loop_count() {
            let total_duration_millis = loop_duration.as_millis() * (count as u64);
            if elapsed.as_millis() >= total_duration_millis {
                return None;
            }
        }

        let time_in_loop_millis = elapsed.as_millis() % loop_duration.as_millis();
        let time_in_loop = I::Duration::from_millis(time_in_loop_millis);

        // Find when current step ends
        let mut accumulated_time = I::Duration::ZERO;
        for i in 0..sequence.step_count() {
            let step_duration = self.get_step_duration(i);
            let step_end_time = I::Duration::from_millis(
                accumulated_time.as_millis() + step_duration.as_millis(),
            );

            if time_in_loop.as_millis() < step_end_time.as_millis() {
                return Some(step_end_time.saturating_sub(time_in_loop));
            }

            accumulated_time = step_end_time;
        }

        // End of loop
        Some(loop_duration.saturating_sub(time_in_loop))
    }

    /// Returns the duration of one complete loop through all steps.
    fn loop_duration(&self) -> I::Duration {
        let sequence = self.sequence.as_ref().unwrap();
        let mut total_millis = 0u64;
        for i in 0..sequence.step_count() {
            total_millis += self.get_step_duration(i).as_millis();
        }
        I::Duration::from_millis(total_millis)
    }

    /// Gets the duration of a step by index.
    fn get_step_duration(&self, index: usize) -> I::Duration {
        let sequence = self.sequence.as_ref().unwrap();
        sequence.get_step(index).map(|s| s.duration).unwrap_or(I::Duration::ZERO)
    }

    /// Checks if a step uses linear transition.
    fn is_step_linear(&self, index: usize) -> bool {
        let sequence = self.sequence.as_ref().unwrap();
        sequence.get_step(index).map(|s| s.transition == crate::types::TransitionStyle::Linear).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::RgbSequence;
    use crate::types::{LoopCount, TransitionStyle};
    use crate::time::{TimeDuration, TimeInstant};
    use palette::Srgb;
    use heapless::Vec;

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

        fn last_color(&self) -> Srgb {
            self.current_color
        }

        fn color_change_count(&self) -> usize {
            self.color_history.len()
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

        fn set_time(&self, time: TestInstant) {
            self.current_time.set(time);
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
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        // Try to start from Idle state (no sequence loaded)
        let result = sequencer.start();
        assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
    }

    #[test]
    fn pause_requires_running_state() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
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
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
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
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .build()
            .unwrap();

        sequencer.load(sequence);

        // Try to service from Loaded state
        let result = sequencer.service();
        assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
    }

    #[test]
    fn load_and_start_updates_led() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
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
    fn service_through_multiple_steps() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
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
    fn pause_resume_maintains_position() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
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
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
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
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
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
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .step(GREEN, TestDuration(500), TransitionStyle::Step)
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        let delay = sequencer.start().unwrap();

        // Should return the remaining time in the first step (1000ms)
        assert_eq!(delay, Some(TestDuration(1000)));
    }

    #[test]
    fn service_returns_zero_for_linear_transition() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(1000), TransitionStyle::Linear)
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // Advance into the linear transition step
        timer.advance(TestDuration(150));
        let delay = sequencer.service().unwrap();

        // Should return ZERO for linear transitions
        assert_eq!(delay, Some(TestDuration::ZERO));
    }

    #[test]
    fn finite_sequence_completes_and_returns_none() {
        let led = MockLed::new();
        let timer = MockTimeSource::new();
        let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .loop_count(LoopCount::Finite(1))
            .landing_color(BLUE)
            .build()
            .unwrap();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        // Advance past the sequence duration
        timer.advance(TestDuration(200));
        let result = sequencer.service().unwrap();

        // Should return None to indicate completion
        assert_eq!(result, None);
        assert_eq!(sequencer.get_state(), SequencerState::Complete);
        assert!(colors_equal(sequencer.current_color(), BLUE));
    }
}
