use defmt::info;

use crate::blink_task::BLINK_COUNT_SIGNAL;
use crate::types::{BrightnessLevel, RgbCommand, BUTTON_SIGNAL, RGB_COMMAND_CHANNEL};

/// Get the blink count for the current brightness level
/// - Full (100%): 0 blinks (solid ON)
/// - High (75%): 1 blink
/// - Medium (50%): 2 blinks
/// - Low (25%): 3 blinks
/// - Dim (10%): 4 blinks
fn get_blink_count(level: BrightnessLevel) -> u8 {
    match level {
        BrightnessLevel::Full => 0,
        BrightnessLevel::High => 1,
        BrightnessLevel::Medium => 2,
        BrightnessLevel::Low => 3,
        BrightnessLevel::Dim => 4,
    }
}

#[embassy_executor::task]
pub async fn app_logic_task() {
    info!("Starting app logic task...");

    let mut current_brightness = BrightnessLevel::Full;

    // Set initial brightness
    info!("Setting initial brightness: {:?}", current_brightness);
    RGB_COMMAND_CHANNEL
        .send(RgbCommand::SetBrightness(current_brightness))
        .await;

    // Set initial blink pattern
    BLINK_COUNT_SIGNAL.signal(get_blink_count(current_brightness));

    loop {
        // Wait for button press signal
        BUTTON_SIGNAL.wait().await;
        info!("Button press received, cycling brightness...");

        // Cycle to next brightness level
        current_brightness = current_brightness.next();
        info!(
            "New brightness: {:?} ({}%)",
            current_brightness,
            (current_brightness.value() * 100.0) as u8
        );

        // Update blink pattern
        BLINK_COUNT_SIGNAL.signal(get_blink_count(current_brightness));

        // Send brightness command to RGB task
        RGB_COMMAND_CHANNEL
            .send(RgbCommand::SetBrightness(current_brightness))
            .await;

        info!("Brightness command sent to RGB task");
    }
}
