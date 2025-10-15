use crate::time::TimeDuration;
use crate::types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};
use heapless::Vec;
use palette::{Mix, rgb::Rgb};

/// An RGB color sequence with precise timing and transitions.
///
/// Defines a complete animation sequence consisting of multiple steps, each with
/// a target color, duration, and transition style. Sequences can loop a finite
/// number of times or indefinitely, and optionally specify a landing color to
/// display after completion.
///
/// # Type Parameters
/// * `D` - The duration type (e.g., `embassy_time::Duration`)
/// * `N` - Maximum number of steps this sequence can hold
///
/// # Examples
/// ```ignore
/// use rgb_sequencer::{RgbSequence, TransitionStyle, LoopCount};
/// use palette::Rgb;
/// use embassy_time::Duration;
///
/// let seq = RgbSequence::<_, 16>::new()
///     .step(Rgb::new(255, 0, 0), Duration::from_millis(500), TransitionStyle::Linear)
///     .step(Rgb::new(0, 0, 255), Duration::from_millis(300), TransitionStyle::Step)
///     .loop_count(LoopCount::Finite(3))
///     .landing_color(Rgb::new(0, 255, 0))
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct RgbSequence<D: TimeDuration, const N: usize> {
    steps: Vec<SequenceStep<D>, N>,
    loop_count: LoopCount,
    landing_color: Option<Rgb<u8>>,
}

impl<D: TimeDuration, const N: usize> RgbSequence<D, N> {
    /// Creates a new sequence builder.
    ///
    /// Returns a `SequenceBuilder` that provides a fluent API for constructing
    /// sequences with validation.
    pub fn new() -> SequenceBuilder<D, N> {
        SequenceBuilder::new()
    }

    /// Calculates the color at a given elapsed time since the sequence started.
    ///
    /// This is the core method that determines what color should be displayed
    /// based on how much time has passed. It handles step progression, loop
    /// counting, transition interpolation, and completion behavior.
    ///
    /// # Arguments
    /// * `elapsed` - Time elapsed since the sequence started
    ///
    /// # Returns
    /// * `Some(color)` - The color to display at this time
    /// * `None` - Should never occur in normal operation (internal error)
    ///
    /// # Behavior
    /// - At time 0: Returns the first step's color
    /// - During sequence execution: Returns interpolated color based on current step
    /// - After finite loops complete with landing color: Returns landing color
    /// - After finite loops complete without landing color: Returns last step's color
    /// - For infinite loops: Never completes, always returns current color based on time wrapped modulo loop duration
    /// - Edge case - zero-duration sequences: Returns first step's color at time 0, then landing color (if set) or last step's color for all subsequent times
    pub fn color_at(&self, elapsed: D) -> Option<Rgb<u8>> {
        // Calculate total duration of one loop through all steps
        let loop_millis = self.total_duration().as_millis();
        
        // Handle edge case: all steps have zero duration
        if loop_millis == 0 {
            if elapsed.as_millis() == 0 {
                return Some(self.steps[0].color);
            }
            // Sequence completes instantly - return final color
            return Some(
                self.landing_color
                    .unwrap_or(self.steps.last().unwrap().color),
            );
        }

        let elapsed_millis = elapsed.as_millis();
        
        // Check if finite sequence has completed
        if let LoopCount::Finite(count) = self.loop_count {
            let total_duration_millis = loop_millis * (count as u64);
            if elapsed_millis >= total_duration_millis {
                // Sequence has completed all loops
                return Some(
                    self.landing_color
                        .unwrap_or(self.steps.last().unwrap().color),
                );
            }
        }
        
        // Calculate time within current loop (works for both finite and infinite)
        let time_in_loop = D::from_millis(elapsed_millis % loop_millis);

        // Find which step we're currently in and how far through it
        let mut accumulated_time = D::ZERO;
        for (step_idx, step) in self.steps.iter().enumerate() {
            let step_end_time = D::from_millis(
                accumulated_time.as_millis() + step.duration.as_millis(),
            );

            if time_in_loop.as_millis() < step_end_time.as_millis() {
                // We're in this step
                let time_in_step = D::from_millis(
                    time_in_loop.as_millis() - accumulated_time.as_millis(),
                );

                return Some(self.calculate_step_color(step_idx, time_in_step));
            }

            accumulated_time = step_end_time;
        }

        // If we get here, we're at or past the end of the loop
        // This should be caught above, but as a fallback return last color
        Some(self.steps.last().unwrap().color)
    }

    /// Calculates the color for a specific step at a given time within that step.
    fn calculate_step_color(&self, step_idx: usize, time_in_step: D) -> Rgb<u8> {
        let step = &self.steps[step_idx];

        match step.transition {
            TransitionStyle::Step => {
                // Instant transition - just return the step's color
                step.color
            }
            TransitionStyle::Linear => {
                // Linear interpolation from previous color to this step's color
                let previous_color = if step_idx == 0 {
                    // First step - transition from the last step's color (for smooth looping)
                    self.steps.last().unwrap().color
                } else {
                    self.steps[step_idx - 1].color
                };

                // Calculate interpolation factor (0.0 to 1.0)
                let duration_millis = step.duration.as_millis();
                if duration_millis == 0 {
                    // Should be caught by validation, but handle anyway
                    return step.color;
                }

                let time_millis = time_in_step.as_millis();
                let progress = (time_millis as f32) / (duration_millis as f32);
                let progress = progress.clamp(0.0, 1.0);

                // Use palette's Mix trait for color interpolation
                previous_color.mix(step.color, progress)
            }
        }
    }

    /// Calculates the total duration of one complete loop through all steps.
    fn total_duration(&self) -> D {
        let total_millis: u64 = self.steps.iter().map(|s| s.duration.as_millis()).sum();
        D::from_millis(total_millis)
    }

    /// Returns the number of steps in this sequence.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Returns the loop count configuration.
    pub fn loop_count(&self) -> LoopCount {
        self.loop_count
    }

    /// Returns the landing color if one is configured.
    pub fn landing_color(&self) -> Option<Rgb<u8>> {
        self.landing_color
    }
}

/// Builder for constructing validated RGB sequences.
///
/// Provides a fluent API for adding steps and configuring sequence behavior.
/// Validates the sequence when `build()` is called.
#[derive(Debug)]
pub struct SequenceBuilder<D: TimeDuration, const N: usize> {
    steps: Vec<SequenceStep<D>, N>,
    loop_count: LoopCount,
    landing_color: Option<Rgb<u8>>,
}

impl<D: TimeDuration, const N: usize> SequenceBuilder<D, N> {
    /// Creates a new empty sequence builder.
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            loop_count: LoopCount::default(), // Defaults to Finite(1)
            landing_color: None,
        }
    }

    /// Adds a step to the sequence.
    ///
    /// Steps are executed in the order they are added. Each step defines a target
    /// color, how long to spend on that step, and how to transition to it.
    ///
    /// # Arguments
    /// * `color` - The target RGB color for this step
    /// * `duration` - How long this step lasts (including transition time)
    /// * `transition` - How to transition to this color from the previous state
    ///
    /// # Returns
    /// The builder for method chaining, or the builder unchanged if capacity is exceeded.
    ///
    /// # Note
    /// Zero-duration steps are only valid with `Step` transitions. This will be
    /// validated when `build()` is called.
    pub fn step(
        mut self,
        color: Rgb<u8>,
        duration: D,
        transition: TransitionStyle,
    ) -> Self {
        if self.steps.push(SequenceStep::new(color, duration, transition)).is_err() {
            panic!("sequence capacity exceeded");
        }
        self
    }

    /// Sets how many times the sequence should loop.
    ///
    /// Default is `LoopCount::Finite(1)` (play once).
    ///
    /// # Arguments
    /// * `count` - Either `Finite(n)` to loop n times, or `Infinite` to loop forever
    pub fn loop_count(mut self, count: LoopCount) -> Self {
        self.loop_count = count;
        self
    }

    /// Sets the color to display after the sequence completes.
    ///
    /// Only relevant for finite loop counts. If not set, the sequence will hold
    /// the last step's color after completion.
    ///
    /// # Arguments
    /// * `color` - The RGB color to display after all loops complete
    pub fn landing_color(mut self, color: Rgb<u8>) -> Self {
        self.landing_color = Some(color);
        self
    }

    /// Builds and validates the sequence.
    ///
    /// # Returns
    /// * `Ok(RgbSequence)` - A valid sequence ready for use
    /// * `Err(SequenceError)` - If validation fails
    ///
    /// # Errors
    /// * `EmptySequence` - No steps were added
    /// * `ZeroDurationWithLinear` - A step has zero duration with Linear transition
    /// * `CapacityExceeded` - More steps added than capacity `N` allows
    pub fn build(self) -> Result<RgbSequence<D, N>, SequenceError> {
        // Validate: must have at least one step
        if self.steps.is_empty() {
            return Err(SequenceError::EmptySequence);
        }

        // Validate: zero-duration steps must use Step transition
        for step in &self.steps {
            if step.duration.as_millis() == 0
                && step.transition == TransitionStyle::Linear
            {
                return Err(SequenceError::ZeroDurationWithLinear);
            }
        }

        Ok(RgbSequence {
            steps: self.steps,
            loop_count: self.loop_count,
            landing_color: self.landing_color,
        })
    }
}

impl<D: TimeDuration, const N: usize> Default for SequenceBuilder<D, N> {
    fn default() -> Self {
        Self::new()
    }
}
