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
fn solid_creates_single_step_sequence() {
    let seq = RgbSequence::<TestDuration, 1>::solid(RED, TestDuration(1000)).unwrap();

    // Should have exactly one step
    assert_eq!(seq.step_count(), 1);

    // Should use Step transition
    let step = seq.get_step(0).unwrap();
    assert!(colors_equal(step.color, RED));
    assert_eq!(step.duration, TestDuration(1000));
    assert_eq!(step.transition, TransitionStyle::Step);

    // Should hold the color for the duration
    let (color, timing) = seq.evaluate(TestDuration(500));
    assert!(colors_equal(color, RED));
    assert_eq!(timing, Some(TestDuration(500))); // 500ms remaining

    // Should complete after duration
    let (color, timing) = seq.evaluate(TestDuration(1000));
    assert!(colors_equal(color, RED));
    assert_eq!(timing, None); // Sequence complete
}

#[test]
fn solid_requires_capacity_of_at_least_one() {
    let result = RgbSequence::<TestDuration, 0>::solid(RED, TestDuration(1000));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SequenceError::CapacityExceeded
    ));
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
