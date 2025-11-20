#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

use stm32f0::time_source::HalTimeSource;

mod app_state;
mod button;
mod hardware_setup;
mod sequences;

use app_state::AppState;

/// SysTick interrupt handler - called every 1ms
#[cortex_m_rt::exception]
fn SysTick() {
    stm32f0::time_source::tick();
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("=== RGB LED Mode Switcher Example ===");
    rprintln!("Starting initialization...");

    // Initialize all hardware
    let hw = hardware_setup::init_hardware();
    rprintln!("Hardware initialized successfully");

    // Create time source
    let time_source = HalTimeSource::new();

    // Initialize application state and run
    let mut app = AppState::new(hw, &time_source);

    rprintln!("=== System Ready ===");
    rprintln!("Press button to cycle through modes:");
    rprintln!("  1. Rainbow (color cycle)");
    rprintln!("  2. Police (red/blue alternating)");
    rprintln!("  3. Flame (flickering orange)");
    rprintln!("  4. Breathing (white fade)");

    // Run the main application loop (never returns)
    app.run()
}
