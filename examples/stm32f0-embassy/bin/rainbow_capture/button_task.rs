use defmt::info;
use embassy_stm32::exti::ExtiInput;
use embassy_time::{Duration, Timer};

use crate::types::BUTTON_SIGNAL;

#[embassy_executor::task]
pub async fn button_task(mut button: ExtiInput<'static>) {
    info!("Button task started - waiting for button presses");
    
    loop {
        // Wait for button press (falling edge on user button)
        button.wait_for_falling_edge().await;
        info!("Button pressed!");
        
        // Signal the app logic task
        BUTTON_SIGNAL.signal(());
        
        // Debounce delay
        Timer::after(Duration::from_millis(200)).await;
    }
}
