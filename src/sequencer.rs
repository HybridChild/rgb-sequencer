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

/// Current playback position within a sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Position {
    /// Current step index (0-based).
    pub step_index: usize,
    /// Current loop number (0-based).
    pub loop_number: u32,
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
    color_epsilon: f32,
    brightness: f32,
}

/// Default epsilon for floating-point color comparisons.
pub const DEFAULT_COLOR_EPSILON: f32 = 0.001;

/// Returns true if two colors are approximately equal within the given epsilon.
#[inline]
fn colors_approximately_equal(a: Srgb, b: Srgb, epsilon: f32) -> bool {
    (a.red - b.red).abs() < epsilon
        && (a.green - b.green).abs() < epsilon
        && (a.blue - b.blue).abs() < epsilon
}

impl<'t, I: TimeInstant, L: RgbLed, T: TimeSource<I>, const N: usize> RgbSequencer<'t, I, L, T, N> {
    /// Creates sequencer with LED off and default color epsilon.
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
            color_epsilon: DEFAULT_COLOR_EPSILON,
            brightness: 1.0,
        }
    }

    /// Creates sequencer with custom color epsilon threshold.
    pub fn with_epsilon(mut led: L, time_source: &'t T, epsilon: f32) -> Self {
        led.set_color(COLOR_OFF);

        Self {
            led,
            time_source,
            state: SequencerState::Idle,
            sequence: None,
            start_time: None,
            pause_start_time: None,
            current_color: COLOR_OFF,
            color_epsilon: epsilon,
            brightness: 1.0,
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

        // Apply brightness to the evaluated color
        let dimmed_color = Srgb::new(
            new_color.red * self.brightness,
            new_color.green * self.brightness,
            new_color.blue * self.brightness,
        );

        // Update LED only if color changed (using approximate equality for f32)
        if !colors_approximately_equal(dimmed_color, self.current_color, self.color_epsilon) {
            self.led.set_color(dimmed_color);
            self.current_color = dimmed_color;
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

    /// Returns the current color epsilon threshold.
    #[inline]
    pub fn color_epsilon(&self) -> f32 {
        self.color_epsilon
    }

    /// Sets the color epsilon threshold.
    ///
    /// Controls the sensitivity of color change detection.
    #[inline]
    pub fn set_color_epsilon(&mut self, epsilon: f32) {
        self.color_epsilon = epsilon;
    }

    /// Returns current brightness multiplier (0.0-1.0).
    #[inline]
    pub fn brightness(&self) -> f32 {
        self.brightness
    }

    /// Sets global brightness multiplier.
    #[inline]
    pub fn set_brightness(&mut self, brightness: f32) {
        self.brightness = brightness.clamp(0.0, 1.0);
    }

    /// Returns current playback position.
    ///
    /// Returns `None` if not running or sequence is function-based.
    #[inline]
    pub fn current_position(&self) -> Option<Position> {
        if self.state != SequencerState::Running {
            return None;
        }

        let sequence = self.sequence.as_ref()?;
        let start_time = self.start_time?;
        let current_time = self.time_source.now();
        let elapsed = current_time.duration_since(start_time);

        let step_position = sequence.find_step_position(elapsed)?;
        Some(Position {
            step_index: step_position.step_index,
            loop_number: step_position.current_loop,
        })
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
