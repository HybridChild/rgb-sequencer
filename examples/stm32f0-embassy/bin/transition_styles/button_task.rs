use embassy_stm32::exti::ExtiInput;
use embassy_time::{Duration, Timer};

use crate::types::BUTTON_SIGNAL;

#[embassy_executor::task]
pub async fn button_task(mut button: ExtiInput<'static>) {
    loop {
        button.wait_for_falling_edge().await;
        BUTTON_SIGNAL.signal(());
        Timer::after(Duration::from_millis(200)).await;
    }
}
