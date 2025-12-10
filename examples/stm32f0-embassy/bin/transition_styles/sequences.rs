use rgb_sequencer::{
    COLOR_BLUE, COLOR_CYAN, COLOR_GREEN, COLOR_MAGENTA, COLOR_OFF, COLOR_RED, COLOR_WHITE,
    COLOR_YELLOW, LoopCount, RgbSequence16, TimeDuration, TransitionStyle,
};
use stm32f0_embassy::time_wrapper::EmbassyDuration;

/// Create a sequence cycling through colors with the specified transition style
pub fn create_transition_sequence(transition: TransitionStyle) -> RgbSequence16<EmbassyDuration> {
    let duration = EmbassyDuration::from_millis(1000);

    RgbSequence16::builder()
        .step(COLOR_RED, duration, transition)
        .unwrap()
        .step(COLOR_OFF, duration, transition)
        .unwrap()
        .step(COLOR_GREEN, duration, transition)
        .unwrap()
        .step(COLOR_OFF, duration, transition)
        .unwrap()
        .step(COLOR_BLUE, duration, transition)
        .unwrap()
        .step(COLOR_OFF, duration, transition)
        .unwrap()
        .step(COLOR_WHITE, duration, transition)
        .unwrap()
        .step(COLOR_OFF, duration, transition)
        .unwrap()
        .step(COLOR_YELLOW, duration, transition)
        .unwrap()
        .step(COLOR_OFF, duration, transition)
        .unwrap()
        .step(COLOR_CYAN, duration, transition)
        .unwrap()
        .step(COLOR_OFF, duration, transition)
        .unwrap()
        .step(COLOR_MAGENTA, duration, transition)
        .unwrap()
        .step(COLOR_OFF, duration, transition)
        .unwrap()
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}
