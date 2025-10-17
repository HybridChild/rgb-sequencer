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

        if let Some(new_color) = sequence.color_at(elapsed) {
            // Update LED only if color changed
            if new_color != self.current_color {
                self.led.set_color(new_color);
                self.current_color = new_color;
            }

            Ok(self.calculate_next_service_time(elapsed))
        } else {
            // Sequence completed
            self.state = SequencerState::Complete;
            Ok(None)
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
