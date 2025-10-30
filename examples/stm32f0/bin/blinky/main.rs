#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

use palette::{Srgb, FromColor, Hsv};
use cortex_m::peripheral::SYST;
use stm32f0xx_hal::{
    gpio::{gpioa, gpiob, Input},
    pac,
    prelude::*,
    pwm,
    time::Hertz,
};

use stm32f0_examples::time_source::{HalTimeSource, HalInstant, HalDuration};
use stm32f0_examples::rgb_led::PwmRgbLed;

use rgb_sequencer::{
    COLOR_OFF,
    LoopCount,
    RgbSequence,
    RgbSequencer,
    TimeSource,
    TimeDuration,
    TransitionStyle,
};

/// Type alias for the LED
pub type Led = PwmRgbLed<
    pwm::PwmChannels<pac::TIM3, pwm::C1>,
    pwm::PwmChannels<pac::TIM3, pwm::C2>,
    pwm::PwmChannels<pac::TIM3, pwm::C3>,
>;

/// Maximum number of steps that can be stored in a sequence
pub const SEQUENCE_STEP_SIZE: usize = 8;
pub const FRAME_RATE_MS: u32 = 16;

/// SysTick interrupt handler - called every 1ms
#[cortex_m_rt::exception]
fn SysTick() {
    stm32f0_examples::time_source::tick();
}

/// Configure the system clock to run at maximum speed
/// 
/// # Returns
/// The configured RCC (Reset and Clock Control) peripheral
fn configure_clock(
    flash: &mut pac::FLASH,
    rcc: pac::RCC,
) -> stm32f0xx_hal::rcc::Rcc {
    let rcc = rcc.configure().freeze(flash);
    
    let sysclk_freq = rcc.clocks.sysclk();
    rprintln!("System clock configured: {} Hz", sysclk_freq.0);
    
    rcc
}

/// Configure SysTick timer for 1ms interrupts
/// 
/// The SysTick interrupt handler increments a global millisecond counter
/// used for timing throughout the application.
fn configure_systick(
    rcc: &stm32f0xx_hal::rcc::Rcc,
    syst: &mut SYST,
) {
    let sysclk_freq = rcc.clocks.sysclk();
    
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload((sysclk_freq.0 / 1_000) - 1);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();
    
    rprintln!("SysTick configured for 1ms interrupts");
}

/// Configure PWM for LED 1 using TIM3
/// 
/// Sets up TIM3 with PWM channels for RGB control:
/// - Red: PA6 (TIM3_CH1)
/// - Green: PA7 (TIM3_CH2)
/// - Blue: PB0 (TIM3_CH3)
/// 
/// # Arguments
/// * `pa6` - GPIO pin for red channel
/// * `pa7` - GPIO pin for green channel
/// * `pb0` - GPIO pin for blue channel
/// * `tim3` - TIM3 peripheral
/// * `rcc` - Reference to RCC for clock configuration
/// 
/// # Returns
/// Configured `Led1` instance (common anode)
fn setup_led(
    pa6: gpioa::PA6<Input<stm32f0xx_hal::gpio::Floating>>,
    pa7: gpioa::PA7<Input<stm32f0xx_hal::gpio::Floating>>,
    pb0: gpiob::PB0<Input<stm32f0xx_hal::gpio::Floating>>,
    tim3: pac::TIM3,
    rcc: &mut stm32f0xx_hal::rcc::Rcc,
) -> Led {
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

/// Sleep until next service time is needed
fn sleep(delay: Option<HalDuration>, time_source: &HalTimeSource) {
    if let Some(delay) = delay {
        let delay_ms = delay.as_millis();
        if delay_ms > 0 {
            // For step transitions, use WFI
            cortex_m::asm::wfi();
        } else {
            // For linear transitions, target ~60fps (16ms)
            let current_time = time_source.now();
            let target_time = current_time.as_millis().wrapping_add(FRAME_RATE_MS);
            loop {
                cortex_m::asm::wfi();
                let now = time_source.now();
                if now.as_millis().wrapping_sub(target_time) < 0x8000_0000 {
                    break;
                }
            }
        }
    } else {
        // Both paused - just sleep and let interrupts wake us
        cortex_m::asm::wfi();
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("=== RGB LED Rainbow Capture Example ===");
    rprintln!("Starting initialization...");

    // Initialize hardware
    let mut dp = pac::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();

    // Configure system clock and SysTick
    let mut rcc = configure_clock(&mut dp.FLASH, dp.RCC);
    configure_systick(&rcc, &mut cp.SYST);

    // Split GPIO ports
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    // Setup hardware components
    let led = setup_led(gpioa.pa6, gpioa.pa7, gpiob.pb0, dp.TIM3, &mut rcc);
    let time_source = HalTimeSource::new();

    rprintln!("=== Hardware Ready ===");

    let mut sequencer: RgbSequencer<HalInstant, Led, HalTimeSource, SEQUENCE_STEP_SIZE>
        = RgbSequencer::new(led, &time_source);

    let sequence = RgbSequence::<HalDuration, SEQUENCE_STEP_SIZE>::new()
        .step(Srgb::from_color(Hsv::new(0.0, 1.0, 1.0)), HalDuration(500), TransitionStyle::Step,)   // Red
        .step(COLOR_OFF, HalDuration(500), TransitionStyle::Linear,)                                 // Off
        .step(Srgb::from_color(Hsv::new(120.0, 1.0, 1.0)), HalDuration(500), TransitionStyle::Step,) // Green
        .step(COLOR_OFF, HalDuration(500), TransitionStyle::Linear,)                                 // Off
        .step(Srgb::from_color(Hsv::new(240.0, 1.0, 1.0)), HalDuration(500), TransitionStyle::Step,) // Blue
        .step(COLOR_OFF, HalDuration(500), TransitionStyle::Linear,)                                 // Off
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap();

    sequencer.load(sequence);
    sequencer.start().unwrap();

    loop {
        if let Ok(delay) = sequencer.service() {
            sleep(delay, &time_source);
        }
    }
}
