#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use panic_halt as _;
use rp_pico::entry;
use rp_pico::hal::{
    clocks::init_clocks_and_plls,
    pac,
    pwm::{Channel, FreeRunning, Pwm1, Pwm2, Slice, A, B},
    watchdog::Watchdog,
    Clock,
    Sio,
};
use rtt_target::{rprintln, rtt_init_print};

use palette::{FromColor, Hsv, Srgb};

use rp_pico_examples::rgb_led::PwmRgbLed;

use rgb_sequencer::{
    LoopCount, RgbSequence, RgbSequencer, TimeDuration, TimeInstant, TimeSource, TransitionStyle,
    COLOR_OFF,
};

/// Maximum number of steps that can be stored in a sequence
pub const SEQUENCE_STEP_CAPACITY: usize = 8;
pub const FRAME_RATE_MS: u64 = 16;

/// Duration type using milliseconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlinkyDuration(pub u64);

impl TimeDuration for BlinkyDuration {
    const ZERO: Self = BlinkyDuration(0);

    fn as_millis(&self) -> u64 {
        self.0
    }

    fn from_millis(millis: u64) -> Self {
        BlinkyDuration(millis)
    }

    fn saturating_sub(self, other: Self) -> Self {
        BlinkyDuration(self.0.saturating_sub(other.0))
    }
}

/// Instant type representing a point in time (in milliseconds)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlinkyInstant(u64);

impl TimeInstant for BlinkyInstant {
    type Duration = BlinkyDuration;

    fn duration_since(&self, earlier: Self) -> Self::Duration {
        BlinkyDuration(self.0.saturating_sub(earlier.0))
    }

    fn checked_add(self, duration: Self::Duration) -> Option<Self> {
        Some(BlinkyInstant(self.0.saturating_add(duration.0)))
    }

    fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_sub(duration.0).map(BlinkyInstant)
    }
}

/// Simple time source that increments on each call
///
/// This works because we call `now()` only after each service/delay cycle,
/// so the time advances naturally with the delays.
pub struct BlinkyTimeSource {
    current_time: core::cell::Cell<u64>,
}

impl BlinkyTimeSource {
    pub fn new() -> Self {
        Self {
            current_time: core::cell::Cell::new(0),
        }
    }

    /// Advance time by the given duration
    pub fn advance(&self, duration: BlinkyDuration) {
        let current = self.current_time.get();
        self.current_time.set(current + duration.as_millis());
    }
}

impl TimeSource<BlinkyInstant> for BlinkyTimeSource {
    fn now(&self) -> BlinkyInstant {
        BlinkyInstant(self.current_time.get())
    }
}

/// Type alias for the RGB LED using PWM channels
pub type Led1 = PwmRgbLed<
    Channel<Slice<Pwm1, FreeRunning>, A>,
    Channel<Slice<Pwm1, FreeRunning>, B>,
    Channel<Slice<Pwm2, FreeRunning>, A>,
>;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("=== RP Pico RGB LED Blinky Example ===");
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

    // Create RGB LED (common cathode = false)
    let led_1 = PwmRgbLed::new(red_channel, green_channel, blue_channel, true);

    // Create time source
    let time_source = BlinkyTimeSource::new();

    rprintln!("=== Hardware Ready ===");

    // Create sequencer
    let mut sequencer: RgbSequencer<
        BlinkyInstant,
        Led1,
        BlinkyTimeSource,
        SEQUENCE_STEP_CAPACITY,
    > = RgbSequencer::new(led_1, &time_source);

    // Create a sequence
    let sequence = RgbSequence::<BlinkyDuration, SEQUENCE_STEP_CAPACITY>::new()
        .step(
            Srgb::from_color(Hsv::new(60.0, 1.0, 1.0)),
            BlinkyDuration(0),
            TransitionStyle::Step,
        ) // Yellow
        .step(COLOR_OFF, BlinkyDuration(1000), TransitionStyle::Linear) // Fade out
        .step(
            Srgb::from_color(Hsv::new(180.0, 1.0, 1.0)),
            BlinkyDuration(0),
            TransitionStyle::Step,
        ) // Cyan
        .step(COLOR_OFF, BlinkyDuration(1000), TransitionStyle::Linear) // Fade out
        .step(
            Srgb::from_color(Hsv::new(300.0, 1.0, 1.0)),
            BlinkyDuration(0),
            TransitionStyle::Step,
        ) // Purple
        .step(COLOR_OFF, BlinkyDuration(1000), TransitionStyle::Linear) // Fade out
        .loop_count(LoopCount::Finite(3))
        .landing_color(Srgb::new(1.0, 1.0, 1.0))
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    rprintln!("Sequence started");

    // Set up delay
    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    loop {
        if let Some(delay_duration) = sequencer.service().unwrap() {
            if delay_duration == TimeDuration::ZERO {
                // Linear transition - maintain frame rate
                delay.delay_ms(FRAME_RATE_MS as u32);
                time_source.advance(BlinkyDuration(FRAME_RATE_MS));
            } else {
                // Step transition - delay for the specified time
                delay.delay_ms(delay_duration.as_millis() as u32);
                time_source.advance(delay_duration);
            }
        } else {
            // Sequence complete
            break;
        }
    }

    rprintln!("Sequence complete");

    loop {
        cortex_m::asm::wfi();
    }
}
