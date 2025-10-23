use cortex_m::peripheral::SYST;
use rtt_target::rprintln;
use stm32f0xx_hal::{
    pac,
    pwm,
    time::Hertz,
};

use stm32f0_examples::rgb_led::PwmRgbLed;

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

/// Initialize the system clock and SysTick timer
/// 
/// Configures the system clock and sets up SysTick to fire interrupts every 1ms.
/// 
/// # Returns
/// The configured RCC (Reset and Clock Control) peripheral
pub fn init_clock_and_systick(
    cfgr: stm32f0xx_hal::rcc::CFGR,
    flash: &mut pac::FLASH,
    syst: &mut SYST,
) -> stm32f0xx_hal::rcc::Rcc {
    let rcc = cfgr.freeze(flash);
    let sysclk_freq = rcc.clocks.sysclk();
    rprintln!("System clock: {} Hz", sysclk_freq.0);

    // Configure SysTick to fire every 1ms
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload((sysclk_freq.0 / 1_000) - 1);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();
    rprintln!("SysTick configured for 1ms interrupts");

    rcc
}

/// Configure PWM for LED 1 (TIM3)
/// 
/// Sets up TIM3 with PWM channels for RGB control on PA6, PA7, and PB0.
pub fn setup_led1_pwm(
    pa6: stm32f0xx_hal::gpio::gpioa::PA6<stm32f0xx_hal::gpio::Input<stm32f0xx_hal::gpio::Floating>>,
    pa7: stm32f0xx_hal::gpio::gpioa::PA7<stm32f0xx_hal::gpio::Input<stm32f0xx_hal::gpio::Floating>>,
    pb0: stm32f0xx_hal::gpio::gpiob::PB0<stm32f0xx_hal::gpio::Input<stm32f0xx_hal::gpio::Floating>>,
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
    PwmRgbLed::new(red, green, blue, true)
}

/// Configure PWM for LED 2 (TIM1)
/// 
/// Sets up TIM1 with PWM channels for RGB control on PA8, PA9, and PA10.
pub fn setup_led2_pwm(
    pa8: stm32f0xx_hal::gpio::gpioa::PA8<stm32f0xx_hal::gpio::Input<stm32f0xx_hal::gpio::Floating>>,
    pa9: stm32f0xx_hal::gpio::gpioa::PA9<stm32f0xx_hal::gpio::Input<stm32f0xx_hal::gpio::Floating>>,
    pa10: stm32f0xx_hal::gpio::gpioa::PA10<stm32f0xx_hal::gpio::Input<stm32f0xx_hal::gpio::Floating>>,
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
    PwmRgbLed::new(red, green, blue, true)
}
