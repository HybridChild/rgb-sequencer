use defmt::info;
use embassy_stm32::gpio::Output;

use crate::types::{BrightnessLevel, RgbCommand, BUTTON_SIGNAL, RGB_COMMAND_CHANNEL};

/// Update the onboard LED to indicate the current brightness level
/// LED blinks in patterns to show brightness:
/// - Full (100%): LED steady on
/// - High (75%): LED off
/// - Medium (50%): LED steady on
/// - Low (25%): LED off
/// - Dim (10%): LED steady on
fn update_brightness_indicator(led: &mut Output<'static>, level: BrightnessLevel) {
    match level {
        BrightnessLevel::Full | BrightnessLevel::Medium | BrightnessLevel::Dim => {
            led.set_high();
        }
        BrightnessLevel::High | BrightnessLevel::Low => {
            led.set_low();
        }
    }
}

#[embassy_executor::task]
pub async fn app_logic_task(mut onboard_led: Output<'static>) {
    info!("Starting app logic task...");

    let mut current_brightness = BrightnessLevel::Full;

    // Set initial brightness
    info!("Setting initial brightness: {:?}", current_brightness);
    RGB_COMMAND_CHANNEL
        .send(RgbCommand::SetBrightness(current_brightness))
        .await;

    update_brightness_indicator(&mut onboard_led, current_brightness);

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

        // Update onboard LED indicator
        update_brightness_indicator(&mut onboard_led, current_brightness);

        // Send brightness command to RGB task
        RGB_COMMAND_CHANNEL
            .send(RgbCommand::SetBrightness(current_brightness))
            .await;

        info!("Brightness command sent to RGB task");
    }
}
