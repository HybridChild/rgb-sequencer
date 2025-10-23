use crate::time::TimeDuration;
use crate::types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};
use crate::COLOR_OFF;
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
    /// Which loop iteration we're currently in (0-based)
    pub current_loop: u32,
}

/// An RGB color sequence with precise timing and transitions.
///
/// Defines a complete animation sequence consisting of multiple steps, each with
/// a target color, duration, and transition style. Sequences can loop a finite
/// number of times or indefinitely, and optionally specify a landing color to
/// display after completion.
///
/// Alternatively, sequences can use custom function-based animation by providing
/// a start color, color computation function, and timing hint function. This allows
/// for algorithmic animations (sine waves, breathing effects, etc.) that can be
/// reused with different colors while maintaining the same type signature for
/// collections.
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
    start_color: Option<Srgb>,
    landing_color: Option<Srgb>,
    loop_duration: D,
    
    // Optional function-based animation (overrides step-based logic)
    color_fn: Option<fn(Srgb, D) -> Srgb>,
    timing_fn: Option<fn(D) -> Option<D>>,
}

impl<D: TimeDuration, const N: usize> RgbSequence<D, N> {
    /// Creates a new sequence builder for step-based sequences.
    pub fn new() -> SequenceBuilder<D, N> {
        SequenceBuilder::new()
    }

    /// Creates a sequence using custom functions instead of steps.
    ///
    /// This is an advanced feature for algorithmic animations. The color function
    /// receives a base color and elapsed time, allowing the same function to be
    /// reused with different colors.
    ///
    /// # Arguments
    /// * `start_color` - The base color passed to the color function
    /// * `color_fn` - Function that computes color based on start color and elapsed time
    /// * `timing_fn` - Function that returns when next service is needed
    ///   - `Some(Duration::ZERO)` = continuous updates (every frame)
    ///   - `Some(duration)` = wait this long before next update
    ///   - `None` = animation complete
    pub fn from_function(
        start_color: Srgb,
        color_fn: fn(Srgb, D) -> Srgb,
        timing_fn: fn(D) -> Option<D>,
    ) -> Self {
        Self {
            steps: Vec::new(),
            loop_count: LoopCount::Finite(1),
            landing_color: None,
            loop_duration: D::ZERO,
            start_color: Some(start_color),
            color_fn: Some(color_fn),
            timing_fn: Some(timing_fn),
        }
    }

    /// Evaluates the sequence at the given elapsed time.
    ///
    /// This is the primary method for computing both the current color and when
    /// the next service call should occur. It efficiently calculates both values
    /// in a single pass, avoiding duplicate position lookups for step-based sequences.
    ///
    /// # Returns
    /// A tuple of:
    /// * `Srgb` - The color to display at this point in time
    /// * `Option<Duration>` - When to service next:
    ///   - `Some(Duration::ZERO)` = Service at frame rate (linear transitions)
    ///   - `Some(duration)` = Service after this duration (step transitions)
    ///   - `None` = Animation complete, no further servicing needed
    pub fn evaluate(&self, elapsed: D) -> (Srgb, Option<D>) {
        // Use custom functions if present
        if let (Some(color_fn), Some(timing_fn)) = (self.color_fn, self.timing_fn) {
            let base = self.start_color.unwrap_or(COLOR_OFF);
            return (color_fn(base, elapsed), timing_fn(elapsed));
        }

        // Step-based evaluation - calculate position once
        if let Some(position) = self.find_step_position(elapsed) {
            let color = self.color_at_position(&position);
            let timing = self.next_service_time_from_position(&position);
            (color, timing)
        } else {
            // Empty sequence fallback (shouldn't happen after validation)
            (COLOR_OFF, None)
        }
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
            current_loop: 0,
        }
    }

    /// Creates a position for a completed finite sequence.
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

    /// Finds which step contains the given time within a loop.
    fn find_step_at_time(&self, time_in_loop: D, current_loop: u32) -> StepPosition<D> {
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
                    current_loop,
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
            current_loop,
        }
    }

    /// Performs linear color interpolation for a step.
    #[inline]
    fn interpolate_color(&self, position: &StepPosition<D>, step: &SequenceStep<D>) -> Srgb {
        // If this is the first step with Linear transition and we're on the first loop,
        // use start_color if available
        let previous_color = if position.step_index == 0 
            && step.transition == TransitionStyle::Linear 
            && position.current_loop == 0 
            && self.start_color.is_some() 
        {
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
        let progress = (time_millis as f32) / (duration_millis as f32);
        let progress = progress.clamp(0.0, 1.0);

        previous_color.mix(step.color, progress)
    }

    /// Finds the current step position information at the given elapsed time.
    ///
    /// Used internally for step-based sequences. Returns None if the sequence
    /// has no steps.
    fn find_step_position(&self, elapsed: D) -> Option<StepPosition<D>> {
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
        
        // Calculate which loop we're in and time within current loop
        let elapsed_millis = elapsed.as_millis();
        let current_loop = (elapsed_millis / loop_millis) as u32;
        let time_in_loop = D::from_millis(elapsed_millis % loop_millis);

        // Find current step within the loop
        Some(self.find_step_at_time(time_in_loop, current_loop))
    }

    /// Calculates the color at a given step position.
    ///
    /// Used internally for step-based sequences.
    #[inline]
    fn color_at_position(&self, position: &StepPosition<D>) -> Srgb {
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

    /// Calculates optimal next service time from a step position.
    ///
    /// Used internally for step-based sequences.
    #[inline]
    fn next_service_time_from_position(&self, position: &StepPosition<D>) -> Option<D> {
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
    ///
    /// For function-based sequences, this checks if the timing function returns None.
    /// For step-based sequences, this checks if all loops have completed.
    #[inline]
    pub fn is_complete(&self, elapsed: D) -> bool {
        if let Some(timing_fn) = self.timing_fn {
            // For function-based sequences, check if timing function returns None
            timing_fn(elapsed).is_none()
        } else {
            // For step-based sequences, check loop completion
            self.check_completion(elapsed)
        }
    }

    /// Returns the duration of one complete loop through all steps.
    ///
    /// Returns `Duration::ZERO` for function-based sequences.
    pub fn loop_duration(&self) -> D {
        self.loop_duration
    }

    /// Returns the number of steps in this sequence.
    ///
    /// Returns 0 for function-based sequences.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Returns the loop count configuration.
    pub fn loop_count(&self) -> LoopCount {
        self.loop_count
    }

    /// Returns the landing color if one is configured.
    ///
    /// For step-based sequences, this is the color shown after completion.
    /// For function-based sequences, this is not used.
    pub fn landing_color(&self) -> Option<Srgb> {
        self.landing_color
    }

    /// Returns the start color if one is configured.
    ///
    /// For function-based sequences, this is the base color passed to the function.
    /// For step-based sequences, this is used as the interpolation starting point
    /// for the first step if it has a Linear transition (first loop only).
    pub fn start_color(&self) -> Option<Srgb> {
        self.start_color
    }

    /// Returns a reference to the step at the given index.
    ///
    /// Returns None for function-based sequences or if index is out of bounds.
    pub fn get_step(&self, index: usize) -> Option<&SequenceStep<D>> {
        self.steps.get(index)
    }

    /// Returns true if this sequence uses custom functions instead of steps.
    pub fn is_function_based(&self) -> bool {
        self.color_fn.is_some()
    }
}

/// Builder for constructing validated RGB sequences.
#[derive(Debug)]
pub struct SequenceBuilder<D: TimeDuration, const N: usize> {
    steps: Vec<SequenceStep<D>, N>,
    loop_count: LoopCount,
    landing_color: Option<Srgb>,
    start_color: Option<Srgb>,
}

impl<D: TimeDuration, const N: usize> SequenceBuilder<D, N> {
    /// Creates a new empty sequence builder.
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

    /// Sets the starting color for the sequence.
    ///
    /// For step-based sequences with a Linear transition on the first step,
    /// this color will be used as the interpolation starting point during
    /// the first loop only. On subsequent loops, the sequence will interpolate
    /// from the last step's color as normal.
    ///
    /// This is useful for creating smooth entry animations into a looping sequence.
    pub fn start_color(mut self, color: Srgb) -> Self {
        self.start_color = Some(color);
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
            start_color: self.start_color,
            color_fn: None,
            timing_fn: None,
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
    const YELLOW: Srgb = Srgb::new(1.0, 1.0, 0.0);

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
    fn function_based_sequence_with_base_color() {
        // Brightness modulation function - works with any base color
        fn brightness_pulse(base: Srgb, elapsed: TestDuration) -> Srgb {
            let brightness = if elapsed.as_millis() < 500 {
                0.5
            } else {
                1.0
            };
            Srgb::new(
                base.red * brightness,
                base.green * brightness,
                base.blue * brightness,
            )
        }

        fn test_timing(elapsed: TestDuration) -> Option<TestDuration> {
            if elapsed.as_millis() < 1000 {
                Some(TestDuration::ZERO)
            } else {
                None
            }
        }

        // Same function, different colors
        let red_pulse = RgbSequence::<TestDuration, 8>::from_function(
            RED,
            brightness_pulse,
            test_timing,
        );

        let blue_pulse = RgbSequence::<TestDuration, 8>::from_function(
            BLUE,
            brightness_pulse,
            test_timing,
        );

        assert!(red_pulse.is_function_based());
        assert_eq!(red_pulse.start_color(), Some(RED));
        assert_eq!(blue_pulse.start_color(), Some(BLUE));

        // Red pulse at 50% brightness
        let (color, _) = red_pulse.evaluate(TestDuration(100));
        assert!(colors_equal(color, Srgb::new(0.5, 0.0, 0.0)));

        // Red pulse at 100% brightness
        let (color, _) = red_pulse.evaluate(TestDuration(600));
        assert!(colors_equal(color, RED));

        // Blue pulse at 50% brightness
        let (color, _) = blue_pulse.evaluate(TestDuration(100));
        assert!(colors_equal(color, Srgb::new(0.0, 0.0, 0.5)));

        // Blue pulse at 100% brightness
        let (color, _) = blue_pulse.evaluate(TestDuration(600));
        assert!(colors_equal(color, BLUE));
    }

    #[test]
    fn function_based_sequence_timing_works() {
        fn test_color(base: Srgb, _elapsed: TestDuration) -> Srgb {
            base
        }

        fn test_timing(elapsed: TestDuration) -> Option<TestDuration> {
            if elapsed.as_millis() < 1000 {
                Some(TestDuration::ZERO)
            } else {
                None
            }
        }

        let seq = RgbSequence::<TestDuration, 8>::from_function(RED, test_color, test_timing);

        let (_, timing) = seq.evaluate(TestDuration(100));
        assert_eq!(timing, Some(TestDuration::ZERO));

        let (_, timing) = seq.evaluate(TestDuration(1000));
        assert_eq!(timing, None);
        assert!(seq.is_complete(TestDuration(1000)));
    }

    #[test]
    fn evaluate_returns_both_color_and_timing() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(200), TransitionStyle::Linear)
            .build()
            .unwrap();

        // At start - RED with 100ms until next step
        let (color, timing) = sequence.evaluate(TestDuration(0));
        assert!(colors_equal(color, RED));
        assert_eq!(timing, Some(TestDuration(100)));

        // During linear transition - should return ZERO for continuous updates
        let (color, timing) = sequence.evaluate(TestDuration(200));
        assert!(colors_equal(color, RED.mix(GREEN, 0.5)));
        assert_eq!(timing, Some(TestDuration::ZERO));
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

        let (color, _) = sequence.evaluate(TestDuration(0));
        assert!(colors_equal(color, RED));
        
        let (color, _) = sequence.evaluate(TestDuration(500));
        assert!(colors_equal(color, RED));
        
        let (color, _) = sequence.evaluate(TestDuration(999));
        assert!(colors_equal(color, RED));
    }

    #[test]
    fn linear_transition_interpolates_correctly() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(BLUE, TestDuration(1000), TransitionStyle::Linear)
            .build()
            .unwrap();

        let (color, _) = sequence.evaluate(TestDuration(100));
        assert!(colors_equal(color, RED));

        let (color, _) = sequence.evaluate(TestDuration(600));
        let expected_middle = RED.mix(BLUE, 0.5);
        assert!(colors_equal(color, expected_middle));

        let (color, _) = sequence.evaluate(TestDuration(1099));
        assert!(colors_equal(color, BLUE));
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

        let (color, _) = sequence.evaluate(TestDuration(0));
        assert!(colors_equal(color, BLUE));
        
        let (color, _) = sequence.evaluate(TestDuration(500));
        assert!(colors_equal(color, expected_middle));
        
        let (color, _) = sequence.evaluate(TestDuration(999));
        assert!(colors_equal(color, RED));
        
        let (color, _) = sequence.evaluate(TestDuration(3000));
        assert!(colors_equal(color, BLUE));
        
        let (color, _) = sequence.evaluate(TestDuration(3500));
        assert!(colors_equal(color, expected_middle));
        
        let (color, _) = sequence.evaluate(TestDuration(3999));
        assert!(colors_equal(color, RED));
    }

    #[test]
    fn start_color_used_for_first_linear_step_first_loop_only() {
        // Create sequence with start_color = BLACK and first step = RED with Linear transition
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .start_color(BLACK)
            .step(RED, TestDuration(1000), TransitionStyle::Linear)
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .loop_count(LoopCount::Infinite)
            .build()
            .unwrap();

        assert_eq!(sequence.start_color(), Some(BLACK));

        // First loop - should interpolate from BLACK to RED
        let (color, _) = sequence.evaluate(TestDuration(0));
        assert!(colors_equal(color, BLACK)); // Start from BLACK

        let (color, _) = sequence.evaluate(TestDuration(500));
        let expected_middle = BLACK.mix(RED, 0.5);
        assert!(colors_equal(color, expected_middle)); // Halfway from BLACK to RED

        let (color, _) = sequence.evaluate(TestDuration(999));
        assert!(colors_equal(color, RED)); // Almost at RED

        // Second loop - should interpolate from GREEN (last step) to RED
        let (color, _) = sequence.evaluate(TestDuration(2000));
        assert!(colors_equal(color, GREEN)); // Start from GREEN (last step's color)

        let (color, _) = sequence.evaluate(TestDuration(2500));
        let expected_middle = GREEN.mix(RED, 0.5);
        assert!(colors_equal(color, expected_middle)); // Halfway from GREEN to RED

        let (color, _) = sequence.evaluate(TestDuration(2999));
        assert!(colors_equal(color, RED)); // Almost at RED
    }

    #[test]
    fn start_color_not_used_when_first_step_is_step_transition() {
        // Create sequence with start_color but first step uses Step transition
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .start_color(BLACK)
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .step(GREEN, TestDuration(1000), TransitionStyle::Linear)
            .loop_count(LoopCount::Infinite)
            .build()
            .unwrap();

        // First step is Step transition, so it should just show RED immediately
        let (color, _) = sequence.evaluate(TestDuration(0));
        assert!(colors_equal(color, RED));

        let (color, _) = sequence.evaluate(TestDuration(500));
        assert!(colors_equal(color, RED));
    }

    #[test]
    fn start_color_with_finite_loops() {
        // Test that start_color only affects the first loop even with finite loops
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .start_color(YELLOW)
            .step(RED, TestDuration(1000), TransitionStyle::Linear)
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .loop_count(LoopCount::Finite(3))
            .build()
            .unwrap();

        // Loop 0 - interpolate from YELLOW to RED
        let (color, _) = sequence.evaluate(TestDuration(0));
        assert!(colors_equal(color, YELLOW));

        let (color, _) = sequence.evaluate(TestDuration(500));
        assert!(colors_equal(color, YELLOW.mix(RED, 0.5)));

        // Loop 1 - interpolate from GREEN to RED
        let (color, _) = sequence.evaluate(TestDuration(2000));
        assert!(colors_equal(color, GREEN));

        let (color, _) = sequence.evaluate(TestDuration(2500));
        assert!(colors_equal(color, GREEN.mix(RED, 0.5)));

        // Loop 2 - still interpolate from GREEN to RED
        let (color, _) = sequence.evaluate(TestDuration(4000));
        assert!(colors_equal(color, GREEN));

        let (color, _) = sequence.evaluate(TestDuration(4500));
        assert!(colors_equal(color, GREEN.mix(RED, 0.5)));
    }

    #[test]
    fn multi_step_sequence_progresses() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .build()
            .unwrap();

        let (color, _) = sequence.evaluate(TestDuration(50));
        assert!(colors_equal(color, RED));
        
        let (color, _) = sequence.evaluate(TestDuration(150));
        assert!(colors_equal(color, GREEN));
        
        let (color, _) = sequence.evaluate(TestDuration(250));
        assert!(colors_equal(color, BLUE));
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
        
        let (color, _) = sequence.evaluate(TestDuration(50));
        assert!(colors_equal(color, RED));
        
        let (color, _) = sequence.evaluate(TestDuration(350));
        assert!(colors_equal(color, GREEN));
        
        let (color, timing) = sequence.evaluate(TestDuration(400));
        assert!(colors_equal(color, BLACK));
        assert_eq!(timing, None);
        
        let (color, _) = sequence.evaluate(TestDuration(1000));
        assert!(colors_equal(color, BLACK));
    }

    #[test]
    fn finite_loop_uses_last_step_color_when_no_landing_color() {
        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .loop_count(LoopCount::Finite(2))
            .build()
            .unwrap();

        let (color, _) = sequence.evaluate(TestDuration(50));
        assert!(colors_equal(color, RED));
        
        let (color, _) = sequence.evaluate(TestDuration(450));
        assert!(colors_equal(color, GREEN));
        
        let (color, _) = sequence.evaluate(TestDuration(600));
        assert!(colors_equal(color, BLUE));
        
        let (color, _) = sequence.evaluate(TestDuration(1000));
        assert!(colors_equal(color, BLUE));
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

        let (color, _) = sequence.evaluate(TestDuration(50));
        assert!(colors_equal(color, RED));
        
        let (color, _) = sequence.evaluate(TestDuration(350));
        assert!(colors_equal(color, GREEN));
        
        let (color, _) = sequence.evaluate(TestDuration(10050));
        assert!(colors_equal(color, RED));
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

        let (color, _) = sequence.evaluate(TestDuration(0));
        assert!(colors_equal(color, RED));
        
        let (color, _) = sequence.evaluate(TestDuration(1));
        assert!(colors_equal(color, BLUE));
        
        let (color, _) = sequence.evaluate(TestDuration(100));
        assert!(colors_equal(color, BLUE));
    }
}
