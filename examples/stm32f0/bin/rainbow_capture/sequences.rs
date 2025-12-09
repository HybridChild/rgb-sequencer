use palette::{FromColor, Hsv, Srgb};
use rgb_sequencer::{LoopCount, RgbSequence8, TransitionStyle};
use stm32f0::time_source::HalDuration;

/// Create a rainbow sequence that cycles through the full color spectrum
///
/// The sequence smoothly transitions through red -> green -> blue over 12 seconds,
/// then loops infinitely.
pub fn create_rainbow_sequence() -> RgbSequence8<HalDuration> {
    RgbSequence8::builder()
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
