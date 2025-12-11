use palette::Srgb;
use rgb_sequencer::{
    BLACK, BLUE, GREEN, LoopCount, RED, RgbSequence, TimeDuration, TransitionStyle, WHITE,
};
use stm32f0::time_source::HalDuration;

/// Sine-based breathing effect function
///
/// Modulates the brightness of a base color using a sine wave to create
/// a smooth breathing effect. The brightness oscillates between 10% and 100%
/// over a 4-second cycle (2 seconds fade up, 2 seconds fade down).
///
/// # Arguments
/// * `base_color` - The color to modulate (typically white)
/// * `elapsed` - Time elapsed since sequence started
///
/// # Returns
/// The color with modulated brightness based on sine wave position
fn breathing_sine_wave(base_color: Srgb, elapsed: HalDuration) -> Srgb {
    // Breathing cycle period in milliseconds (4 seconds total)
    const PERIOD_MS: u64 = 4000;

    // Calculate position within the current breathing cycle (0.0 to 1.0)
    let elapsed_ms = elapsed.as_millis();
    let time_in_cycle = (elapsed_ms % PERIOD_MS) as f32 / PERIOD_MS as f32;

    // Convert to angle in radians (0 to 2π)
    let angle = time_in_cycle * 2.0 * core::f32::consts::PI;

    // Calculate brightness using sine wave
    // sin(angle) ranges from -1 to 1
    // We transform it to range from 0.1 (dim) to 1.0 (bright)
    let sine_value = libm::sinf(angle);
    let brightness = 0.1 + 0.45 * (1.0 + sine_value);

    // Apply brightness to the base color
    Srgb::new(
        base_color.red * brightness,
        base_color.green * brightness,
        base_color.blue * brightness,
    )
}

/// Timing function for continuous animation
///
/// Returns Some(Duration::ZERO) to indicate that the animation should be
/// serviced continuously (every frame) for smooth transitions. Never returns
/// None since this is an infinite animation.
fn continuous_timing(_elapsed: HalDuration) -> Option<HalDuration> {
    Some(HalDuration(0))
}

/// Create a breathing white sequence using function-based animation
///
/// Uses a sine wave to create a smooth breathing effect, demonstrating
/// the function-based sequence feature. The brightness oscillates between
/// 10% and 100% over a 4-second cycle.
///
/// This is an alternative to the step-based breathing sequence, showing
/// how the same visual effect can be achieved with algorithmic animation.
pub fn create_breathing_sequence() -> RgbSequence<HalDuration, 16> {
    RgbSequence::from_function(WHITE, breathing_sine_wave, continuous_timing)
}

/// Create a breathing white sequence using step-based animation (original)
///
/// Smoothly fades between dim white and bright white over 4 seconds,
/// creating a gentle breathing effect. Loops infinitely.
///
/// This is the original step-based implementation, kept for comparison.
#[allow(dead_code)]
pub fn create_breathing_sequence_step_based() -> RgbSequence<HalDuration, 16> {
    let dim_white = Srgb::new(0.1, 0.1, 0.1);

    RgbSequence::builder()
        .step(dim_white, HalDuration(2000), TransitionStyle::Linear)
        .unwrap()
        .step(WHITE, HalDuration(2000), TransitionStyle::Linear)
        .unwrap()
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

/// Create a rainbow cycle sequence
///
/// Smoothly transitions through red -> green -> blue over 12 seconds,
/// creating a full spectrum color cycle. Loops infinitely.
pub fn create_rainbow_sequence() -> RgbSequence<HalDuration, 16> {
    RgbSequence::builder()
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

/// Create a police lights sequence
///
/// Alternates between red flashes and blue flashes with off periods,
/// creating a police siren effect. Loops infinitely.
pub fn create_police_sequence() -> RgbSequence<HalDuration, 16> {
    RgbSequence::builder()
        .step(RED, HalDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(BLACK, HalDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(RED, HalDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(BLACK, HalDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(BLUE, HalDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(BLACK, HalDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(BLUE, HalDuration(100), TransitionStyle::Step)
        .unwrap()
        .step(BLACK, HalDuration(100), TransitionStyle::Step)
        .unwrap()
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

/// Flickering flame effect function
///
/// Simulates a flickering flame by combining multiple sine waves at different
/// frequencies to create pseudo-random variations in brightness and color temperature.
/// The flame color shifts between deep orange and bright yellow-orange while the
/// brightness flickers irregularly.
///
/// # Arguments
/// * `base_color` - The base color to modulate (typically orange)
/// * `elapsed` - Time elapsed since sequence started
///
/// # Returns
/// The color with modulated brightness and temperature to simulate flame flicker
fn flame_flicker(base_color: Srgb, elapsed: HalDuration) -> Srgb {
    let elapsed_ms = elapsed.as_millis();

    // Use multiple sine waves at different frequencies to create pseudo-random flicker
    // Main flicker: fast, irregular brightness changes (~4 Hz)
    let fast_angle = (elapsed_ms as f32 * 0.004) * 2.0 * core::f32::consts::PI; // ~4 Hz
    let fast_flicker = libm::sinf(fast_angle);

    // Medium flicker: adds complexity (~1.5 Hz)
    let med_angle = (elapsed_ms as f32 * 0.0015) * 2.0 * core::f32::consts::PI; // ~1.5 Hz
    let med_flicker = libm::sinf(med_angle);

    // Slow wave: gentle overall brightness variation (~0.7 Hz)
    let slow_angle = (elapsed_ms as f32 * 0.0007) * 2.0 * core::f32::consts::PI; // ~0.7 Hz
    let slow_wave = libm::sinf(slow_angle);

    // Color temperature variation (flame shifts between deep orange and bright yellow)
    let color_angle = (elapsed_ms as f32 * 0.001) * 2.0 * core::f32::consts::PI; // ~1 Hz
    let color_shift = libm::sinf(color_angle);

    // Combine flickers with different weights
    // Fast flicker dominates (50%), medium adds complexity (30%), slow adds drift (20%)
    let combined_flicker = 0.5 * fast_flicker + 0.3 * med_flicker + 0.2 * slow_wave;

    // Map to brightness range: 0.5 (50%) to 1.0 (100%)
    // Flames are never completely dim, but flicker noticeably
    let brightness = 0.5 + 0.25 * (1.0 + combined_flicker);

    // Color temperature: shift between deep orange (more red) and bright yellow-orange
    // Positive color_shift = more yellow (hotter), negative = more red (cooler flame)
    let red_component = base_color.red * brightness;
    let green_component = base_color.green * brightness * (1.0 + 0.15 * color_shift);
    let blue_component = base_color.blue * brightness * (1.0 + 0.3 * color_shift);

    Srgb::new(
        red_component.min(1.0),
        green_component.min(1.0),
        blue_component.min(1.0),
    )
}

/// Create a flickering flame sequence using function-based animation
///
/// Simulates a realistic candle or torch flame with irregular flickering.
/// The effect combines multiple sine waves at different frequencies to create
/// complex, pseudo-random brightness and color temperature variations.
/// The flame stays within orange/yellow tones and never goes completely dark.
pub fn create_flame_sequence() -> RgbSequence<HalDuration, 16> {
    // Base flame color: warm orange
    let flame_orange = Srgb::new(1.0, 0.4, 0.0);

    RgbSequence::from_function(flame_orange, flame_flicker, continuous_timing)
}
