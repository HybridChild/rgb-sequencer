#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use embedded_hal::digital::OutputPin;
use panic_halt as _;
use rp_pico::entry;
use rp_pico::hal::{clocks::init_clocks_and_plls, pac, watchdog::Watchdog, Clock, Sio};

#[entry]
fn main() -> ! {
    // Get peripherals
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    
    // Set up watchdog driver
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    
    // Configure clocks (125 MHz)
    let clocks = init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok().unwrap();
    
    // Set up the Single Cycle IO (for GPIO access)
    let sio = Sio::new(pac.SIO);
    
    // Set the pins to their default state
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    
    // Configure GPIO25 (onboard LED) as an output
    let mut led_pin = pins.led.into_push_pull_output();
    
    // Set up delay
    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    
    // Blink loop
    loop {
        led_pin.set_high().unwrap();
        delay.delay_ms(500);
        led_pin.set_low().unwrap();
        delay.delay_ms(500);
    }
}
