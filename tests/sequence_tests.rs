//! Integration tests for RgbSequence

mod common;
use common::*;

use palette::{Mix, Srgb};
use rgb_sequencer::sequence::RgbSequence;
use rgb_sequencer::types::{LoopCount, SequenceError, TransitionStyle};
use rgb_sequencer::{TimeDuration, YELLOW};

#[test]
fn builder_rejects_empty_sequence() {
    let result = RgbSequence::<TestDuration, 8>::builder().build();
    assert!(matches!(result, Err(SequenceError::EmptySequence)));
}

#[test]
fn builder_rejects_zero_duration_with_non_step() {
    let result = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(0), TransitionStyle::Linear)
        .unwrap()
        .build();
    assert!(matches!(result, Err(SequenceError::ZeroDurationWithLinear)));

    let result = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(0), TransitionStyle::EaseIn)
        .unwrap()
        .build();
    assert!(matches!(result, Err(SequenceError::ZeroDurationWithLinear)));

    let result = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(0), TransitionStyle::EaseOut)
        .unwrap()
        .build();
    assert!(matches!(result, Err(SequenceError::ZeroDurationWithLinear)));

    let result = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(0), TransitionStyle::EaseInOut)
        .unwrap()
        .build();
    assert!(matches!(result, Err(SequenceError::ZeroDurationWithLinear)));
}

#[test]
fn builder_rejects_start_color_with_step_transition() {
    // start_color only applies to interpolating transitions
    let result = RgbSequence::<TestDuration, 8>::builder()
        .start_color(BLACK)
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build();
    assert!(matches!(
        result,
        Err(SequenceError::StartColorWithStepTransition)
    ));

    // But should accept with interpolating first step
    let result = RgbSequence::<TestDuration, 8>::builder()
        .start_color(BLACK)
        .step(RED, TestDuration(1000), TransitionStyle::Linear)
        .unwrap()
        .build();
    assert!(result.is_ok());

    let result = RgbSequence::<TestDuration, 8>::builder()
        .start_color(BLACK)
        .step(RED, TestDuration(1000), TransitionStyle::EaseIn)
        .unwrap()
        .build();
    assert!(result.is_ok());

    let result = RgbSequence::<TestDuration, 8>::builder()
        .start_color(BLACK)
        .step(RED, TestDuration(1000), TransitionStyle::EaseOut)
        .unwrap()
        .build();
    assert!(result.is_ok());

    let result = RgbSequence::<TestDuration, 8>::builder()
        .start_color(BLACK)
        .step(RED, TestDuration(1000), TransitionStyle::EaseInOut)
        .unwrap()
        .build();
    assert!(result.is_ok());
}

#[test]
fn builder_rejects_landing_color_with_infinite_loop() {
    // landing_color only applies to finite sequences
    let result = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Infinite)
        .landing_color(BLACK)
        .build();
    assert!(matches!(
        result,
        Err(SequenceError::LandingColorWithInfiniteLoop)
    ));

    // Should accept with finite loop
    let result = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .landing_color(BLACK)
        .build();
    assert!(result.is_ok());
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
fn solid_creates_single_step_sequence() {
    // BEHAVIOR: solid() creates a single-step sequence that holds one color with zero duration
    let seq = RgbSequence::<TestDuration, 1>::solid(RED).unwrap();

    assert_eq!(seq.step_count(), 1);
    let step = seq.get_step(0).unwrap();
    assert!(colors_equal(step.color, RED));
    assert_eq!(step.duration, TestDuration::ZERO);
    assert_eq!(step.transition, TransitionStyle::Step);
}

#[test]
fn solid_requires_capacity_of_at_least_one() {
    let result = RgbSequence::<TestDuration, 0>::solid(RED);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SequenceError::CapacityExceeded
    ));
}

#[test]
fn function_based_sequence_applies_function_to_base_color() {
    // BEHAVIOR: Function receives base_color as parameter, allowing reusable color functions
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

    let seq = RgbSequence::<TestDuration, 8>::from_function(RED, brightness_pulse, test_timing);

    assert!(seq.is_function_based());
    assert_eq!(seq.start_color(), Some(RED));

    // Function modulates base color: RED at 50%, then 100%
    assert!(colors_equal(
        seq.evaluate(TestDuration(100)).0,
        Srgb::new(0.5, 0.0, 0.0)
    ));
    assert!(colors_equal(seq.evaluate(TestDuration(600)).0, RED));
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
    // BEHAVIOR: Step transition holds constant color for entire duration
    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    assert!(colors_equal(sequence.evaluate(TestDuration(0)).0, RED));
    assert!(colors_equal(sequence.evaluate(TestDuration(999)).0, RED));
}

#[test]
fn linear_transition_interpolates_correctly() {
    // BEHAVIOR: Linear transition smoothly interpolates from previous color to target color
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
    // BEHAVIOR: When first step uses Linear and no start color set, loops interpolate from last step (seamless looping)
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
    // BEHAVIOR: start_color provides smooth entry into first loop only.
    // Subsequent loops interpolate from last step for seamless looping.
    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .start_color(BLACK)
        .step(RED, TestDuration(1000), TransitionStyle::Linear)
        .unwrap()
        .step(GREEN, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap();

    // First loop: BLACK → RED (uses start_color)
    assert!(colors_equal(sequence.evaluate(TestDuration(0)).0, BLACK));
    assert!(colors_equal(
        sequence.evaluate(TestDuration(500)).0,
        BLACK.mix(RED, 0.5)
    ));

    // Second loop: GREEN → RED (last step → first step)
    assert!(colors_equal(sequence.evaluate(TestDuration(2000)).0, GREEN));
    assert!(colors_equal(
        sequence.evaluate(TestDuration(2500)).0,
        GREEN.mix(RED, 0.5)
    ));
}

#[test]
fn start_color_only_affects_first_loop_with_finite_loops() {
    // BEHAVIOR: start_color applies to first loop only, regardless of loop count
    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .start_color(YELLOW)
        .step(RED, TestDuration(1000), TransitionStyle::Linear)
        .unwrap()
        .step(GREEN, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(3))
        .build()
        .unwrap();

    // Loop 0: YELLOW → RED
    assert!(colors_equal(
        sequence.evaluate(TestDuration(500)).0,
        YELLOW.mix(RED, 0.5)
    ));

    // Loop 1 and beyond: GREEN → RED
    assert!(colors_equal(
        sequence.evaluate(TestDuration(2500)).0,
        GREEN.mix(RED, 0.5)
    ));
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
        .build()
        .unwrap();

    let (color, _) = sequence.evaluate(TestDuration(50));
    assert!(colors_equal(color, RED));

    let (color, _) = sequence.evaluate(TestDuration(350));
    assert!(colors_equal(color, GREEN));

    let (color, _) = sequence.evaluate(TestDuration(1000050));
    assert!(colors_equal(color, RED));

    let (color, _) = sequence.evaluate(TestDuration(1000350));
    assert!(colors_equal(color, GREEN));
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
        .step(RED, TestDuration(100), TransitionStyle::Linear)
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
    assert_eq!(step0.transition, TransitionStyle::Linear);
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
fn loop_boundaries_transition_at_exact_step_durations() {
    // BEHAVIOR: Steps transition precisely at duration boundaries (critical for timing accuracy)
    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(2))
        .landing_color(BLUE)
        .build()
        .unwrap();

    // Step boundaries: transition at exact duration, not before/after
    assert!(colors_equal(sequence.evaluate(TestDuration(99)).0, RED));
    assert!(colors_equal(sequence.evaluate(TestDuration(100)).0, GREEN));

    // Loop boundaries: restart at exact loop duration
    assert!(colors_equal(sequence.evaluate(TestDuration(199)).0, GREEN));
    assert!(colors_equal(sequence.evaluate(TestDuration(200)).0, RED));

    // Completion boundary: landing color at exact completion time
    assert!(colors_equal(sequence.evaluate(TestDuration(399)).0, GREEN));
    assert!(colors_equal(sequence.evaluate(TestDuration(400)).0, BLUE));
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
fn ease_in_transition_accelerates() {
    // BEHAVIOR: EaseIn starts slow and accelerates (quadratic: t²)
    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .start_color(BLACK)
        .step(RED, TestDuration(1000), TransitionStyle::EaseIn)
        .unwrap()
        .build()
        .unwrap();

    // At 50% time: 0.5² = 0.25 progress (slow start)
    assert!(
        sequence.evaluate(TestDuration(500)).0.red > 0.2
            && sequence.evaluate(TestDuration(500)).0.red < 0.3
    );

    // At 100% time: reaches full color
    assert!(colors_equal(sequence.evaluate(TestDuration(1000)).0, RED));
}

#[test]
fn ease_out_transition_decelerates() {
    // BEHAVIOR: EaseOut starts fast and decelerates (quadratic: t * (2 - t))
    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .start_color(BLACK)
        .step(RED, TestDuration(1000), TransitionStyle::EaseOut)
        .unwrap()
        .build()
        .unwrap();

    // At 50% time: 0.5 * 1.5 = 0.75 progress (fast start)
    assert!(
        sequence.evaluate(TestDuration(500)).0.red > 0.7
            && sequence.evaluate(TestDuration(500)).0.red < 0.8
    );

    // At 100% time: reaches full color
    assert!(colors_equal(sequence.evaluate(TestDuration(1000)).0, RED));
}

#[test]
fn ease_in_out_transition_symmetric() {
    // BEHAVIOR: EaseInOut is slow at both ends, fast in middle (S-curve)
    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .start_color(BLACK)
        .step(RED, TestDuration(1000), TransitionStyle::EaseInOut)
        .unwrap()
        .build()
        .unwrap();

    // At 25% time: ease-in phase (slow start, progress < 0.2)
    assert!(sequence.evaluate(TestDuration(250)).0.red < 0.2);

    // At 50% time: midpoint (progress ≈ 0.5)
    assert!(
        sequence.evaluate(TestDuration(500)).0.red > 0.49
            && sequence.evaluate(TestDuration(500)).0.red < 0.51
    );

    // At 75% time: ease-out phase (fast middle, progress > 0.8)
    assert!(sequence.evaluate(TestDuration(750)).0.red > 0.8);

    // At 100% time: reaches full color
    assert!(colors_equal(sequence.evaluate(TestDuration(1000)).0, RED));
}

#[test]
fn ease_out_in_transition_inverted_curve() {
    // BEHAVIOR: EaseOutIn is fast at both ends, slow in middle (inverted S-curve)
    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .start_color(BLACK)
        .step(RED, TestDuration(1000), TransitionStyle::EaseOutIn)
        .unwrap()
        .build()
        .unwrap();

    // At 25% time: fast start phase (progress ≈ 0.375)
    assert!(
        sequence.evaluate(TestDuration(250)).0.red > 0.35
            && sequence.evaluate(TestDuration(250)).0.red < 0.40
    );

    // At 50% time: midpoint (progress ≈ 0.5)
    assert!(
        sequence.evaluate(TestDuration(500)).0.red > 0.49
            && sequence.evaluate(TestDuration(500)).0.red < 0.51
    );

    // At 75% time: slow middle transitioning to fast end (progress ≈ 0.625)
    assert!(
        sequence.evaluate(TestDuration(750)).0.red > 0.60
            && sequence.evaluate(TestDuration(750)).0.red < 0.65
    );

    // At 100% time: reaches full color
    assert!(colors_equal(sequence.evaluate(TestDuration(1000)).0, RED));
}

#[test]
fn easing_transitions_return_continuous_timing() {
    let test_cases = [
        TransitionStyle::EaseIn,
        TransitionStyle::EaseOut,
        TransitionStyle::EaseInOut,
        TransitionStyle::EaseOutIn,
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
