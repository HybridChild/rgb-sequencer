use defmt::info;
use embassy_stm32::gpio::Output;
use embassy_time::Duration;
use palette::{Srgb, FromColor, Hsv};
use rgb_sequencer::{RgbSequence, TransitionStyle, LoopCount, TimeDuration};

use crate::types::{Mode, RgbCommand, BUTTON_SIGNAL, RGB_COMMAND_CHANNEL, EmbassyDuration, SEQUENCE_STEP_CAPACITY};

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
fn create_breathing_sequence() -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_CAPACITY> {
    let white = Srgb::new(1.0, 1.0, 1.0);
    
    RgbSequence::from_function(
        white,
        breathing_sine_wave,
        continuous_timing,
    )
}

/// Create a rainbow cycle sequence
fn create_rainbow_sequence() -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_CAPACITY> {
    RgbSequence::new()
        .step(
            Srgb::from_color(Hsv::new(0.0, 1.0, 1.0)),
            EmbassyDuration(Duration::from_millis(4000)),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::from_color(Hsv::new(120.0, 1.0, 1.0)),
            EmbassyDuration(Duration::from_millis(4000)),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::from_color(Hsv::new(240.0, 1.0, 1.0)),
            EmbassyDuration(Duration::from_millis(4000)),
            TransitionStyle::Linear,
        )
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

/// Create a police lights sequence
fn create_police_sequence() -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_CAPACITY> {
    let red = Srgb::new(1.0, 0.0, 0.0);
    let blue = Srgb::new(0.0, 0.0, 1.0);
    let off = Srgb::new(0.0, 0.0, 0.0);
    
    RgbSequence::new()
        .step(red, EmbassyDuration(Duration::from_millis(100)), TransitionStyle::Step)
        .step(off, EmbassyDuration(Duration::from_millis(100)), TransitionStyle::Step)
        .step(red, EmbassyDuration(Duration::from_millis(100)), TransitionStyle::Step)
        .step(off, EmbassyDuration(Duration::from_millis(100)), TransitionStyle::Step)
        .step(blue, EmbassyDuration(Duration::from_millis(100)), TransitionStyle::Step)
        .step(off, EmbassyDuration(Duration::from_millis(100)), TransitionStyle::Step)
        .step(blue, EmbassyDuration(Duration::from_millis(100)), TransitionStyle::Step)
        .step(off, EmbassyDuration(Duration::from_millis(100)), TransitionStyle::Step)
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

/// Get the sequence for a given mode
fn get_sequence_for_mode(mode: Mode) -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_CAPACITY> {
    match mode {
        Mode::Rainbow => create_rainbow_sequence(),
        Mode::Police => create_police_sequence(),
        Mode::Breathing => create_breathing_sequence(),
    }
}

/// Update the onboard LED to indicate the current mode
fn update_mode_indicator(led: &mut Output<'static>, mode: Mode) {
    match mode {
        Mode::Rainbow => {
            // Mode 2: LED on
            led.set_high();
        }
        Mode::Police => {
            // Mode 3: LED on (could blink in future)
            led.set_high();
        }
        Mode::Breathing => {
            // Mode 1: LED off
            led.set_low();
        }
    }
}

#[embassy_executor::task]
pub async fn app_logic_task(mut onboard_led: Output<'static>) {
    info!("Starting app logic task...");
    
    let mut current_mode = Mode::Rainbow;
    
    // Load initial sequence
    info!("Loading initial mode: {:?}", current_mode);
    let initial_sequence = get_sequence_for_mode(current_mode);
    RGB_COMMAND_CHANNEL
        .send(RgbCommand::Load(initial_sequence))
        .await;
    
    update_mode_indicator(&mut onboard_led, current_mode);
    
    loop {
        // Wait for button press signal
        BUTTON_SIGNAL.wait().await;
        info!("Button press received, cycling mode...");
        
        // Cycle to next mode
        current_mode = current_mode.next();
        info!("New mode: {:?}", current_mode);
        
        // Update onboard LED indicator
        update_mode_indicator(&mut onboard_led, current_mode);
        
        // Create and send new sequence
        let new_sequence = get_sequence_for_mode(current_mode);
        RGB_COMMAND_CHANNEL
            .send(RgbCommand::Load(new_sequence))
            .await;
        
        info!("New sequence sent to RGB task");
    }
}
