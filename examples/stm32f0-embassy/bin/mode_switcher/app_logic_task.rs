use defmt::info;
use embassy_stm32::gpio::Output;
use embassy_time::Duration;
use palette::{Srgb, FromColor, Hsv};
use rgb_sequencer::{RgbSequence, TransitionStyle, LoopCount};

use crate::types::{Mode, RgbCommand, BUTTON_SIGNAL, RGB_COMMAND_CHANNEL, EmbassyDuration, SEQUENCE_STEP_SIZE};

/// Create a breathing white sequence
fn create_breathing_sequence() -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_SIZE> {
    let white = Srgb::new(1.0, 1.0, 1.0);
    let dim_white = Srgb::new(0.1, 0.1, 0.1);
    
    RgbSequence::new()
        .step(dim_white, EmbassyDuration(Duration::from_millis(2000)), TransitionStyle::Linear)
        .step(white, EmbassyDuration(Duration::from_millis(2000)), TransitionStyle::Linear)
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

/// Create a rainbow cycle sequence
fn create_rainbow_sequence() -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_SIZE> {
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
fn create_police_sequence() -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_SIZE> {
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
fn get_sequence_for_mode(mode: Mode) -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_SIZE> {
    match mode {
        Mode::Breathing => create_breathing_sequence(),
        Mode::Rainbow => create_rainbow_sequence(),
        Mode::Police => create_police_sequence(),
    }
}

/// Update the onboard LED to indicate the current mode
fn update_mode_indicator(led: &mut Output<'static>, mode: Mode) {
    match mode {
        Mode::Breathing => {
            // Mode 1: LED off
            led.set_low();
        }
        Mode::Rainbow => {
            // Mode 2: LED on
            led.set_high();
        }
        Mode::Police => {
            // Mode 3: LED on (could blink in future)
            led.set_high();
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
        .send(RgbCommand::LoadCoordinated(initial_sequence))
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
            .send(RgbCommand::LoadCoordinated(new_sequence))
            .await;
        
        info!("New sequence sent to RGB task");
    }
}