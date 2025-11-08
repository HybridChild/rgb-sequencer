#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

use palette::{FromColor, Hsv, Srgb};
use stm32f0xx_hal::{
    delay::Delay,
    gpio::{Input, gpioa, gpiob},
    pac,
    prelude::*,
    pwm,
    time::Hertz,
};

use stm32f0_examples::rgb_led::PwmRgbLed;

use rgb_sequencer::{
    COLOR_OFF, LoopCount, RgbSequence, RgbSequencer, ServiceTiming, TimeDuration, TimeInstant,
    TimeSource, TransitionStyle,
};

/// Type alias for LED 1
pub type Led1 = PwmRgbLed<
    pwm::PwmChannels<pac::TIM3, pwm::C1>,
    pwm::PwmChannels<pac::TIM3, pwm::C2>,
    pwm::PwmChannels<pac::TIM3, pwm::C3>,
>;

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

/// Configure the system clock
fn configure_clock(flash: &mut pac::FLASH, rcc: pac::RCC) -> stm32f0xx_hal::rcc::Rcc {
    let rcc = rcc.configure().freeze(flash);

    let sysclk_freq = rcc.clocks.sysclk();
    rprintln!("System clock configured: {} Hz", sysclk_freq.0);

    rcc
}

/// Configure PWM for LED using TIM3
fn setup_led1(
    pa6: gpioa::PA6<Input<stm32f0xx_hal::gpio::Floating>>,
    pa7: gpioa::PA7<Input<stm32f0xx_hal::gpio::Floating>>,
    pb0: gpiob::PB0<Input<stm32f0xx_hal::gpio::Floating>>,
    tim3: pac::TIM3,
    rcc: &mut stm32f0xx_hal::rcc::Rcc,
) -> Led1 {
    let pins = cortex_m::interrupt::free(|cs| {
        (
            pa6.into_alternate_af1(cs),
            pa7.into_alternate_af1(cs),
            pb0.into_alternate_af1(cs),
        )
    });

    let pwm_freq = Hertz(1_000);
    let (red, green, blue) = pwm::tim3(tim3, pins, rcc, pwm_freq);

    rprintln!("LED 1 configured on TIM3 (PA6, PA7, PB0)");

    // Common anode = true
    PwmRgbLed::new(red, green, blue, true)
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("=== Simple RGB LED Blinky Example ===");
    rprintln!("Starting initialization...");

    // Initialize hardware
    let mut dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let mut rcc = configure_clock(&mut dp.FLASH, dp.RCC);

    // Create delay provider using SysTick
    let mut delay = Delay::new(cp.SYST, &rcc);

    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    let led_1 = setup_led1(gpioa.pa6, gpioa.pa7, gpiob.pb0, dp.TIM3, &mut rcc);
    let time_source = BlinkyTimeSource::new();

    rprintln!("=== Hardware Ready ===");

    let mut sequencer: RgbSequencer<BlinkyInstant, Led1, BlinkyTimeSource, SEQUENCE_STEP_CAPACITY> =
        RgbSequencer::new(led_1, &time_source);

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
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    rprintln!("Sequence started");

    loop {
        match sequencer.service().unwrap() {
            ServiceTiming::Continuous => {
                // Linear transition - maintain frame rate
                delay.delay_ms(FRAME_RATE_MS as u32);
                time_source.advance(BlinkyDuration(FRAME_RATE_MS));
            }
            ServiceTiming::Delay(delay_duration) => {
                // Step transition - delay for the specified time
                delay.delay_ms(delay_duration.as_millis() as u32);
                time_source.advance(delay_duration);
            }
            ServiceTiming::Complete => {
                // Sequence complete
                break;
            }
        }
    }

    rprintln!("Sequence complete");

    loop {
        cortex_m::asm::wfi();
    }
}
