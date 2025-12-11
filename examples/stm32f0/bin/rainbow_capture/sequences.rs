use rgb_sequencer::{BLUE, GREEN, LoopCount, RED, RgbSequence8, TransitionStyle};
use stm32f0::time_source::HalDuration;

/// Create a rainbow sequence that cycles through the full color spectrum
///
/// The sequence smoothly transitions through red -> green -> blue over 12 seconds,
/// then loops infinitely.
pub fn create_rainbow_sequence() -> RgbSequence8<HalDuration> {
    RgbSequence8::builder()
        .step(RED, HalDuration(4000), TransitionStyle::Linear)
        .unwrap()
        .step(GREEN, HalDuration(4000), TransitionStyle::Linear)
        .unwrap()
        .step(BLUE, HalDuration(4000), TransitionStyle::Linear)
        .unwrap()
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}
