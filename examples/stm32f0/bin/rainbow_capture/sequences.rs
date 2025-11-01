use palette::{Srgb, FromColor, Hsv};
use rgb_sequencer::{RgbSequence, TransitionStyle, LoopCount};
use stm32f0_examples::time_source::HalDuration;

/// Maximum number of steps that can be stored in a sequence
pub const SEQUENCE_STEP_CAPACITY: usize = 8;

/// Create a rainbow sequence that cycles through the full color spectrum
/// 
/// The sequence smoothly transitions through red -> green -> blue over 12 seconds,
/// then loops infinitely.
pub fn create_rainbow_sequence() -> RgbSequence<HalDuration, SEQUENCE_STEP_CAPACITY> {    
    RgbSequence::new()
        .step(
            Srgb::from_color(Hsv::new(0.0, 1.0, 1.0)),
            HalDuration(4000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::from_color(Hsv::new(120.0, 1.0, 1.0)),
            HalDuration(4000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::from_color(Hsv::new(240.0, 1.0, 1.0)),
            HalDuration(4000),
            TransitionStyle::Linear,
        )
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}
