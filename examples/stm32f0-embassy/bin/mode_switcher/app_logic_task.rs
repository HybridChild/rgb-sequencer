use defmt::info;
use embassy_time::Duration;
use palette::Srgb;
use rgb_sequencer::{
    BLACK, BLUE, GREEN, LoopCount, RED, RgbSequence8, SequencerAction8, SequencerCommand8,
    TimeDuration, TransitionStyle, WHITE,
};

use crate::blink_task::BLINK_COUNT_SIGNAL;
use crate::types::{BUTTON_SIGNAL, EmbassyDuration, Mode, RGB_COMMAND_CHANNEL};

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
fn breathing_sine_wave(base_color: Srgb, elapsed: EmbassyDuration) -> Srgb {
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
fn continuous_timing(_elapsed: EmbassyDuration) -> Option<EmbassyDuration> {
    Some(EmbassyDuration(Duration::from_ticks(0)))
}

/// Create a breathing white sequence using function-based animation
///
/// Uses a sine wave to create a smooth breathing effect, demonstrating
/// the function-based sequence feature. The brightness oscillates between
/// 10% and 100% over a 4-second cycle.
fn create_breathing_sequence() -> RgbSequence8<EmbassyDuration> {
    RgbSequence8::from_function(WHITE, breathing_sine_wave, continuous_timing)
}

/// Create a rainbow cycle sequence
fn create_rainbow_sequence() -> RgbSequence8<EmbassyDuration> {
    RgbSequence8::builder()
        .step(
            RED,
            EmbassyDuration(Duration::from_millis(4000)),
            TransitionStyle::Linear,
        )
        .unwrap()
        .step(
            GREEN,
            EmbassyDuration(Duration::from_millis(4000)),
            TransitionStyle::Linear,
        )
        .unwrap()
        .step(
            BLUE,
            EmbassyDuration(Duration::from_millis(4000)),
            TransitionStyle::Linear,
        )
        .unwrap()
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

/// Create a police lights sequence
fn create_police_sequence() -> RgbSequence8<EmbassyDuration> {
    RgbSequence8::builder()
        .step(
            RED,
            EmbassyDuration(Duration::from_millis(100)),
            TransitionStyle::Step,
        )
        .unwrap()
        .step(
            BLACK,
            EmbassyDuration(Duration::from_millis(100)),
            TransitionStyle::Step,
        )
        .unwrap()
        .step(
            RED,
            EmbassyDuration(Duration::from_millis(100)),
            TransitionStyle::Step,
        )
        .unwrap()
        .step(
            BLACK,
            EmbassyDuration(Duration::from_millis(100)),
            TransitionStyle::Step,
        )
        .unwrap()
        .step(
            BLUE,
            EmbassyDuration(Duration::from_millis(100)),
            TransitionStyle::Step,
        )
        .unwrap()
        .step(
            BLACK,
            EmbassyDuration(Duration::from_millis(100)),
            TransitionStyle::Step,
        )
        .unwrap()
        .step(
            BLUE,
            EmbassyDuration(Duration::from_millis(100)),
            TransitionStyle::Step,
        )
        .unwrap()
        .step(
            BLACK,
            EmbassyDuration(Duration::from_millis(100)),
            TransitionStyle::Step,
        )
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
fn flame_flicker(base_color: Srgb, elapsed: EmbassyDuration) -> Srgb {
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
fn create_flame_sequence() -> RgbSequence8<EmbassyDuration> {
    // Base flame color: warm orange
    let flame_orange = Srgb::new(1.0, 0.4, 0.0);

    RgbSequence8::from_function(flame_orange, flame_flicker, continuous_timing)
}

/// Get the sequence for a given mode
fn get_sequence_for_mode(mode: Mode) -> RgbSequence8<EmbassyDuration> {
    match mode {
        Mode::Rainbow => create_rainbow_sequence(),
        Mode::Police => create_police_sequence(),
        Mode::Breathing => create_breathing_sequence(),
        Mode::Flame => create_flame_sequence(),
    }
}

/// Get the blink count for the current mode
/// - Rainbow: 1 blink
/// - Police: 2 blinks
/// - Flame: 3 blinks
/// - Breathing: 4 blinks
fn get_blink_count(mode: Mode) -> u8 {
    match mode {
        Mode::Rainbow => 1,
        Mode::Police => 2,
        Mode::Flame => 3,
        Mode::Breathing => 4,
    }
}

#[embassy_executor::task]
pub async fn app_logic_task() {
    info!("Starting app logic task...");

    let mut current_mode = Mode::Rainbow;

    // Load initial sequence using library's SequencerCommand
    info!("Loading initial mode: {:?}", current_mode);
    let initial_sequence = get_sequence_for_mode(current_mode);
    RGB_COMMAND_CHANNEL
        .send(SequencerCommand8::new(
            (), // Unit LED ID since we only have one LED
            SequencerAction8::Load(initial_sequence),
        ))
        .await;

    // Set initial blink pattern
    BLINK_COUNT_SIGNAL.signal(get_blink_count(current_mode));

    loop {
        // Wait for button press signal
        BUTTON_SIGNAL.wait().await;
        info!("Button press received, cycling mode...");

        // Cycle to next mode
        current_mode = current_mode.next();
        info!("New mode: {:?}", current_mode);

        // Update blink pattern
        BLINK_COUNT_SIGNAL.signal(get_blink_count(current_mode));

        // Create and send new sequence using library's SequencerCommand
        let new_sequence = get_sequence_for_mode(current_mode);
        RGB_COMMAND_CHANNEL
            .send(SequencerCommand8::new(
                (),
                SequencerAction8::Load(new_sequence),
            ))
            .await;

        info!("New sequence sent to RGB task");
    }
}
