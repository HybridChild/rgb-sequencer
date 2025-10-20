use crate::time::TimeDuration;
use crate::types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};
use heapless::Vec;
use palette::{Mix, Srgb};

/// Information about the current position within a sequence.
#[derive(Debug, Clone, Copy)]
pub struct StepPosition<D: TimeDuration> {
    /// Index of the current step
    pub step_index: usize,
    /// Time elapsed within the current step
    pub time_in_step: D,
    /// Time remaining until this step ends
    pub time_until_step_end: D,
    /// Whether the sequence has completed (finite sequences only)
    pub is_complete: bool,
}

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
    loop_duration: D,
}

impl<D: TimeDuration, const N: usize> RgbSequence<D, N> {
    /// Creates a new sequence builder.
    pub fn new() -> SequenceBuilder<D, N> {
        SequenceBuilder::new()
    }

    /// Checks if a finite sequence has completed at the given elapsed time.
    #[inline]
    fn check_completion(&self, elapsed: D) -> bool {
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

    /// Handles the special case where all steps have zero duration.
    #[inline]
    fn handle_zero_duration_sequence(&self, elapsed: D) -> StepPosition<D> {
        let is_complete = elapsed.as_millis() > 0;
        let step_index = if is_complete {
            self.steps.len() - 1
        } else {
            0
        };
        
        StepPosition {
            step_index,
            time_in_step: D::ZERO,
            time_until_step_end: D::ZERO,
            is_complete,
        }
    }

    /// Creates a position for a completed finite sequence.
    #[inline]
    fn create_complete_position(&self) -> StepPosition<D> {
        let last_index = self.steps.len() - 1;
        StepPosition {
            step_index: last_index,
            time_in_step: self.steps[last_index].duration,
            time_until_step_end: D::ZERO,
            is_complete: true,
        }
    }

    /// Finds which step contains the given time within a loop.
    fn find_step_at_time(&self, time_in_loop: D) -> StepPosition<D> {
        let mut accumulated_time = D::ZERO;
        
        for (step_idx, step) in self.steps.iter().enumerate() {
            let step_end_time = D::from_millis(
                accumulated_time.as_millis() + step.duration.as_millis(),
            );

            if time_in_loop.as_millis() < step_end_time.as_millis() {
                let time_in_step = D::from_millis(
                    time_in_loop.as_millis() - accumulated_time.as_millis(),
                );
                let time_until_end = step_end_time.saturating_sub(time_in_loop);
                
                return StepPosition {
                    step_index: step_idx,
                    time_in_step,
                    time_until_step_end: time_until_end,
                    is_complete: false,
                };
            }

            accumulated_time = step_end_time;
        }

        // Fallback to last step
        let last_index = self.steps.len() - 1;
        StepPosition {
            step_index: last_index,
            time_in_step: self.steps[last_index].duration,
            time_until_step_end: D::ZERO,
            is_complete: false,
        }
    }

    /// Performs linear color interpolation for a step.
    #[inline]
    fn interpolate_color(&self, position: &StepPosition<D>, step: &SequenceStep<D>) -> Srgb {
        let previous_color = if position.step_index == 0 {
            self.steps.last().unwrap().color
        } else {
            self.steps[position.step_index - 1].color
        };

        let duration_millis = step.duration.as_millis();
        if duration_millis == 0 {
            return step.color;
        }

        let time_millis = position.time_in_step.as_millis();
        let progress = (time_millis as f32) / (duration_millis as f32);
        let progress = progress.clamp(0.0, 1.0);

        previous_color.mix(step.color, progress)
    }

    /// Finds the current step position information at the given elapsed time.
    pub fn find_step_position(&self, elapsed: D) -> Option<StepPosition<D>> {
        if self.steps.is_empty() {
            return None;
        }

        let loop_millis = self.loop_duration.as_millis();
        
        // Handle all-zero-duration case
        if loop_millis == 0 {
            return Some(self.handle_zero_duration_sequence(elapsed));
        }

        
        // Check if finite sequence is complete
        if self.check_completion(elapsed) {
            return Some(self.create_complete_position());
        }
        
        // Calculate time within current loop
        let time_in_loop = D::from_millis(elapsed.as_millis() % loop_millis);

        // Find current step within the loop
        Some(self.find_step_at_time(time_in_loop))
    }

    /// Calculates the color at a given step position.
    #[inline]
    pub fn color_at_position(&self, position: &StepPosition<D>) -> Srgb {
        if position.is_complete {
            return self.landing_color
                .unwrap_or(self.steps.last().unwrap().color);
        }

        let step = &self.steps[position.step_index];

        match step.transition {
            TransitionStyle::Step => step.color,
            TransitionStyle::Linear => {
                self.interpolate_color(position, step)
            }
        }
    }

    /// Calculates the color at a given elapsed time since the sequence started.
    ///
    /// Returns the interpolated color based on current position in the sequence,
    /// handling step progression, looping, and transitions. For finite sequences
    /// that have completed, returns the landing color (or last step color if no
    /// landing color was specified).
    pub fn color_at(&self, elapsed: D) -> Srgb {
        if let Some(position) = self.find_step_position(elapsed) {
            self.color_at_position(&position)
        } else {
            // Fallback for empty sequence (shouldn't happen after validation)
            Srgb::new(0.0, 0.0, 0.0)
        }
    }

    /// Calculates the optimal time until the next service call is needed.
    ///
    /// Returns:
    /// * `Some(Duration::ZERO)` - Linear transition in progress, service at frame rate
    /// * `Some(duration)` - Step transition, service after this duration
    /// * `None` - Sequence complete, no further servicing needed
    pub fn next_service_time(&self, position: &StepPosition<D>) -> Option<D> {
        if position.is_complete {
            return None;
        }

        let step = &self.steps[position.step_index];
        match step.transition {
            TransitionStyle::Linear => Some(D::ZERO),
            TransitionStyle::Step => Some(position.time_until_step_end),
        }
    }

    /// Returns true if a finite sequence has completed at the given elapsed time.
    #[inline]
    pub fn is_complete(&self, elapsed: D) -> bool {
        self.check_completion(elapsed)
    }

    /// Returns the duration of one complete loop through all steps.
    pub fn loop_duration(&self) -> D {
        self.loop_duration
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

        // Calculate and cache loop duration here to avoid repeated calculation during operation
        let total_millis: u64 = self.steps.iter().map(|s| s.duration.as_millis()).sum();
        let loop_duration = D::from_millis(total_millis);

        Ok(RgbSequence {
            steps: self.steps,
            loop_count: self.loop_count,
            landing_color: self.landing_color,
            loop_duration,
        })
    }
}

impl<D: TimeDuration, const N: usize> Default for SequenceBuilder<D, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock Duration type for testing
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    // Helper colors
    const RED: Srgb = Srgb::new(1.0, 0.0, 0.0);
    const GREEN: Srgb = Srgb::new(0.0, 1.0, 0.0);
    const BLUE: Srgb = Srgb::new(0.0, 0.0, 1.0);
    const BLACK: Srgb = Srgb::new(0.0, 0.0, 0.0);

    // Helper to compare colors with small tolerance for floating point
    fn colors_equal(a: Srgb, b: Srgb) -> bool {
        const EPSILON: f32 = 0.001;
        (a.red - b.red).abs() < EPSILON
            && (a.green - b.green).abs() < EPSILON
            && (a.blue - b.blue).abs() < EPSILON
    }

    #[test]
    fn builder_rejects_empty_sequence() {
        let result = RgbSequence::<TestDuration, 8>::new().build();
        assert!(matches!(result, Err(SequenceError::EmptySequence)));
    }

    #[test]
    fn builder_rejects_zero_duration_with_linear() {
        let result = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(0), TransitionStyle::Linear)
            .build();
        assert!(matches!(result, Err(SequenceError::ZeroDurationWithLinear)));
    }

    #[test]
    fn builder_accepts_valid_sequence() {
        let result = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(200), TransitionStyle::Linear)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn loop_duration_is_cached_correctly() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(200), TransitionStyle::Step)
            .step(BLUE, TestDuration(50), TransitionStyle::Step)
            .build()
            .unwrap();

        assert_eq!(sequence.loop_duration(), TestDuration(350));
    }

    #[test]
    fn step_transition_holds_color() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .build()
            .unwrap();

        // At start
        assert!(colors_equal(sequence.color_at(TestDuration(0)), RED));
        
        // At middle
        assert!(colors_equal(sequence.color_at(TestDuration(500)), RED));
        
        // At end
        assert!(colors_equal(sequence.color_at(TestDuration(999)), RED));
    }

    #[test]
    fn linear_transition_interpolates_correctly() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(BLUE, TestDuration(1000), TransitionStyle::Linear)
            .build()
            .unwrap();

        // At start of linear step (just after red step ends)
        let color_at_start = sequence.color_at(TestDuration(100));
        assert!(colors_equal(color_at_start, RED));

        // At 50% through linear transition
        let color_at_middle = sequence.color_at(TestDuration(600));
        let expected_middle = RED.mix(BLUE, 0.5);
        assert!(colors_equal(color_at_middle, expected_middle));

        // At end of linear step
        let color_at_end = sequence.color_at(TestDuration(1099));
        assert!(colors_equal(color_at_end, BLUE));
    }

    #[test]
    fn first_step_linear_interpolates_from_last() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(1000), TransitionStyle::Linear)
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .step(BLUE, TestDuration(1000), TransitionStyle::Step)
            .loop_count(LoopCount::Infinite)
            .build()
            .unwrap();

        let expected_middle = BLUE.mix(RED, 0.5);

        // First loop's first step
        assert!(colors_equal(sequence.color_at(TestDuration(0)), BLUE));
        let color_at_middle = sequence.color_at(TestDuration(500));
        assert!(colors_equal(color_at_middle, expected_middle));
        assert!(colors_equal(sequence.color_at(TestDuration(999)), RED));
        
        // Second loop's first step
        assert!(colors_equal(sequence.color_at(TestDuration(3000)), BLUE));
        let color_at_middle = sequence.color_at(TestDuration(3500));
        assert!(colors_equal(color_at_middle, expected_middle));
        assert!(colors_equal(sequence.color_at(TestDuration(3999)), RED));
    }

    #[test]
    fn multi_step_sequence_progresses() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .build()
            .unwrap();

        assert!(colors_equal(sequence.color_at(TestDuration(50)), RED));
        assert!(colors_equal(sequence.color_at(TestDuration(150)), GREEN));
        assert!(colors_equal(sequence.color_at(TestDuration(250)), BLUE));
    }

    #[test]
    fn finite_loop_completes_and_shows_landing_color() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .loop_count(LoopCount::Finite(2))
            .landing_color(BLACK)
            .build()
            .unwrap();
        
        // During first loop
        assert!(colors_equal(sequence.color_at(TestDuration(50)), RED));
        
        // During second loop
        assert!(colors_equal(sequence.color_at(TestDuration(350)), GREEN));
        
        // After completion - should show landing color
        assert!(colors_equal(sequence.color_at(TestDuration(400)), BLACK));
        assert!(colors_equal(sequence.color_at(TestDuration(1000)), BLACK));
    }

    #[test]
    fn finite_loop_uses_last_step_color_when_no_landing_color() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .loop_count(LoopCount::Finite(2))
            // Note: no .landing_color() call
            .build()
            .unwrap();

        // During loops - normal behavior
        assert!(colors_equal(sequence.color_at(TestDuration(50)), RED));
        assert!(colors_equal(sequence.color_at(TestDuration(450)), GREEN));
        
        // After completion - should show BLUE (the last step's color)
        assert!(colors_equal(sequence.color_at(TestDuration(600)), BLUE));
        assert!(colors_equal(sequence.color_at(TestDuration(1000)), BLUE));
    }
    
    #[test]
    fn infinite_loop_never_completes() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .loop_count(LoopCount::Infinite)
            .landing_color(BLACK)
            .build()
            .unwrap();

        // First loop
        assert!(colors_equal(sequence.color_at(TestDuration(50)), RED));
        
        // Second loop
        assert!(colors_equal(sequence.color_at(TestDuration(350)), GREEN));
        
        // Many loops later - still cycling
        assert!(colors_equal(sequence.color_at(TestDuration(10050)), RED));
    }

    #[test]
    fn all_zero_duration_steps() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(0), TransitionStyle::Step)
            .step(GREEN, TestDuration(0), TransitionStyle::Step)
            .loop_count(LoopCount::Finite(1))
            .landing_color(BLUE)
            .build()
            .unwrap();

        // At time zero, should show first color
        assert!(colors_equal(sequence.color_at(TestDuration(0)), RED));
        
        // Any time after zero, should show landing color
        assert!(colors_equal(sequence.color_at(TestDuration(1)), BLUE));
        assert!(colors_equal(sequence.color_at(TestDuration(100)), BLUE));
    }

    #[test]
    fn find_step_position_returns_correct_info() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(200), TransitionStyle::Linear)
            .step(BLUE, TestDuration(50), TransitionStyle::Step)
            .build()
            .unwrap();

        // Middle of first step
        let pos = sequence.find_step_position(TestDuration(50)).unwrap();
        assert_eq!(pos.step_index, 0);
        assert_eq!(pos.time_in_step, TestDuration(50));
        assert_eq!(pos.time_until_step_end, TestDuration(50));
        assert!(!pos.is_complete);

        // Middle of second step
        let pos = sequence.find_step_position(TestDuration(200)).unwrap();
        assert_eq!(pos.step_index, 1);
        assert_eq!(pos.time_in_step, TestDuration(100));
        assert_eq!(pos.time_until_step_end, TestDuration(100));
        assert!(!pos.is_complete);

        // In third step
        let pos = sequence.find_step_position(TestDuration(320)).unwrap();
        assert_eq!(pos.step_index, 2);
        assert_eq!(pos.time_in_step, TestDuration(20));
        assert_eq!(pos.time_until_step_end, TestDuration(30));
        assert!(!pos.is_complete);

        // After sequence completes
        let pos = sequence.find_step_position(TestDuration(400)).unwrap();
        assert_eq!(pos.step_index, 2);  // Last step index
        assert_eq!(pos.time_in_step, TestDuration(50));  // Duration of last step
        assert_eq!(pos.time_until_step_end, TestDuration(0));
        assert!(pos.is_complete);
    }
}
