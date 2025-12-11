use embassy_stm32::gpio::Output;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

pub static BLINK_COUNT_SIGNAL: Signal<ThreadModeRawMutex, u8> = Signal::new();

#[embassy_executor::task]
pub async fn blink_task(mut led: Output<'static>) {
    const BLINK_ON_MS: u64 = 150;
    const BLINK_OFF_MS: u64 = 150;
    const CYCLE_PERIOD_MS: u64 = 2000;

    let mut blink_count: u8 = 0;

    loop {
        if blink_count == 0 {
            led.set_high();

            match embassy_time::with_timeout(Duration::from_secs(1), BLINK_COUNT_SIGNAL.wait())
                .await
            {
                Ok(new_count) => blink_count = new_count,
                Err(_) => {}
            }
            continue;
        }

        for _ in 0..blink_count {
            led.set_high();
            Timer::after(Duration::from_millis(BLINK_ON_MS)).await;
            led.set_low();
            Timer::after(Duration::from_millis(BLINK_OFF_MS)).await;
        }

        let blink_time = (BLINK_ON_MS + BLINK_OFF_MS) * blink_count as u64;
        let pause_time = CYCLE_PERIOD_MS.saturating_sub(blink_time);

        match embassy_time::with_timeout(
            Duration::from_millis(pause_time),
            BLINK_COUNT_SIGNAL.wait(),
        )
        .await
        {
            Ok(new_count) => blink_count = new_count,
            Err(_) => {}
        }
    }
}
