use cortex_m::peripheral::SYST;
use rtt_target::rprintln;
use stm32f0xx_hal::{
    gpio::{Input, PullUp, gpioa, gpiob, gpioc},
    pac,
    prelude::*,
    pwm,
    time::Hertz,
};

use stm32f0::rgb_led::PwmRgbLed;

/// Type aliases for the two LEDs
pub type Led1 = PwmRgbLed<
    pwm::PwmChannels<pac::TIM3, pwm::C1>,
    pwm::PwmChannels<pac::TIM3, pwm::C2>,
    pwm::PwmChannels<pac::TIM3, pwm::C3>,
>;

pub type Led2 = PwmRgbLed<
    pwm::PwmChannels<pac::TIM1, pwm::C1>,
    pwm::PwmChannels<pac::TIM1, pwm::C2>,
    pwm::PwmChannels<pac::TIM1, pwm::C3>,
>;

/// Button type (user button on PC13)
pub type Button = gpioc::PC13<Input<PullUp>>;

/// Container for all initialized hardware peripherals
pub struct HardwareContext {
    pub led_1: Led1,
    pub led_2: Led2,
    pub button: Button,
}

/// Initialize all hardware peripherals
///
/// This function handles all hardware initialization in one place:
/// - System clock configuration
/// - SysTick timer setup (1ms interrupts)
/// - GPIO port initialization
/// - PWM configuration for both RGB LEDs
/// - Button configuration
///
/// # Returns
/// A `HardwareContext` containing all initialized peripherals ready for use
pub fn init_hardware() -> HardwareContext {
    let mut dp = pac::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();

    // Configure system clock and SysTick
    let mut rcc = configure_clock(&mut dp.FLASH, dp.RCC);
    configure_systick(&rcc, &mut cp.SYST);

    // Split GPIO ports
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);

    // Setup hardware components
    let led_1 = setup_led1_pwm(gpioa.pa6, gpioa.pa7, gpiob.pb0, dp.TIM3, &mut rcc);
    let led_2 = setup_led2_pwm(gpioa.pa8, gpioa.pa9, gpioa.pa10, dp.TIM1, &mut rcc);
    let button = setup_button(gpioc.pc13);

    HardwareContext {
        led_1,
        led_2,
        button,
    }
}

/// Configure the system clock to run at maximum speed
///
/// # Returns
/// The configured RCC (Reset and Clock Control) peripheral
fn configure_clock(flash: &mut pac::FLASH, rcc: pac::RCC) -> stm32f0xx_hal::rcc::Rcc {
    let rcc = rcc.configure().freeze(flash);

    let sysclk_freq = rcc.clocks.sysclk();
    rprintln!("System clock configured: {} Hz", sysclk_freq.0);

    rcc
}

/// Configure SysTick timer for 1ms interrupts
///
/// The SysTick interrupt handler increments a global millisecond counter
/// used for timing throughout the application.
fn configure_systick(rcc: &stm32f0xx_hal::rcc::Rcc, syst: &mut SYST) {
    let sysclk_freq = rcc.clocks.sysclk();

    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload((sysclk_freq.0 / 1_000) - 1);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();

    rprintln!("SysTick configured for 1ms interrupts");
}

/// Configure user button (PC13) with pull-up
fn setup_button(pc13: gpioc::PC13<Input<stm32f0xx_hal::gpio::Floating>>) -> Button {
    let button = cortex_m::interrupt::free(|cs| pc13.into_pull_up_input(cs));

    rprintln!("Button configured on PC13");
    button
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
fn setup_led1_pwm(
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

/// Configure PWM for LED 2 using TIM1
///
/// Sets up TIM1 with PWM channels for RGB control:
/// - Red: PA8 (TIM1_CH1)
/// - Green: PA9 (TIM1_CH2)
/// - Blue: PA10 (TIM1_CH3)
///
/// # Arguments
/// * `pa8` - GPIO pin for red channel
/// * `pa9` - GPIO pin for green channel
/// * `pa10` - GPIO pin for blue channel
/// * `tim1` - TIM1 peripheral
/// * `rcc` - Reference to RCC for clock configuration
///
/// # Returns
/// Configured `Led2` instance (common anode)
fn setup_led2_pwm(
    pa8: gpioa::PA8<Input<stm32f0xx_hal::gpio::Floating>>,
    pa9: gpioa::PA9<Input<stm32f0xx_hal::gpio::Floating>>,
    pa10: gpioa::PA10<Input<stm32f0xx_hal::gpio::Floating>>,
    tim1: pac::TIM1,
    rcc: &mut stm32f0xx_hal::rcc::Rcc,
) -> Led2 {
    let pins = cortex_m::interrupt::free(|cs| {
        (
            pa8.into_alternate_af2(cs),
            pa9.into_alternate_af2(cs),
            pa10.into_alternate_af2(cs),
        )
    });

    let pwm_freq = Hertz(1_000);
    let (red, green, blue) = pwm::tim1(tim1, pins, rcc, pwm_freq);

    rprintln!("LED 2 configured on TIM1 (PA8, PA9, PA10)");

    // Common anode = true
    PwmRgbLed::new(red, green, blue, true)
}
