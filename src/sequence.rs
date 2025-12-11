//! RGB color sequence definitions and evaluation.

use crate::BLACK;
use crate::time::TimeDuration;
use crate::types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};
use heapless::Vec;
use palette::{Mix, Srgb};

/// Applies easing curve to linear progress value (0.0 to 1.0).
#[inline]
fn apply_easing(t: f32, transition: TransitionStyle) -> f32 {
    match transition {
        TransitionStyle::Step => t,
        TransitionStyle::Linear => t,
        TransitionStyle::EaseIn => t * t, // Quadratic ease-in
        TransitionStyle::EaseOut => t * (2.0 - t), // Quadratic ease-out
        TransitionStyle::EaseInOut => {
            // Quadratic ease-in-out
            if t < 0.5 {
                2.0 * t * t
            } else {
                -1.0 + (4.0 - 2.0 * t) * t
            }
        }
    }
}

/// Position within a sequence.
#[derive(Debug, Clone, Copy)]
pub struct StepPosition<D: TimeDuration> {
    /// Current step index.
    pub step_index: usize,
    /// Elapsed time in current step.
    pub time_in_step: D,
    /// Time remaining until step ends.
    pub time_until_step_end: D,
    /// Whether sequence is complete.
    pub is_complete: bool,
    /// Current loop iteration.
    pub current_loop: u32,
}

/// An RGB color sequence.
#[derive(Debug, Clone)]
pub struct RgbSequence<D: TimeDuration, const N: usize> {
    steps: Vec<SequenceStep<D>, N>,
    loop_count: LoopCount,
    start_color: Option<Srgb>,
    landing_color: Option<Srgb>,
    loop_duration: D,

    color_fn: Option<fn(Srgb, D) -> Srgb>,
    timing_fn: Option<fn(D) -> Option<D>>,
}

impl<D: TimeDuration, const N: usize> RgbSequence<D, N> {
    /// Creates a new sequence builder for step-based sequences.
    pub fn builder() -> SequenceBuilder<D, N> {
        SequenceBuilder::new()
    }

    /// Creates a function-based sequence for algorithmic animations.
    ///
    /// The `color_fn` receives base color and elapsed time, returning the current color.
    /// The `timing_fn` returns next service delay (`Some(D::ZERO)` for continuous updates,
    /// `Some(delay)` to wait, `None` when complete).
    pub fn from_function(
        base_color: Srgb,
        color_fn: fn(Srgb, D) -> Srgb,
        timing_fn: fn(D) -> Option<D>,
    ) -> Self {
        Self {
            steps: Vec::new(),
            loop_count: LoopCount::Finite(1),
            landing_color: None,
            loop_duration: D::ZERO,
            start_color: Some(base_color),
            color_fn: Some(color_fn),
            timing_fn: Some(timing_fn),
        }
    }

    /// Creates a simple solid color sequence.
    ///
    /// Returns `SequenceError::CapacityExceeded` if `N < 1`.
    pub fn solid(color: Srgb, duration: D) -> Result<Self, SequenceError> {
        Self::builder()
            .step(color, duration, TransitionStyle::Step)?
            .build()
    }

    /// Evaluates color and next service time at elapsed time.
    ///
    /// Returns `(color, timing)` where timing is `Some(D::ZERO)` for continuous animation,
    /// `Some(delay)` for static hold, or `None` when sequence completes.
    #[inline]
    pub fn evaluate(&self, elapsed: D) -> (Srgb, Option<D>) {
        // Use custom functions if present
        if let (Some(color_fn), Some(timing_fn)) = (self.color_fn, self.timing_fn) {
            let base = self.start_color.unwrap_or(BLACK);
            return (color_fn(base, elapsed), timing_fn(elapsed));
        }

        // Step-based evaluation - calculate position once
        if let Some(position) = self.find_step_position(elapsed) {
            let color = self.color_at_position(&position);
            let timing = self.next_service_time_from_position(&position);
            (color, timing)
        } else {
            // Empty sequence fallback (shouldn't happen after validation)
            (BLACK, None)
        }
    }

    /// Returns true if step-based finite sequence has completed all loops.
    #[inline]
    fn is_complete_step_based(&self, elapsed: D) -> bool {
        match self.loop_count {
            LoopCount::Finite(count) => {
                let loop_millis = self.loop_duration.as_millis();
                if loop_millis == 0 {
                    elapsed.as_millis() > 0
                } else {
                    let total_duration = loop_millis * (count as u64);
                    elapsed.as_millis() >= total_duration
                }
            }
            LoopCount::Infinite => false,
        }
    }

    /// Creates position for zero-duration sequences.
    #[inline]
    fn handle_zero_duration_sequence(&self, elapsed: D) -> StepPosition<D> {
        let is_complete = elapsed.as_millis() > 0;
        let step_index = if is_complete { self.steps.len() - 1 } else { 0 };

        StepPosition {
            step_index,
            time_in_step: D::ZERO,
            time_until_step_end: D::ZERO,
            is_complete,
            current_loop: 0,
        }
    }

    /// Creates position representing sequence completion.
    #[inline]
    fn create_complete_position(&self) -> StepPosition<D> {
        let last_index = self.steps.len() - 1;
        let loop_count = match self.loop_count {
            LoopCount::Finite(count) => count,
            LoopCount::Infinite => 0,
        };

        StepPosition {
            step_index: last_index,
            time_in_step: self.steps[last_index].duration,
            time_until_step_end: D::ZERO,
            is_complete: true,
            current_loop: loop_count.saturating_sub(1),
        }
    }

    /// Finds the step position at a specific time within a loop.
    #[inline]
    fn find_step_at_time(&self, time_in_loop: D, current_loop: u32) -> StepPosition<D> {
        let mut accumulated_time = D::ZERO;

        for (step_idx, step) in self.steps.iter().enumerate() {
            let step_end_time =
                D::from_millis(accumulated_time.as_millis() + step.duration.as_millis());

            if time_in_loop.as_millis() < step_end_time.as_millis() {
                let time_in_step =
                    D::from_millis(time_in_loop.as_millis() - accumulated_time.as_millis());
                let time_until_end = step_end_time.saturating_sub(time_in_loop);

                return StepPosition {
                    step_index: step_idx,
                    time_in_step,
                    time_until_step_end: time_until_end,
                    is_complete: false,
                    current_loop,
                };
            }

            accumulated_time = step_end_time;
        }

        let last_index = self.steps.len() - 1;
        StepPosition {
            step_index: last_index,
            time_in_step: self.steps[last_index].duration,
            time_until_step_end: D::ZERO,
            is_complete: false,
            current_loop,
        }
    }

    /// Interpolates color at current position with easing applied.
    #[inline]
    fn interpolate_color(&self, position: &StepPosition<D>, step: &SequenceStep<D>) -> Srgb {
        // Determine if this transition should use start_color for first step of first loop
        let use_start_color = position.step_index == 0
            && position.current_loop == 0
            && self.start_color.is_some()
            && matches!(
                step.transition,
                TransitionStyle::Linear
                    | TransitionStyle::EaseIn
                    | TransitionStyle::EaseOut
                    | TransitionStyle::EaseInOut
            );

        let previous_color = if use_start_color {
            self.start_color.unwrap()
        } else if position.step_index == 0 {
            self.steps.last().unwrap().color
        } else {
            self.steps[position.step_index - 1].color
        };

        let duration_millis = step.duration.as_millis();
        if duration_millis == 0 {
            return step.color;
        }

        let time_millis = position.time_in_step.as_millis();
        let mut progress = (time_millis as f32) / (duration_millis as f32);
        progress = progress.clamp(0.0, 1.0);

        // Apply easing function
        progress = apply_easing(progress, step.transition);

        previous_color.mix(step.color, progress)
    }

    /// Returns the current position within the sequence at the given elapsed time.
    ///
    /// Includes step index, loop number, and timing information within the current step.
    /// Returns `None` if the sequence is empty or function-based.
    pub fn find_step_position(&self, elapsed: D) -> Option<StepPosition<D>> {
        if self.steps.is_empty() {
            return None;
        }

        let loop_millis = self.loop_duration.as_millis();

        if loop_millis == 0 {
            return Some(self.handle_zero_duration_sequence(elapsed));
        }

        if self.is_complete_step_based(elapsed) {
            return Some(self.create_complete_position());
        }

        let elapsed_millis = elapsed.as_millis();
        let current_loop = (elapsed_millis / loop_millis) as u32;
        let time_in_loop = D::from_millis(elapsed_millis % loop_millis);

        Some(self.find_step_at_time(time_in_loop, current_loop))
    }

    /// Returns the color at the given position.
    #[inline]
    fn color_at_position(&self, position: &StepPosition<D>) -> Srgb {
        if position.is_complete {
            return self
                .landing_color
                .unwrap_or(self.steps.last().unwrap().color);
        }

        let step = &self.steps[position.step_index];

        match step.transition {
            TransitionStyle::Step => step.color,
            TransitionStyle::Linear
            | TransitionStyle::EaseIn
            | TransitionStyle::EaseOut
            | TransitionStyle::EaseInOut => self.interpolate_color(position, step),
        }
    }

    /// Returns next service delay based on position and transition type.
    #[inline]
    fn next_service_time_from_position(&self, position: &StepPosition<D>) -> Option<D> {
        if position.is_complete {
            return None;
        }

        let step = &self.steps[position.step_index];
        match step.transition {
            // Interpolating transitions need continuous updates
            TransitionStyle::Linear
            | TransitionStyle::EaseIn
            | TransitionStyle::EaseOut
            | TransitionStyle::EaseInOut => Some(D::ZERO),
            // Step transition can wait until the end
            TransitionStyle::Step => Some(position.time_until_step_end),
        }
    }

    /// Returns true if sequence has completed.
    #[inline]
    pub fn has_completed(&self, elapsed: D) -> bool {
        if let Some(timing_fn) = self.timing_fn {
            timing_fn(elapsed).is_none()
        } else {
            self.is_complete_step_based(elapsed)
        }
    }

    /// Returns loop duration.
    #[inline]
    pub fn loop_duration(&self) -> D {
        self.loop_duration
    }

    /// Returns step count.
    #[inline]
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Returns loop count.
    #[inline]
    pub fn loop_count(&self) -> LoopCount {
        self.loop_count
    }

    /// Returns landing color.
    #[inline]
    pub fn landing_color(&self) -> Option<Srgb> {
        self.landing_color
    }

    /// Returns start color.
    #[inline]
    pub fn start_color(&self) -> Option<Srgb> {
        self.start_color
    }

    /// Returns step at index.
    #[inline]
    pub fn get_step(&self, index: usize) -> Option<&SequenceStep<D>> {
        self.steps.get(index)
    }

    /// Returns true if function-based.
    #[inline]
    pub fn is_function_based(&self) -> bool {
        self.color_fn.is_some()
    }
}

/// Builder for RGB sequences.
#[derive(Debug)]
pub struct SequenceBuilder<D: TimeDuration, const N: usize> {
    steps: Vec<SequenceStep<D>, N>,
    loop_count: LoopCount,
    landing_color: Option<Srgb>,
    start_color: Option<Srgb>,
}

impl<D: TimeDuration, const N: usize> SequenceBuilder<D, N> {
    /// Creates a new sequence builder.
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            loop_count: LoopCount::default(),
            landing_color: None,
            start_color: None,
        }
    }

    /// Adds a step to the sequence.
    ///
    /// Panics if capacity `N` is exceeded.
    pub fn step(
        mut self,
        color: Srgb,
        duration: D,
        transition: TransitionStyle,
    ) -> Result<Self, SequenceError> {
        self.steps
            .push(SequenceStep::new(color, duration, transition))
            .map_err(|_| SequenceError::CapacityExceeded)?;
        Ok(self)
    }

    /// Sets loop count (default: `Finite(1)`).
    pub fn loop_count(mut self, count: LoopCount) -> Self {
        self.loop_count = count;
        self
    }

    /// Sets landing color shown after sequence completes (finite sequences only).
    pub fn landing_color(mut self, color: Srgb) -> Self {
        self.landing_color = Some(color);
        self
    }

    /// Sets start color for smooth entry into first step (first loop only, Linear transitions only).
    pub fn start_color(mut self, color: Srgb) -> Self {
        self.start_color = Some(color);
        self
    }

    /// Builds and validates sequence.
    ///
    /// Returns error if sequence is empty or has zero-duration steps with interpolating transitions.
    pub fn build(self) -> Result<RgbSequence<D, N>, SequenceError> {
        if self.steps.is_empty() {
            return Err(SequenceError::EmptySequence);
        }

        for step in &self.steps {
            if step.duration.as_millis() == 0
                && matches!(
                    step.transition,
                    TransitionStyle::Linear
                        | TransitionStyle::EaseIn
                        | TransitionStyle::EaseOut
                        | TransitionStyle::EaseInOut
                )
            {
                return Err(SequenceError::ZeroDurationWithLinear);
            }
        }

        // Calculate and cache loop duration here to avoid repeated calculation during operation
        let total_millis: u64 = self.steps.iter().map(|s| s.duration.as_millis()).sum();
        let loop_duration = D::from_millis(total_millis);

        Ok(RgbSequence {
            steps: self.steps,
            loop_count: self.loop_count,
            landing_color: self.landing_color,
            loop_duration,
            start_color: self.start_color,
            color_fn: None,
            timing_fn: None,
        })
    }
}

impl<D: TimeDuration, const N: usize> Default for SequenceBuilder<D, N> {
    /// Returns a new default sequence builder.
    fn default() -> Self {
        Self::new()
    }
}
