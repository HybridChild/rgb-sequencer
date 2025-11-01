use defmt::info;
use embassy_stm32::gpio::Output;
use palette::{Srgb, FromColor, Hsv};
use rgb_sequencer::{RgbSequence, TransitionStyle, LoopCount};

use crate::types::{
    RgbCommand, BUTTON_SIGNAL, RGB_COMMAND_CHANNEL, COLOR_RESPONSE_SIGNAL,
    EmbassyDuration, SEQUENCE_STEP_CAPACITY, LedId,
};

/// Create a rainbow cycle sequence (red -> green -> blue)
fn create_rainbow_sequence() -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_CAPACITY> {
    RgbSequence::new()
        .step(
            Srgb::from_color(Hsv::new(0.0, 1.0, 1.0)),      // Red
            EmbassyDuration(embassy_time::Duration::from_millis(4000)),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::from_color(Hsv::new(120.0, 1.0, 1.0)),    // Green
            EmbassyDuration(embassy_time::Duration::from_millis(4000)),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::from_color(Hsv::new(240.0, 1.0, 1.0)),    // Blue
            EmbassyDuration(embassy_time::Duration::from_millis(4000)),
            TransitionStyle::Linear,
        )
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

/// Create a static color sequence that holds a single color
fn create_static_sequence(color: Srgb) -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_CAPACITY> {
    RgbSequence::new()
        .step(
            color,
            EmbassyDuration(embassy_time::Duration::from_millis(0)),
            TransitionStyle::Step,
        )
        .build()
        .unwrap()
}

/// Create a smooth transition sequence from one color to another
fn create_transition_sequence(
    from: Srgb,
    to: Srgb,
    duration_ms: u64,
) -> RgbSequence<EmbassyDuration, SEQUENCE_STEP_CAPACITY> {
    RgbSequence::new()
        .start_color(from)
        .step(
            to,
            EmbassyDuration(embassy_time::Duration::from_millis(duration_ms)),
            TransitionStyle::Linear,
        )
        .build()
        .unwrap()
}

#[embassy_executor::task]
pub async fn app_logic_task(mut onboard_led: Output<'static>) {
    info!("App logic task started");
    
    // Initial state:
    // - LED 1: Rainbow animation (continuously cycling)
    // - LED 2: Off
    
    let rainbow_sequence = create_rainbow_sequence();
    
    // Load and start rainbow on LED 1
    RGB_COMMAND_CHANNEL
        .send(RgbCommand::Load {
            led_id: LedId::Led1,
            sequence: rainbow_sequence,
        })
        .await;
    
    RGB_COMMAND_CHANNEL
        .send(RgbCommand::Start {
            led_id: LedId::Led1,
        })
        .await;
    
    // Load and start black (off) on LED 2
    let off_sequence = create_static_sequence(Srgb::new(0.0, 0.0, 0.0));
    RGB_COMMAND_CHANNEL
        .send(RgbCommand::Load {
            led_id: LedId::Led2,
            sequence: off_sequence,
        })
        .await;
    
    RGB_COMMAND_CHANNEL
        .send(RgbCommand::Start {
            led_id: LedId::Led2,
        })
        .await;
    
    info!("Initial sequences loaded - LED 1 rainbow, LED 2 off");
    info!("Press button to capture color from LED 1 to LED 2");
    
    // Turn on onboard LED to show we're ready
    onboard_led.set_high();
    
    // Track LED 1 state (running or paused)
    let mut led1_is_paused = false;
    
    loop {
        // Wait for button press
        BUTTON_SIGNAL.wait().await;
        
        if led1_is_paused {
            // Button pressed while LED 1 is paused -> resume LED 1
            info!("Button pressed - resuming LED 1");
            
            RGB_COMMAND_CHANNEL
                .send(RgbCommand::Resume {
                    led_id: LedId::Led1,
                })
                .await;
            
            led1_is_paused = false;
            onboard_led.set_high();
        } else {
            // Button pressed while LED 1 is running -> pause LED 1 and capture color to LED 2
            info!("Button pressed - pausing LED 1 and capturing color");
            
            // Pause LED 1
            RGB_COMMAND_CHANNEL
                .send(RgbCommand::Pause {
                    led_id: LedId::Led1,
                })
                .await;
            
            // Query the current color from LED 1
            RGB_COMMAND_CHANNEL
                .send(RgbCommand::GetColor {
                    led_id: LedId::Led1,
                    response: &COLOR_RESPONSE_SIGNAL,
                })
                .await;
            
            // Wait for the response
            let captured_color = COLOR_RESPONSE_SIGNAL.wait().await;
            info!("Captured color: R={} G={} B={}", 
                  captured_color.red, captured_color.green, captured_color.blue);
            
            // Get LED 2's current color for smooth transition
            RGB_COMMAND_CHANNEL
                .send(RgbCommand::GetColor {
                    led_id: LedId::Led2,
                    response: &COLOR_RESPONSE_SIGNAL,
                })
                .await;
            
            let led2_current = COLOR_RESPONSE_SIGNAL.wait().await;
            
            // Create a smooth transition from LED 2's current color to the captured color
            let transition_sequence = create_transition_sequence(
                led2_current,
                captured_color,
                2000,  // 2 second transition
            );
            
            // Load and start the transition on LED 2
            RGB_COMMAND_CHANNEL
                .send(RgbCommand::Load {
                    led_id: LedId::Led2,
                    sequence: transition_sequence,
                })
                .await;
            
            RGB_COMMAND_CHANNEL
                .send(RgbCommand::Start {
                    led_id: LedId::Led2,
                })
                .await;
            
            info!("LED 2 transitioning to captured color");
            
            led1_is_paused = true;
            onboard_led.set_low();
        }
    }
}
