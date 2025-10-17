use crate::time::TimeDuration;
use crate::types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};
use heapless::Vec;
use palette::{Mix, Srgb};

/// An RGB color sequence with precise timing and transitions.
///
/// Defines a complete animation sequence consisting of multiple steps, each with
/// a target color, duration, and transition style. Sequences can loop a finite
/// number of times or indefinitely, and optionally specify a landing color to
/// display after completion.
///
/// Colors are represented as `Srgb<f32>` (0.0-1.0 range) for accurate interpolation.
///
/// # Type Parameters
/// * `D` - The duration type (e.g., `embassy_time::Duration`)
/// * `N` - Maximum number of steps this sequence can hold
#[derive(Debug, Clone)]
pub struct RgbSequence<D: TimeDuration, const N: usize> {
    steps: Vec<SequenceStep<D>, N>,
    loop_count: LoopCount,
    landing_color: Option<Srgb>,
}

impl<D: TimeDuration, const N: usize> RgbSequence<D, N> {
    /// Creates a new sequence builder.
    pub fn new() -> SequenceBuilder<D, N> {
        SequenceBuilder::new()
    }

    /// Calculates the color at a given elapsed time since the sequence started.
    ///
    /// Returns the interpolated color based on current position in the sequence,
    /// handling step progression, looping, and transitions.
    ///
    /// # Returns
    /// * `Some(color)` - The color to display at this time
    /// * `None` - Sequence has completed (only for finite loops)
    pub fn color_at(&self, elapsed: D) -> Option<Srgb> {
        let loop_millis = self.total_duration().as_millis();
        
        // Handle edge case: all steps have zero duration
        if loop_millis == 0 {
            if elapsed.as_millis() == 0 {
                return Some(self.steps[0].color);
            }
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
                return Some(
                    self.landing_color
                        .unwrap_or(self.steps.last().unwrap().color),
                );
            }
        }
        
        // Calculate time within current loop
        let time_in_loop = D::from_millis(elapsed_millis % loop_millis);

        // Find current step and calculate color
        let mut accumulated_time = D::ZERO;
        for (step_idx, step) in self.steps.iter().enumerate() {
            let step_end_time = D::from_millis(
                accumulated_time.as_millis() + step.duration.as_millis(),
            );

            if time_in_loop.as_millis() < step_end_time.as_millis() {
                let time_in_step = D::from_millis(
                    time_in_loop.as_millis() - accumulated_time.as_millis(),
                );
                return Some(self.calculate_step_color(step_idx, time_in_step));
            }

            accumulated_time = step_end_time;
        }

        // Fallback: return last color (should not reach here)
        Some(self.steps.last().unwrap().color)
    }

    /// Calculates the color for a specific step at a given time within that step.
    fn calculate_step_color(&self, step_idx: usize, time_in_step: D) -> Srgb {
        let step = &self.steps[step_idx];

        match step.transition {
            TransitionStyle::Step => step.color,
            TransitionStyle::Linear => {
                let previous_color = if step_idx == 0 {
                    self.steps.last().unwrap().color
                } else {
                    self.steps[step_idx - 1].color
                };

                let duration_millis = step.duration.as_millis();
                if duration_millis == 0 {
                    return step.color;
                }

                let time_millis = time_in_step.as_millis();
                let progress = (time_millis as f32) / (duration_millis as f32);
                let progress = progress.clamp(0.0, 1.0);

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
    pub fn landing_color(&self) -> Option<Srgb> {
        self.landing_color
    }

    /// Returns a reference to the step at the given index.
    pub fn get_step(&self, index: usize) -> Option<&SequenceStep<D>> {
        self.steps.get(index)
    }
}

/// Builder for constructing validated RGB sequences.
#[derive(Debug)]
pub struct SequenceBuilder<D: TimeDuration, const N: usize> {
    steps: Vec<SequenceStep<D>, N>,
    loop_count: LoopCount,
    landing_color: Option<Srgb>,
}

impl<D: TimeDuration, const N: usize> SequenceBuilder<D, N> {
    /// Creates a new empty sequence builder.
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            loop_count: LoopCount::default(),
            landing_color: None,
        }
    }

    /// Adds a step to the sequence.
    ///
    /// # Panics
    /// Panics if the sequence capacity is exceeded.
    pub fn step(
        mut self,
        color: Srgb,
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
    /// Default is `LoopCount::Finite(1)`.
    pub fn loop_count(mut self, count: LoopCount) -> Self {
        self.loop_count = count;
        self
    }

    /// Sets the color to display after the sequence completes.
    ///
    /// Only relevant for finite loop counts.
    pub fn landing_color(mut self, color: Srgb) -> Self {
        self.landing_color = Some(color);
        self
    }

    /// Builds and validates the sequence.
    ///
    /// # Errors
    /// * `EmptySequence` - No steps were added
    /// * `ZeroDurationWithLinear` - A step has zero duration with Linear transition
    pub fn build(self) -> Result<RgbSequence<D, N>, SequenceError> {
        if self.steps.is_empty() {
            return Err(SequenceError::EmptySequence);
        }

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
