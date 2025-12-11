use rgb_sequencer::{
    BLACK, BLUE, CYAN, GREEN, LoopCount, MAGENTA, RED, RgbSequence16, TimeDuration,
    TransitionStyle, WHITE, YELLOW,
};
use stm32f0_embassy::time_wrapper::EmbassyDuration;

/// Create a sequence cycling through colors with the specified transition style
pub fn create_transition_sequence(transition: TransitionStyle) -> RgbSequence16<EmbassyDuration> {
    let duration = EmbassyDuration::from_millis(1000);

    RgbSequence16::builder()
        .step(RED, duration, transition)
        .unwrap()
        .step(BLACK, duration, transition)
        .unwrap()
        .step(GREEN, duration, transition)
        .unwrap()
        .step(BLACK, duration, transition)
        .unwrap()
        .step(BLUE, duration, transition)
        .unwrap()
        .step(BLACK, duration, transition)
        .unwrap()
        .step(WHITE, duration, transition)
        .unwrap()
        .step(BLACK, duration, transition)
        .unwrap()
        .step(YELLOW, duration, transition)
        .unwrap()
        .step(BLACK, duration, transition)
        .unwrap()
        .step(CYAN, duration, transition)
        .unwrap()
        .step(BLACK, duration, transition)
        .unwrap()
        .step(MAGENTA, duration, transition)
        .unwrap()
        .step(BLACK, duration, transition)
        .unwrap()
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}
