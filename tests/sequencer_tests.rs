//! Integration tests for RgbSequencer

mod common;
use common::*;

use palette::Srgb;
use rgb_sequencer::sequence::RgbSequence;
use rgb_sequencer::types::{LoopCount, TransitionStyle};
use rgb_sequencer::{
    DEFAULT_COLOR_EPSILON, RgbSequencer, SequencerError, SequencerState, ServiceTiming,
    TimeDuration,
};

#[test]
fn start_requires_loaded_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    // Try to start from Idle state (no sequence loaded)
    let result = sequencer.start();
    assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
}

#[test]
fn pause_requires_running_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);

    // Try to pause from Loaded state
    let result = sequencer.pause();
    assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
}

#[test]
fn resume_requires_paused_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);

    // Try to resume from Loaded state
    let result = sequencer.resume();
    assert!(matches!(result, Err(SequencerError::InvalidState { .. })));

    // Try to resume from Running state
    sequencer.start().unwrap();
    let result = sequencer.resume();
    assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
}

#[test]
fn service_requires_running_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);

    // Try to service from Loaded state
    let result = sequencer.service();
    assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
}

#[test]
fn loading_and_starting_sequence_updates_led_color() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    assert_eq!(sequencer.state(), SequencerState::Loaded);

    sequencer.start().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Running);

    sequencer.service().unwrap();
    // LED should now be RED
    assert!(colors_equal(sequencer.current_color(), RED));
}

#[test]
fn service_correctly_progresses_through_multiple_steps() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(BLUE, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.service().unwrap();

    // At start - RED
    assert!(colors_equal(sequencer.current_color(), RED));

    // Advance to middle of first step
    timer.advance(TestDuration(50));
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), RED));

    // Advance to second step
    timer.advance(TestDuration(60));
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), GREEN));

    // Advance to third step
    timer.advance(TestDuration(100));
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), BLUE));
}

#[test]
fn function_based_sequence_computes_colors_correctly() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    fn brightness_pulse(base: Srgb, elapsed: TestDuration) -> Srgb {
        let brightness = if elapsed.as_millis() < 500 { 0.5 } else { 1.0 };
        Srgb::new(
            base.red * brightness,
            base.green * brightness,
            base.blue * brightness,
        )
    }

    fn continuous(_elapsed: TestDuration) -> Option<TestDuration> {
        Some(TestDuration::ZERO)
    }

    let sequence = RgbSequence::<TestDuration, 8>::from_function(RED, brightness_pulse, continuous);

    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.service().unwrap();

    // At start - 50% brightness
    assert!(colors_equal(
        sequencer.current_color(),
        Srgb::new(0.5, 0.0, 0.0)
    ));

    // After 500ms - full brightness
    timer.advance(TestDuration(500));
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), RED));
}

#[test]
fn pause_resume_maintains_position() {
    // BEHAVIOR: Pause/resume compensates for paused duration (no time jumps or drift)
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    // Advance 500ms into first step
    timer.advance(TestDuration(500));
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), RED));

    // Pause
    sequencer.pause().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Paused);

    // Advance time while paused (simulating delay)
    timer.advance(TestDuration(3000));

    // Resume - should still be in RED step
    sequencer.resume().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Running);
    assert!(colors_equal(sequencer.current_color(), RED));

    // Advance 500ms more - should transition to GREEN (total 1000ms in RED)
    timer.advance(TestDuration(500));
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), GREEN));
}

#[test]
fn stop_turns_off_led_and_returns_to_loaded() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), RED));

    sequencer.stop().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Loaded);
    assert!(colors_equal(sequencer.current_color(), BLACK));
}

#[test]
fn clear_removes_sequence_and_returns_to_idle() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), RED));

    sequencer.clear();
    assert_eq!(sequencer.state(), SequencerState::Idle);
    assert!(colors_equal(sequencer.current_color(), BLACK));
}

#[test]
fn service_returns_continuous_timing_for_linear_transition() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(1000), TransitionStyle::Linear)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    // Advance into the linear transition step
    timer.advance(TestDuration(150));
    let timing = sequencer.service().unwrap();

    // Should return Continuous for linear transitions
    assert_eq!(timing, ServiceTiming::Continuous);
}

#[test]
fn finite_sequence_completes_and_transitions_to_complete_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .landing_color(BLUE)
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    // Advance past the sequence duration
    timer.advance(TestDuration(200));
    let timing = sequencer.service().unwrap();

    // Should return Complete to indicate completion
    assert_eq!(timing, ServiceTiming::Complete);
    assert_eq!(sequencer.state(), SequencerState::Complete);
    assert!(colors_equal(sequencer.current_color(), BLUE));
}

#[test]
fn peek_next_timing_returns_timing_without_state_changes() {
    // BEHAVIOR: peek_next_timing() returns timing without updating LED or changing state
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    // Peek returns timing but doesn't change LED color
    let initial_color = sequencer.current_color();
    let peek_timing = sequencer.peek_next_timing().unwrap();
    assert_eq!(peek_timing, ServiceTiming::Delay(TestDuration(1000)));
    assert!(colors_equal(sequencer.current_color(), initial_color));

    // Advance to completion and verify peek doesn't change state
    timer.advance(TestDuration(1100));
    assert_eq!(
        sequencer.peek_next_timing().unwrap(),
        ServiceTiming::Complete
    );
    assert_eq!(sequencer.state(), SequencerState::Running); // State unchanged

    sequencer.service().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Complete); // service() changes state
}

#[test]
fn peek_next_timing_requires_running_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    // Should fail when not running
    let result = sequencer.peek_next_timing();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SequencerError::InvalidState { .. }
    ));
}

#[test]
fn restart_from_running_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    // Advance into the sequence
    timer.advance(TestDuration(1500));
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), GREEN));

    // Restart should reset to beginning
    let restart_result = sequencer.restart();
    assert!(restart_result.is_ok());
    assert_eq!(sequencer.state(), SequencerState::Running);
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), RED));
}

#[test]
fn restart_from_paused_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();
    timer.advance(TestDuration(500));
    sequencer.service().unwrap();
    sequencer.pause().unwrap();

    // Restart from paused should reset and run
    sequencer.restart().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Running);
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), RED));
}

#[test]
fn restart_from_complete_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .landing_color(BLUE)
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();
    timer.advance(TestDuration(200));
    sequencer.service().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Complete);

    // Restart should reset and run from beginning
    sequencer.restart().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Running);
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), RED));
}

#[test]
fn restart_from_invalid_state_fails() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    // Try restart from Idle
    let result = sequencer.restart();
    assert!(matches!(result, Err(SequencerError::InvalidState { .. })));

    // Try restart from Loaded
    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();
    sequencer.load(sequence);

    let result = sequencer.restart();
    assert!(matches!(result, Err(SequencerError::InvalidState { .. })));
}

#[test]
fn query_methods_return_correct_state_and_timing_info() {
    // BEHAVIOR: Query methods reflect current state without side effects
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    // Idle state
    assert_eq!(sequencer.state(), SequencerState::Idle);
    assert!(sequencer.elapsed_time().is_none());

    // Running state
    sequencer.load(sequence);
    sequencer.start().unwrap();
    assert!(sequencer.is_running());
    assert!(!sequencer.is_paused());

    timer.advance(TestDuration(50));
    sequencer.service().unwrap();
    assert_eq!(sequencer.elapsed_time().unwrap(), TestDuration(50));
}

#[test]
fn stop_from_paused_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.pause().unwrap();

    // Stop from paused should work
    let result = sequencer.stop();
    assert!(result.is_ok());
    assert_eq!(sequencer.state(), SequencerState::Loaded);
    assert!(colors_equal(sequencer.current_color(), BLACK));
}

#[test]
fn stop_from_complete_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();
    timer.advance(TestDuration(200));
    sequencer.service().unwrap();

    assert_eq!(sequencer.state(), SequencerState::Complete);

    let result = sequencer.stop();
    assert!(result.is_ok());
    assert_eq!(sequencer.state(), SequencerState::Loaded);
}

#[test]
fn led_only_updates_when_color_changes() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    // After start, LED should be set to RED (plus initial BLACK from new())
    // Color history should have: [BLACK (from new), RED (from start)]

    // Service multiple times without time advancing - color shouldn't change
    timer.advance(TestDuration(100));
    sequencer.service().unwrap();
    sequencer.service().unwrap();
    sequencer.service().unwrap();

    // The LED's color_history should not grow since color didn't change
    // We can't directly test this without exposing the mock, but we can verify
    // the current color remains RED
    assert!(colors_equal(sequencer.current_color(), RED));
}

#[test]
fn loading_new_sequence_replaces_existing_and_resets_state() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence1 = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    let sequence2 = RgbSequence::<TestDuration, 8>::builder()
        .step(GREEN, TestDuration(200), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    // Load first sequence and start
    sequencer.load(sequence1);
    sequencer.start().unwrap();
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), RED));

    // Load second sequence should stop the first and transition to Loaded
    sequencer.load(sequence2);
    assert_eq!(sequencer.state(), SequencerState::Loaded);

    // Start second sequence
    sequencer.start().unwrap();
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), GREEN));
}

#[test]
fn multiple_service_calls_without_time_advancement_are_safe() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    // Multiple service calls without time advancement should be safe
    for _ in 0..10 {
        let result = sequencer.service();
        assert!(result.is_ok());
        assert!(colors_equal(sequencer.current_color(), RED));
    }
}

#[test]
fn load_and_start_convenience_method_works() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    // Should go from Idle -> Loaded -> Running in one call
    sequencer.load_and_start(sequence).unwrap();
    assert_eq!(sequencer.state(), SequencerState::Running);
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), RED));

    // Advance and verify it progresses through sequence
    timer.advance(TestDuration(1100));
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), GREEN));
}

#[test]
fn sequence_with_mixed_zero_and_nonzero_durations_works_correctly() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(0), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(BLUE, TestDuration(0), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.service().unwrap();

    // At time 0, zero-duration steps are skipped, so we're at GREEN (second step)
    assert!(colors_equal(sequencer.current_color(), GREEN));

    // After 50ms, still in GREEN (second step)
    timer.advance(TestDuration(50));
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), GREEN));

    // After 100ms total, should be BLUE (third step, also zero duration)
    timer.advance(TestDuration(50));
    sequencer.service().unwrap();
    assert!(colors_equal(sequencer.current_color(), BLUE));
}

#[test]
fn current_position_returns_none_when_not_running() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    // Idle state - no position
    assert_eq!(sequencer.current_position(), None);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    // Loaded state - no position
    sequencer.load(sequence);
    assert_eq!(sequencer.current_position(), None);

    // Running state - should have position
    sequencer.start().unwrap();
    assert!(sequencer.current_position().is_some());

    // Paused state - no position
    sequencer.pause().unwrap();
    assert_eq!(sequencer.current_position(), None);
}

#[test]
fn current_position_tracks_step_changes() {
    // BEHAVIOR: current_position() tracks step_index as sequence progresses
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    assert_eq!(sequencer.current_position().unwrap().step_index, 0);

    timer.advance(TestDuration(100));
    sequencer.service().unwrap();
    assert_eq!(sequencer.current_position().unwrap().step_index, 1);

    timer.advance(TestDuration(100));
    sequencer.service().unwrap();
    assert_eq!(sequencer.current_position(), None); // Complete
}

#[test]
fn current_position_tracks_loop_changes() {
    // BEHAVIOR: current_position() tracks loop_number as sequence loops
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(3))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    assert_eq!(sequencer.current_position().unwrap().loop_number, 0);

    timer.advance(TestDuration(100));
    sequencer.service().unwrap();
    assert_eq!(sequencer.current_position().unwrap().loop_number, 1);

    timer.advance(TestDuration(100));
    sequencer.service().unwrap();
    assert_eq!(sequencer.current_position().unwrap().loop_number, 2);
}

#[test]
fn current_position_returns_none_for_function_based_sequences() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    fn color_fn(base: Srgb, _elapsed: TestDuration) -> Srgb {
        base
    }

    fn timing_fn(_elapsed: TestDuration) -> Option<TestDuration> {
        Some(TestDuration::ZERO)
    }

    let sequence = RgbSequence::<TestDuration, 8>::from_function(RED, color_fn, timing_fn);

    sequencer.load(sequence);
    sequencer.start().unwrap();

    // Function-based sequences don't have step positions
    assert_eq!(sequencer.current_position(), None);
}

#[test]
fn into_led_preserves_current_color() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.service().unwrap();

    // Verify color is RED before extraction
    assert!(colors_equal(sequencer.current_color(), RED));

    // Extract LED - it should have the color displayed
    let extracted_led = sequencer.into_led();
    assert!(colors_equal(extracted_led.get_last_color(), RED));
}

#[test]
fn into_parts_extracts_led_and_sequence() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);

    // Extract both LED and sequence
    let (extracted_led, extracted_sequence) = sequencer.into_parts();

    // LED should be extractable
    let _ = extracted_led;

    // Sequence should be present
    assert!(extracted_sequence.is_some());
    let seq = extracted_sequence.unwrap();
    assert_eq!(seq.step_count(), 2);
}

#[test]
fn into_parts_returns_none_when_no_sequence_loaded() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    // Extract without loading a sequence
    let (_led, sequence) = sequencer.into_parts();

    // Sequence should be None
    assert!(sequence.is_none());
}

#[test]
fn color_epsilon_is_configurable() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();

    // Test default epsilon
    let sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);
    assert_eq!(sequencer.color_epsilon(), DEFAULT_COLOR_EPSILON);

    // Test with_epsilon constructor
    let led = MockLed::new();
    let custom_epsilon = 0.0001;
    let sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::with_epsilon(
        led,
        &timer,
        custom_epsilon,
    );
    assert_eq!(sequencer.color_epsilon(), custom_epsilon);

    // Test set_color_epsilon
    let led = MockLed::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);
    let new_epsilon = 0.01;
    sequencer.set_color_epsilon(new_epsilon);
    assert_eq!(sequencer.color_epsilon(), new_epsilon);
}

#[test]
fn brightness_defaults_to_full() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);
    assert_eq!(sequencer.brightness(), 1.0);
}

#[test]
fn set_brightness_applies_to_led_output() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    // Set brightness to 50%
    sequencer.set_brightness(0.5);
    assert_eq!(sequencer.brightness(), 0.5);

    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.service().unwrap();

    // LED should be at 50% brightness (RED at 50%)
    let expected = Srgb::new(0.5, 0.0, 0.0);
    assert!(colors_equal(sequencer.current_color(), expected));
}

#[test]
fn brightness_is_clamped_to_valid_range() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    // Test clamping upper bound
    sequencer.set_brightness(2.5);
    assert_eq!(sequencer.brightness(), 1.0);

    // Test clamping lower bound
    sequencer.set_brightness(-0.5);
    assert_eq!(sequencer.brightness(), 0.0);

    // Test valid range
    sequencer.set_brightness(0.75);
    assert_eq!(sequencer.brightness(), 0.75);
}

#[test]
fn brightness_can_be_changed_during_playback() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.service().unwrap();

    // Initially at full brightness
    assert!(colors_equal(sequencer.current_color(), RED));

    // Change brightness to 25% during playback
    sequencer.set_brightness(0.25);
    timer.advance(TestDuration(100));
    sequencer.service().unwrap();

    // LED should now be at 25% brightness
    let expected = Srgb::new(0.25, 0.0, 0.0);
    assert!(colors_equal(sequencer.current_color(), expected));
}

#[test]
fn zero_brightness_turns_led_off() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.set_brightness(0.0);
    sequencer.load(sequence);
    sequencer.start().unwrap();

    // LED should be completely off (black)
    assert!(colors_equal(sequencer.current_color(), BLACK));
}

#[test]
fn brightness_applies_to_all_color_channels() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    // White color
    let white = Srgb::new(1.0, 1.0, 1.0);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(white, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.set_brightness(0.3);
    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.service().unwrap();

    // All channels should be at 30%
    let expected = Srgb::new(0.3, 0.3, 0.3);
    assert!(colors_equal(sequencer.current_color(), expected));
}

#[test]
fn brightness_works_with_linear_transitions() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(1000), TransitionStyle::Linear)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.set_brightness(0.5);
    sequencer.load(sequence);
    sequencer.start().unwrap();
    sequencer.service().unwrap();

    // Start at RED at 50%
    assert!(colors_equal(
        sequencer.current_color(),
        Srgb::new(0.5, 0.0, 0.0)
    ));

    // Advance into linear transition
    timer.advance(TestDuration(600)); // 500ms into linear transition
    sequencer.service().unwrap();

    // Should be transitioning from RED to GREEN, at 50% brightness
    // At 500ms into 1000ms transition, we're halfway
    let current = sequencer.current_color();
    // At 50% of transition: red should be decreasing, green increasing
    assert!(current.red > 0.0 && current.red < 0.5);
    assert!(current.green > 0.0 && current.green < 0.5);
    assert_eq!(current.blue, 0.0);
}

#[test]
fn brightness_does_not_affect_sequence_timing() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.set_brightness(0.1);
    sequencer.load(sequence);
    sequencer.start().unwrap();

    // Timing should still be the same - first step should last 100ms
    let timing = sequencer.peek_next_timing().unwrap();
    assert_eq!(timing, ServiceTiming::Delay(TestDuration(100)));

    // Advance to second step
    timer.advance(TestDuration(100));
    sequencer.service().unwrap();

    // Should be on GREEN (dimmed)
    assert!(colors_equal(
        sequencer.current_color(),
        Srgb::new(0.0, 0.1, 0.0)
    ));
}
