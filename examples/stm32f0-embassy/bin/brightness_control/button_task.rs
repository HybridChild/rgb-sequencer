use defmt::info;
use embassy_stm32::exti::ExtiInput;
use embassy_time::Timer;

use crate::types::BUTTON_SIGNAL;

/// Debounce delay in milliseconds
const DEBOUNCE_MS: u64 = 50;

#[embassy_executor::task]
pub async fn button_task(mut button: ExtiInput<'static>) {
    info!("Button task started");

    loop {
        // Wait for button press (button is active low - goes low when pressed)
        button.wait_for_falling_edge().await;

        // Debounce
        Timer::after_millis(DEBOUNCE_MS).await;

        // Signal the app logic task
        BUTTON_SIGNAL.signal(());
        info!("Button press signaled");

        // Wait for button release
        button.wait_for_rising_edge().await;

        // Debounce release
        Timer::after_millis(DEBOUNCE_MS).await;
    }
}
