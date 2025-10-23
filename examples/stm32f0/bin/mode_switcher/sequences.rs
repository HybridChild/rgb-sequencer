use palette::{Srgb, FromColor, Hsv};
use rgb_sequencer::{RgbSequence, TransitionStyle, LoopCount};
use stm32f0_examples::time_source::HalDuration;

/// Create a breathing white sequence
/// 
/// Smoothly fades between dim white and bright white over 4 seconds,
/// creating a gentle breathing effect. Loops infinitely.
pub fn create_breathing_sequence() -> RgbSequence<HalDuration, 16> {
    let white = Srgb::new(1.0, 1.0, 1.0);
    let dim_white = Srgb::new(0.1, 0.1, 0.1);
    
    RgbSequence::new()
        .step(dim_white, HalDuration(2000), TransitionStyle::Linear)
        .step(white, HalDuration(2000), TransitionStyle::Linear)
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

/// Create a rainbow cycle sequence
/// 
/// Smoothly transitions through red -> green -> blue over 12 seconds,
/// creating a full spectrum color cycle. Loops infinitely.
pub fn create_rainbow_sequence() -> RgbSequence<HalDuration, 16> {
    RgbSequence::new()
        .step(
            Srgb::from_color(Hsv::new(0.0, 1.0, 1.0)),      // Red
            HalDuration(4000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::from_color(Hsv::new(120.0, 1.0, 1.0)),    // Green
            HalDuration(4000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::from_color(Hsv::new(240.0, 1.0, 1.0)),    // Blue
            HalDuration(4000),
            TransitionStyle::Linear,
        )
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

/// Create a police lights sequence
/// 
/// Alternates between red flashes and blue flashes with off periods,
/// creating a police siren effect. Loops infinitely.
pub fn create_police_sequence() -> RgbSequence<HalDuration, 16> {
    let red = Srgb::new(1.0, 0.0, 0.0);
    let blue = Srgb::new(0.0, 0.0, 1.0);
    let off = Srgb::new(0.0, 0.0, 0.0);
    
    RgbSequence::new()
        .step(red, HalDuration(100), TransitionStyle::Step)
        .step(off, HalDuration(100), TransitionStyle::Step)
        .step(red, HalDuration(100), TransitionStyle::Step)
        .step(off, HalDuration(100), TransitionStyle::Step)
        .step(blue, HalDuration(100), TransitionStyle::Step)
        .step(off, HalDuration(100), TransitionStyle::Step)
        .step(blue, HalDuration(100), TransitionStyle::Step)
        .step(off, HalDuration(100), TransitionStyle::Step)
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}
