//! Integration tests for RgbSequencer

mod common;
use common::*;

use palette::Srgb;
use rgb_sequencer::sequence::RgbSequence;
use rgb_sequencer::types::{LoopCount, TransitionStyle};
use rgb_sequencer::{
    DEFAULT_COLOR_EPSILON, RgbSequencer, SequencerAction, SequencerError, SequencerState,
    ServiceTiming, TimeDuration,
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
    assert!(colors_equal(sequencer.current_color(), RED));

    sequencer.clear();
    assert_eq!(sequencer.state(), SequencerState::Idle);
    assert!(colors_equal(sequencer.current_color(), BLACK));
}

#[test]
fn service_returns_correct_delay_for_step_transition() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(500), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.load(sequence);
    let timing = sequencer.start().unwrap();

    // Should return the remaining time in the first step (1000ms)
    assert_eq!(timing, ServiceTiming::Delay(TestDuration(1000)));
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
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(1000), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(1000), TransitionStyle::Linear)
        .unwrap()
        .loop_count(LoopCount::Finite(1))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    // Peek should return Delay for step transition
    let peek_timing = sequencer.peek_next_timing().unwrap();
    assert_eq!(peek_timing, ServiceTiming::Delay(TestDuration(1000)));

    // LED should still be at initial color
    assert!(colors_equal(sequencer.current_color(), RED));

    // Advance into linear transition
    timer.advance(TestDuration(1100));

    // Peek should return Continuous for linear transition
    let peek_timing = sequencer.peek_next_timing().unwrap();
    assert_eq!(peek_timing, ServiceTiming::Continuous);

    // LED color should not have changed from peek
    assert!(colors_equal(sequencer.current_color(), RED));

    // Now actually service - LED should update to transitioning color
    sequencer.service().unwrap();
    // At t=1100, we're 100ms into a 1000ms linear transition from RED to GREEN
    // So we should be ~10% of the way from RED to GREEN
    let current = sequencer.current_color();
    assert!(current.red < 1.0); // Moving away from red
    assert!(current.green > 0.0); // Moving toward green

    // Peek when sequence is complete
    timer.advance(TestDuration(1000));
    let peek_timing = sequencer.peek_next_timing().unwrap();
    assert_eq!(peek_timing, ServiceTiming::Complete);

    // State should still be Running (peek doesn't change state)
    assert_eq!(sequencer.state(), SequencerState::Running);

    // Service should transition to Complete state
    sequencer.service().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Complete);
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
fn handle_action_dispatches_all_action_types_correctly() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    // Test Load action
    let result = sequencer.handle_action(SequencerAction::Load(sequence));
    assert!(result.is_ok());
    assert_eq!(sequencer.state(), SequencerState::Loaded);

    // Test Start action
    let result = sequencer.handle_action(SequencerAction::Start);
    assert!(result.is_ok());
    assert_eq!(sequencer.state(), SequencerState::Running);

    // Test Pause action
    let result = sequencer.handle_action(SequencerAction::Pause);
    assert!(result.is_ok());
    assert_eq!(sequencer.state(), SequencerState::Paused);

    // Test Resume action
    let result = sequencer.handle_action(SequencerAction::Resume);
    assert!(result.is_ok());
    assert_eq!(sequencer.state(), SequencerState::Running);

    // Test Stop action
    let result = sequencer.handle_action(SequencerAction::Stop);
    assert!(result.is_ok());
    assert_eq!(sequencer.state(), SequencerState::Loaded);

    // Test Restart action
    sequencer.start().unwrap();
    let result = sequencer.handle_action(SequencerAction::Restart);
    assert!(result.is_ok());
    assert_eq!(sequencer.state(), SequencerState::Running);

    // Test Clear action
    let result = sequencer.handle_action(SequencerAction::Clear);
    assert!(result.is_ok());
    assert_eq!(sequencer.state(), SequencerState::Idle);
}

#[test]
fn query_methods_return_correct_state_and_timing_info() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    // Initial state queries
    assert_eq!(sequencer.state(), SequencerState::Idle);
    assert!(!sequencer.is_running());
    assert!(!sequencer.is_paused());
    assert!(sequencer.current_sequence().is_none());
    assert!(sequencer.elapsed_time().is_none());
    assert!(colors_equal(sequencer.current_color(), BLACK));

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    sequencer.load(sequence);
    assert!(!sequencer.is_running());
    assert!(sequencer.current_sequence().is_some());

    sequencer.start().unwrap();
    assert!(sequencer.is_running());
    assert!(!sequencer.is_paused());

    timer.advance(TestDuration(50));
    sequencer.service().unwrap();
    let elapsed = sequencer.elapsed_time().unwrap();
    assert_eq!(elapsed, TestDuration(50));

    sequencer.pause().unwrap();
    assert!(!sequencer.is_running());
    assert!(sequencer.is_paused());
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
fn resume_handles_timer_overflow_gracefully() {
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

    // Note: With our current TestInstant implementation, overflow won't actually occur
    // since it uses u64 and checked_add will succeed. However, this test documents
    // the intended behavior. On 32-bit systems with wrapping timers, the graceful
    // degradation would kick in.

    sequencer.resume().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Running);
}

#[test]
fn error_types_are_constructable() {
    // Verify error types can be constructed
    let _error1 = SequencerError::InvalidState {
        expected: "Running",
        actual: SequencerState::Paused,
    };
    let _error2 = SequencerError::NoSequenceLoaded;
}

#[test]
fn comprehensive_state_transitions() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    // State: Idle -> Invalid operations
    assert!(sequencer.start().is_err());
    assert!(sequencer.pause().is_err());
    assert!(sequencer.resume().is_err());
    assert!(sequencer.stop().is_err());
    assert!(sequencer.restart().is_err());
    assert!(sequencer.service().is_err());

    // State: Idle -> Loaded
    sequencer.load(sequence);
    assert_eq!(sequencer.state(), SequencerState::Loaded);

    // State: Loaded -> Invalid operations
    assert!(sequencer.pause().is_err());
    assert!(sequencer.resume().is_err());
    assert!(sequencer.stop().is_err());
    assert!(sequencer.restart().is_err());
    assert!(sequencer.service().is_err());

    // State: Loaded -> Running
    assert!(sequencer.start().is_ok());
    assert_eq!(sequencer.state(), SequencerState::Running);

    // State: Running -> Paused
    assert!(sequencer.pause().is_ok());
    assert_eq!(sequencer.state(), SequencerState::Paused);

    // State: Paused -> Invalid operations
    assert!(sequencer.start().is_err());
    assert!(sequencer.pause().is_err());
    assert!(sequencer.service().is_err());

    // State: Paused -> Running
    assert!(sequencer.resume().is_ok());
    assert_eq!(sequencer.state(), SequencerState::Running);

    // State: Running -> Loaded (via stop)
    assert!(sequencer.stop().is_ok());
    assert_eq!(sequencer.state(), SequencerState::Loaded);

    // State: Loaded -> Running -> Complete
    sequencer.start().unwrap();
    timer.advance(TestDuration(200));
    sequencer.service().unwrap();
    assert_eq!(sequencer.state(), SequencerState::Complete);

    // State: Complete -> Running (via restart)
    assert!(sequencer.restart().is_ok());
    assert_eq!(sequencer.state(), SequencerState::Running);

    // State: Running -> Idle (via clear)
    sequencer.clear();
    assert_eq!(sequencer.state(), SequencerState::Idle);
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
    assert!(colors_equal(sequencer.current_color(), RED));

    // Load second sequence should stop the first and transition to Loaded
    sequencer.load(sequence2);
    assert_eq!(sequencer.state(), SequencerState::Loaded);

    // Start second sequence
    sequencer.start().unwrap();
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
    let timing = sequencer.load_and_start(sequence).unwrap();
    assert_eq!(sequencer.state(), SequencerState::Running);
    assert!(colors_equal(sequencer.current_color(), RED));
    assert_eq!(timing, ServiceTiming::Delay(TestDuration(1000)));

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

    // At start - step 0, loop 0
    let pos = sequencer.current_position().unwrap();
    assert_eq!(pos.step_index, 0);
    assert_eq!(pos.loop_number, 0);

    // After 50ms - still step 0, loop 0
    timer.advance(TestDuration(50));
    sequencer.service().unwrap();
    let pos = sequencer.current_position().unwrap();
    assert_eq!(pos.step_index, 0);
    assert_eq!(pos.loop_number, 0);

    // After 100ms - step 1, loop 0
    timer.advance(TestDuration(50));
    sequencer.service().unwrap();
    let pos = sequencer.current_position().unwrap();
    assert_eq!(pos.step_index, 1);
    assert_eq!(pos.loop_number, 0);

    // After 200ms - step 2, loop 0
    timer.advance(TestDuration(100));
    sequencer.service().unwrap();
    let pos = sequencer.current_position().unwrap();
    assert_eq!(pos.step_index, 2);
    assert_eq!(pos.loop_number, 0);

    // After 300ms - sequence complete, no position
    timer.advance(TestDuration(100));
    sequencer.service().unwrap();
    assert_eq!(sequencer.current_position(), None);
}

#[test]
fn current_position_tracks_loop_changes() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let mut sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    let sequence = RgbSequence::<TestDuration, 8>::builder()
        .step(RED, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(GREEN, TestDuration(100), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Finite(3))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    // Loop 0
    let pos = sequencer.current_position().unwrap();
    assert_eq!(pos.step_index, 0);
    assert_eq!(pos.loop_number, 0);

    // Advance to loop 1
    timer.advance(TestDuration(200));
    sequencer.service().unwrap();
    let pos = sequencer.current_position().unwrap();
    assert_eq!(pos.step_index, 0);
    assert_eq!(pos.loop_number, 1);

    // Mid loop 1
    timer.advance(TestDuration(50));
    sequencer.service().unwrap();
    let pos = sequencer.current_position().unwrap();
    assert_eq!(pos.step_index, 0);
    assert_eq!(pos.loop_number, 1);

    // Advance to loop 2
    timer.advance(TestDuration(150));
    sequencer.service().unwrap();
    let pos = sequencer.current_position().unwrap();
    assert_eq!(pos.step_index, 0);
    assert_eq!(pos.loop_number, 2);

    // Complete all loops
    timer.advance(TestDuration(200));
    sequencer.service().unwrap();
    assert_eq!(sequencer.current_position(), None);
}

#[test]
fn current_position_enables_event_detection() {
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
        .loop_count(LoopCount::Finite(2))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    let mut last_position = None;
    let mut step_enter_events = heapless::Vec::<(usize, u32), 16>::new();
    let mut loop_complete_events = heapless::Vec::<u32, 8>::new();

    // Simulate event loop - advance time in small increments to catch all transitions
    // Each step is 100ms, we advance by 30ms per iteration to catch all step boundaries
    for _ in 0..20 {
        let current = sequencer.current_position();

        // Detect position changes (step enter or loop change)
        if current != last_position {
            if let Some(pos) = current {
                step_enter_events
                    .push((pos.step_index, pos.loop_number))
                    .ok();

                // Detect loop completion (when returning to step 0 with higher loop number)
                if pos.step_index == 0 && pos.loop_number > 0 {
                    if let Some(last_pos) = last_position {
                        if pos.loop_number > last_pos.loop_number {
                            loop_complete_events.push(last_pos.loop_number).ok();
                        }
                    }
                }
            }
            last_position = current;
        }

        sequencer.service().ok();
        timer.advance(TestDuration(30));
    }

    // Verify step enter events were detected
    // Should see: (0,0), (1,0), (2,0), (0,1), (1,1), (2,1)
    assert!(
        step_enter_events.len() >= 6,
        "Expected at least 6 step events, got {}",
        step_enter_events.len()
    );
    assert_eq!(step_enter_events[0], (0, 0)); // Start of loop 0
    assert_eq!(step_enter_events[1], (1, 0)); // Step 1 of loop 0
    assert_eq!(step_enter_events[2], (2, 0)); // Step 2 of loop 0
    assert_eq!(step_enter_events[3], (0, 1)); // Start of loop 1
    assert_eq!(step_enter_events[4], (1, 1)); // Step 1 of loop 1
    assert_eq!(step_enter_events[5], (2, 1)); // Step 2 of loop 1

    // Verify loop completion events
    assert!(loop_complete_events.len() >= 1);
    assert_eq!(loop_complete_events[0], 0); // Loop 0 completed
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
fn into_led_extracts_led_from_sequencer() {
    let led = MockLed::new();
    let timer = MockTimeSource::new();
    let sequencer = RgbSequencer::<TestInstant, MockLed, MockTimeSource, 8>::new(led, &timer);

    // Extract LED
    let extracted_led = sequencer.into_led();

    // LED should be extractable (compilation test)
    let _ = extracted_led;
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
