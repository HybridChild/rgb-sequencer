#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use panic_halt as _;
use rp_pico::entry;
use rp_pico::hal::{
    Clock, Sio, Timer,
    clocks::init_clocks_and_plls,
    pac,
    pwm::{A, B, Channel, FreeRunning, Pwm1, Pwm2, Slice},
    watchdog::Watchdog,
};
use rtt_target::{rprintln, rtt_init_print};

use rp_pico_examples::rgb_led::PwmRgbLed;
use rp_pico_examples::time::{Duration, HardwareTimer, Instant};

use palette::Srgb;
use rgb_sequencer::{RgbSequence8, RgbSequencer8, ServiceTiming, TimeDuration, WHITE};

pub const FRAME_RATE_MS: u64 = 16;

/// Type alias for the RGB LED using PWM channels
pub type Led = PwmRgbLed<
    Channel<Slice<Pwm1, FreeRunning>, A>,
    Channel<Slice<Pwm1, FreeRunning>, B>,
    Channel<Slice<Pwm2, FreeRunning>, A>,
>;

/// Sine-based breathing effect function
fn breathing_sine_wave(base_color: Srgb, elapsed: Duration) -> Srgb {
    // Breathing cycle period in milliseconds (4 seconds total)
    const PERIOD_MS: u64 = 4000;

    // Calculate position within the current breathing cycle (0.0 to 1.0)
    let elapsed_ms = elapsed.as_millis();
    let time_in_cycle = (elapsed_ms % PERIOD_MS) as f32 / PERIOD_MS as f32;

    // Convert to angle in radians (0 to 2π)
    let angle = time_in_cycle * 2.0 * core::f32::consts::PI;

    // Calculate brightness using sine wave
    // sin(angle) ranges from -1 to 1
    // We transform it to range from 0.1 (dim) to 1.0 (bright)
    let sine_value = libm::sinf(angle);
    let brightness = 0.1 + 0.45 * (1.0 + sine_value);

    // Apply brightness to the base color
    Srgb::new(
        base_color.red * brightness,
        base_color.green * brightness,
        base_color.blue * brightness,
    )
}

/// Timing function for continuous animation
fn continuous_timing(_elapsed: Duration) -> Option<Duration> {
    Some(Duration::ZERO)
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("=== RP Pico White Breathing Example ===");
    rprintln!("Starting initialization...");

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
    .ok()
    .unwrap();

    rprintln!(
        "System clock configured: {} Hz",
        clocks.system_clock.freq().to_Hz()
    );

    // Set up the Single Cycle IO (for GPIO access)
    let sio = Sio::new(pac.SIO);

    // Set the pins to their default state
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Initialize PWM for RGB LED
    // Using GPIO2 (Red), GPIO3 (Green), GPIO4 (Blue)
    let mut pwm_slices = rp_pico::hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

    // Configure PWM1 for Red (GPIO2/A) and Green (GPIO3/B)
    pwm_slices.pwm1.set_ph_correct();
    pwm_slices.pwm1.set_div_int(125u8); // 125 MHz / 125 = 1 MHz
    pwm_slices.pwm1.set_top(1000u16); // 1 MHz / 1000 = 1 kHz PWM frequency
    pwm_slices.pwm1.enable();

    // Configure PWM2 for Blue (GPIO4/A)
    pwm_slices.pwm2.set_ph_correct();
    pwm_slices.pwm2.set_div_int(125u8);
    pwm_slices.pwm2.set_top(1000u16);
    pwm_slices.pwm2.enable();

    // Get the channels and bind GPIO pins
    let mut red_channel = pwm_slices.pwm1.channel_a;
    let mut green_channel = pwm_slices.pwm1.channel_b;
    let mut blue_channel = pwm_slices.pwm2.channel_a;

    red_channel.output_to(pins.gpio2);
    green_channel.output_to(pins.gpio3);
    blue_channel.output_to(pins.gpio4);

    rprintln!("RGB LED configured on GPIO2 (R), GPIO3 (G), GPIO4 (B)");

    // Create RGB LED (common anode = true)
    let led = PwmRgbLed::new(red_channel, green_channel, blue_channel, true);

    // Create hardware timer
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let time_source = HardwareTimer::new(timer);

    rprintln!("=== Hardware Ready ===");

    // Create sequencer
    let mut sequencer: RgbSequencer8<Instant, Led, HardwareTimer> =
        RgbSequencer8::new(led, &time_source);

    // Create breathing sequence using function-based animation
    let sequence =
        RgbSequence8::<Duration>::from_function(WHITE, breathing_sine_wave, continuous_timing);

    sequencer.load_and_start(sequence).unwrap();

    rprintln!("Breathing sequence started");

    // Set up delay
    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    loop {
        match sequencer.service().unwrap() {
            ServiceTiming::Continuous => {
                // Function-based animation - maintain frame rate
                delay.delay_ms(FRAME_RATE_MS as u32);
            }
            ServiceTiming::Delay(delay_duration) => {
                // Should not happen for function-based sequence
                delay.delay_ms(delay_duration.as_millis() as u32);
            }
            ServiceTiming::Complete => {
                // Should not happen for infinite sequence
                break;
            }
        }
    }

    rprintln!("Sequence complete");

    loop {
        cortex_m::asm::wfi();
    }
}
