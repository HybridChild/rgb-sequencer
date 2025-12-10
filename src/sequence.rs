//! RGB color sequence definitions and evaluation.

use crate::COLOR_OFF;
use crate::time::TimeDuration;
use crate::types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};
use heapless::Vec;
use palette::{Mix, Srgb};

/// Applies easing function to linear progress value (0.0 to 1.0).
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

    /// Creates a function-based sequence.
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

    /// Evaluates color and next service time at elapsed time.
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

    /// Checks if a step-based finite sequence has completed all loops at the given elapsed time.
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

    fn find_step_position(&self, elapsed: D) -> Option<StepPosition<D>> {
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
    pub fn loop_duration(&self) -> D {
        self.loop_duration
    }

    /// Returns step count.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Returns loop count.
    pub fn loop_count(&self) -> LoopCount {
        self.loop_count
    }

    /// Returns landing color.
    pub fn landing_color(&self) -> Option<Srgb> {
        self.landing_color
    }

    /// Returns start color.
    pub fn start_color(&self) -> Option<Srgb> {
        self.start_color
    }

    /// Returns step at index.
    pub fn get_step(&self, index: usize) -> Option<&SequenceStep<D>> {
        self.steps.get(index)
    }

    /// Returns true if function-based.
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

    /// Adds a step.
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

    /// Sets loop count.
    pub fn loop_count(mut self, count: LoopCount) -> Self {
        self.loop_count = count;
        self
    }

    /// Sets landing color.
    pub fn landing_color(mut self, color: Srgb) -> Self {
        self.landing_color = Some(color);
        self
    }

    /// Sets start color.
    pub fn start_color(mut self, color: Srgb) -> Self {
        self.start_color = Some(color);
        self
    }

    /// Builds and validates sequence.
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
        let result = RgbSequence::<TestDuration, 8>::builder().build();
        assert!(matches!(result, Err(SequenceError::EmptySequence)));
    }

    #[test]
    fn builder_rejects_zero_duration_with_linear() {
        let result = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(0), TransitionStyle::Linear)
            .unwrap()
            .build();
        assert!(matches!(result, Err(SequenceError::ZeroDurationWithLinear)));
    }

    #[test]
    fn builder_accepts_valid_sequence() {
        let result = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(200), TransitionStyle::Linear)
            .unwrap()
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn function_based_sequence_applies_function_to_base_color() {
        // Brightness modulation function - works with any base color
        fn brightness_pulse(base: Srgb, elapsed: TestDuration) -> Srgb {
            let brightness = if elapsed.as_millis() < 500 { 0.5 } else { 1.0 };
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
        let red_pulse =
            RgbSequence::<TestDuration, 8>::from_function(RED, brightness_pulse, test_timing);

        let blue_pulse =
            RgbSequence::<TestDuration, 8>::from_function(BLUE, brightness_pulse, test_timing);

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
    fn function_based_sequence_respects_timing_function() {
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
        assert!(seq.has_completed(TestDuration(1000)));
    }

    #[test]
    fn evaluate_returns_both_color_and_timing() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(200), TransitionStyle::Linear)
            .unwrap()
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
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(200), TransitionStyle::Step)
            .unwrap()
            .step(BLUE, TestDuration(50), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(sequence.loop_duration(), TestDuration(350));
    }

    #[test]
    fn step_transition_holds_color() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
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
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(BLUE, TestDuration(1000), TransitionStyle::Linear)
            .unwrap()
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
    fn first_step_with_linear_transition_interpolates_from_last_step() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Linear)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .step(BLUE, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
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
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .start_color(BLACK)
            .step(RED, TestDuration(1000), TransitionStyle::Linear)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
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
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .start_color(BLACK)
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Linear)
            .unwrap()
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
    fn start_color_only_affects_first_loop_with_finite_loops() {
        // Test that start_color only affects the first loop even with finite loops
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .start_color(YELLOW)
            .step(RED, TestDuration(1000), TransitionStyle::Linear)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
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
    fn multi_step_sequence_progresses_through_steps_over_time() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .unwrap()
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
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
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
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
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
    fn sequence_with_all_zero_duration_steps_completes_immediately() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(0), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(0), TransitionStyle::Step)
            .unwrap()
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

    #[test]
    fn query_methods_return_correct_sequence_properties() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(200), TransitionStyle::Linear)
            .unwrap()
            .step(BLUE, TestDuration(50), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(3))
            .landing_color(YELLOW)
            .start_color(BLACK)
            .build()
            .unwrap();

        assert_eq!(sequence.step_count(), 3);
        assert_eq!(sequence.loop_count(), LoopCount::Finite(3));
        assert_eq!(sequence.loop_duration(), TestDuration(350));
        assert_eq!(sequence.landing_color(), Some(YELLOW));
        assert_eq!(sequence.start_color(), Some(BLACK));
        assert!(!sequence.is_function_based());

        // Test get_step
        assert!(sequence.get_step(0).is_some());
        assert!(sequence.get_step(1).is_some());
        assert!(sequence.get_step(2).is_some());
        assert!(sequence.get_step(3).is_none());

        let step0 = sequence.get_step(0).unwrap();
        assert!(colors_equal(step0.color, RED));
        assert_eq!(step0.duration, TestDuration(100));
        assert_eq!(step0.transition, TransitionStyle::Step);
    }

    #[test]
    fn has_completed_for_step_based_finite_sequence() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(200), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(2))
            .build()
            .unwrap();

        // Total duration: 2 loops * 300ms = 600ms
        assert!(!sequence.has_completed(TestDuration(0)));
        assert!(!sequence.has_completed(TestDuration(299)));
        assert!(!sequence.has_completed(TestDuration(300))); // First loop done
        assert!(!sequence.has_completed(TestDuration(599)));
        assert!(sequence.has_completed(TestDuration(600))); // Both loops done
        assert!(sequence.has_completed(TestDuration(1000)));
    }

    #[test]
    fn infinite_sequence_never_reports_completion() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Infinite)
            .build()
            .unwrap();

        assert!(!sequence.has_completed(TestDuration(0)));
        assert!(!sequence.has_completed(TestDuration(1000)));
        assert!(!sequence.has_completed(TestDuration(1000000)));
    }

    #[test]
    fn function_based_sequence_query_methods_return_expected_values() {
        fn test_color(base: Srgb, _elapsed: TestDuration) -> Srgb {
            base
        }

        fn test_timing(_elapsed: TestDuration) -> Option<TestDuration> {
            Some(TestDuration::ZERO)
        }

        let sequence = RgbSequence::<TestDuration, 8>::from_function(RED, test_color, test_timing);

        assert!(sequence.is_function_based());
        assert_eq!(sequence.step_count(), 0);
        assert_eq!(sequence.loop_duration(), TestDuration::ZERO);
        assert_eq!(sequence.start_color(), Some(RED));
        assert!(sequence.get_step(0).is_none());
    }

    #[test]
    fn sequence_at_max_capacity_works_correctly() {
        // Build a sequence with exactly 4 steps (capacity)
        let sequence = RgbSequence::<TestDuration, 4>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(YELLOW, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(sequence.step_count(), 4);

        let (color, _) = sequence.evaluate(TestDuration(0));
        assert!(colors_equal(color, RED));

        let (color, _) = sequence.evaluate(TestDuration(150));
        assert!(colors_equal(color, GREEN));

        let (color, _) = sequence.evaluate(TestDuration(350));
        assert!(colors_equal(color, YELLOW));
    }

    #[test]
    fn sequence_exceeds_capacity() {
        // Try to build a sequence with 5 steps when capacity is 4
        let result = RgbSequence::<TestDuration, 4>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(YELLOW, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(BLACK, TestDuration(100), TransitionStyle::Step); // This should error

        assert!(matches!(result, Err(SequenceError::CapacityExceeded)));
    }

    #[test]
    fn loop_boundaries_are_precise_to_millisecond() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(2))
            .landing_color(BLUE)
            .build()
            .unwrap();

        // Test exact loop boundaries
        let (color, _) = sequence.evaluate(TestDuration(0));
        assert!(colors_equal(color, RED)); // Loop 0, step 0

        let (color, _) = sequence.evaluate(TestDuration(99));
        assert!(colors_equal(color, RED)); // Loop 0, still step 0

        let (color, _) = sequence.evaluate(TestDuration(100));
        assert!(colors_equal(color, GREEN)); // Loop 0, step 1

        let (color, _) = sequence.evaluate(TestDuration(199));
        assert!(colors_equal(color, GREEN)); // Loop 0, still step 1

        let (color, _) = sequence.evaluate(TestDuration(200));
        assert!(colors_equal(color, RED)); // Loop 1, step 0

        let (color, _) = sequence.evaluate(TestDuration(300));
        assert!(colors_equal(color, GREEN)); // Loop 1, step 1

        let (color, _) = sequence.evaluate(TestDuration(399));
        assert!(colors_equal(color, GREEN)); // Loop 1, still step 1

        let (color, _) = sequence.evaluate(TestDuration(400));
        assert!(colors_equal(color, BLUE)); // Complete - landing color
    }

    #[test]
    fn sequence_with_mixed_transition_styles_works_correctly() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Linear)
            .unwrap()
            .step(BLUE, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(YELLOW, TestDuration(100), TransitionStyle::Linear)
            .unwrap()
            .build()
            .unwrap();

        // Step transition - holds color
        let (color, timing) = sequence.evaluate(TestDuration(50));
        assert!(colors_equal(color, RED));
        assert_eq!(timing, Some(TestDuration(50))); // Time until step end

        // Linear transition - interpolates
        let (color, timing) = sequence.evaluate(TestDuration(150));
        assert!(colors_equal(color, RED.mix(GREEN, 0.5)));
        assert_eq!(timing, Some(TestDuration::ZERO)); // Continuous

        // Another step transition
        let (color, _) = sequence.evaluate(TestDuration(250));
        assert!(colors_equal(color, BLUE));

        // Another linear transition
        let (color, _) = sequence.evaluate(TestDuration(350));
        assert!(colors_equal(color, BLUE.mix(YELLOW, 0.5)));
    }

    #[test]
    fn linear_interpolation_works_correctly_at_step_boundaries() {
        // Test that interpolation works correctly at boundaries
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::Linear)
            .unwrap()
            .loop_count(LoopCount::Infinite)
            .build()
            .unwrap();

        // At start of linear transition (beginning of step 1)
        let (color, _) = sequence.evaluate(TestDuration(100));
        assert!(colors_equal(color, RED));

        // Midway through linear transition
        let (color, _) = sequence.evaluate(TestDuration(150));
        assert!(colors_equal(color, RED.mix(GREEN, 0.5)));

        // Near end of linear transition (99% progress)
        let (color, _) = sequence.evaluate(TestDuration(199));
        assert!(colors_equal(color, RED.mix(GREEN, 0.99)));

        // Past end (should wrap to next loop, back to step 0 which is RED with Step transition)
        let (color, _) = sequence.evaluate(TestDuration(200));
        assert!(colors_equal(color, RED));
    }

    #[test]
    fn single_step_infinite_sequence_works_correctly() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Infinite)
            .build()
            .unwrap();

        let (color, timing) = sequence.evaluate(TestDuration(0));
        assert!(colors_equal(color, RED));
        assert_eq!(timing, Some(TestDuration(1000)));

        let (color, _) = sequence.evaluate(TestDuration(500));
        assert!(colors_equal(color, RED));

        let (color, _) = sequence.evaluate(TestDuration(1500));
        assert!(colors_equal(color, RED));
    }

    #[test]
    fn sequence_with_many_loops_handles_large_durations_without_overflow() {
        // Test with many loops to check for potential overflow issues
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .loop_count(LoopCount::Finite(1000))
            .build()
            .unwrap();

        assert!(!sequence.has_completed(TestDuration(99_900)));
        assert!(sequence.has_completed(TestDuration(100_000)));

        let (color, _) = sequence.evaluate(TestDuration(50_000));
        assert!(colors_equal(color, RED));
    }

    #[test]
    fn error_types_are_constructable() {
        // Verify error types can be constructed
        let _error1 = SequenceError::EmptySequence;
        let _error2 = SequenceError::ZeroDurationWithLinear;
    }

    #[test]
    fn ease_in_transition_accelerates() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .start_color(BLACK)
            .step(RED, TestDuration(1000), TransitionStyle::EaseIn)
            .unwrap()
            .build()
            .unwrap();

        // At 25% time, ease-in should be at less than 25% progress (0.25² = 0.0625)
        let (color_25, _) = sequence.evaluate(TestDuration(250));
        // Expected: mix(BLACK, RED, 0.0625) = (0.0625, 0, 0)
        assert!(color_25.red < 0.1);

        // At 50% time, ease-in should be at 25% progress (0.5² = 0.25)
        let (color_50, _) = sequence.evaluate(TestDuration(500));
        assert!(color_50.red > 0.2 && color_50.red < 0.3);

        // At 75% time, ease-in should be at ~56% progress (0.75² = 0.5625)
        let (color_75, _) = sequence.evaluate(TestDuration(750));
        assert!(color_75.red > 0.5 && color_75.red < 0.6);

        // At 100% time, should be at full color
        let (color_100, _) = sequence.evaluate(TestDuration(1000));
        assert!(colors_equal(color_100, RED));
    }

    #[test]
    fn ease_out_transition_decelerates() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .start_color(BLACK)
            .step(RED, TestDuration(1000), TransitionStyle::EaseOut)
            .unwrap()
            .build()
            .unwrap();

        // At 25% time, ease-out should be at more than 25% progress
        // Formula: t * (2 - t) = 0.25 * 1.75 = 0.4375
        let (color_25, _) = sequence.evaluate(TestDuration(250));
        assert!(color_25.red > 0.4 && color_25.red < 0.5);

        // At 50% time, ease-out should be at 75% progress (0.5 * 1.5 = 0.75)
        let (color_50, _) = sequence.evaluate(TestDuration(500));
        assert!(color_50.red > 0.7 && color_50.red < 0.8);

        // At 75% time, ease-out should be at ~94% progress (0.75 * 1.25 = 0.9375)
        let (color_75, _) = sequence.evaluate(TestDuration(750));
        assert!(color_75.red > 0.9);

        // At 100% time, should be at full color
        let (color_100, _) = sequence.evaluate(TestDuration(1000));
        assert!(colors_equal(color_100, RED));
    }

    #[test]
    fn ease_in_out_transition_symmetric() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .start_color(BLACK)
            .step(RED, TestDuration(1000), TransitionStyle::EaseInOut)
            .unwrap()
            .build()
            .unwrap();

        // At 25% time, should be in ease-in phase (slow)
        // Formula: 2 * t² = 2 * 0.25² = 0.125
        let (color_25, _) = sequence.evaluate(TestDuration(250));
        assert!(color_25.red < 0.2);

        // At 50% time, should be at midpoint
        let (color_50, _) = sequence.evaluate(TestDuration(500));
        assert!(color_50.red > 0.45 && color_50.red < 0.55);

        // At 75% time, should be in ease-out phase (fast then slow)
        // Formula: -1 + (4 - 2*t) * t = -1 + 2.5 * 0.75 = 0.875
        let (color_75, _) = sequence.evaluate(TestDuration(750));
        assert!(color_75.red > 0.8 && color_75.red < 0.9);

        // At 100% time, should be at full color
        let (color_100, _) = sequence.evaluate(TestDuration(1000));
        assert!(colors_equal(color_100, RED));
    }

    #[test]
    fn easing_transitions_return_continuous_timing() {
        let test_cases = [
            TransitionStyle::EaseIn,
            TransitionStyle::EaseOut,
            TransitionStyle::EaseInOut,
        ];

        for transition in test_cases {
            let sequence = RgbSequence::<TestDuration, 8>::builder()
                .step(RED, TestDuration(1000), transition)
                .unwrap()
                .build()
                .unwrap();

            let (_, timing) = sequence.evaluate(TestDuration(500));
            assert_eq!(
                timing,
                Some(TestDuration::ZERO),
                "Easing transitions should return continuous timing"
            );
        }
    }

    #[test]
    fn zero_duration_with_easing_is_rejected() {
        let test_cases = [
            TransitionStyle::EaseIn,
            TransitionStyle::EaseOut,
            TransitionStyle::EaseInOut,
        ];

        for transition in test_cases {
            let result = RgbSequence::<TestDuration, 8>::builder()
                .step(RED, TestDuration(0), transition)
                .unwrap()
                .build();

            assert!(
                matches!(result, Err(SequenceError::ZeroDurationWithLinear)),
                "Zero-duration with {:?} should be rejected",
                transition
            );
        }
    }

    #[test]
    fn easing_works_with_start_color() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .start_color(BLACK)
            .step(RED, TestDuration(1000), TransitionStyle::EaseIn)
            .unwrap()
            .build()
            .unwrap();

        // Should interpolate from BLACK to RED with ease-in
        let (color_50, _) = sequence.evaluate(TestDuration(500));
        // At 50% time with ease-in: 0.5² = 0.25 progress
        assert!(color_50.red > 0.2 && color_50.red < 0.3);
        assert!(color_50.green < 0.01);
        assert!(color_50.blue < 0.01);
    }

    #[test]
    fn easing_works_in_multi_step_sequences() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .unwrap()
            .step(GREEN, TestDuration(1000), TransitionStyle::EaseOut)
            .unwrap()
            .step(BLUE, TestDuration(1000), TransitionStyle::EaseIn)
            .unwrap()
            .build()
            .unwrap();

        // At 100ms: Should be RED (end of first step)
        let (color_100, _) = sequence.evaluate(TestDuration(100));
        assert!(colors_equal(color_100, RED));

        // At 600ms (500ms into ease-out transition): Should be mostly green
        // Ease-out at 50%: 0.5 * 1.5 = 0.75 progress from RED to GREEN
        let (color_600, _) = sequence.evaluate(TestDuration(600));
        assert!(color_600.red < 0.3); // Less red
        assert!(color_600.green > 0.7); // More green

        // At 1600ms (500ms into ease-in transition): Should be transitioning slowly to blue
        // Ease-in at 50%: 0.5² = 0.25 progress from GREEN to BLUE
        let (color_1600, _) = sequence.evaluate(TestDuration(1600));
        assert!(color_1600.green > 0.7); // Still mostly green
        assert!(color_1600.blue < 0.3); // Not much blue yet
    }

    #[test]
    fn easing_loops_correctly() {
        let sequence = RgbSequence::<TestDuration, 8>::builder()
            .step(RED, TestDuration(100), TransitionStyle::EaseIn)
            .unwrap()
            .step(GREEN, TestDuration(100), TransitionStyle::EaseOut)
            .unwrap()
            .loop_count(LoopCount::Infinite)
            .build()
            .unwrap();

        // First loop
        let (color_50, _) = sequence.evaluate(TestDuration(50));
        let first_loop_red = color_50.red;

        // Second loop - should have same color at same position
        let (color_250, _) = sequence.evaluate(TestDuration(250));
        assert!((color_250.red - first_loop_red).abs() < 0.01);
    }
}
