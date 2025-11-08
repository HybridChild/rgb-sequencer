#![no_std]
#![no_main]

use core::future::pending;
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::peripherals::{TIM1, TIM3};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::{Config, Peripherals, bind_interrupts};
use {defmt_rtt as _, panic_probe as _};

mod app_logic_task;
mod button_task;
mod rgb_task;
mod types;

use app_logic_task::app_logic_task;
use button_task::button_task;
use rgb_task::rgb_task;

// Bind interrupts for Embassy's time driver
bind_interrupts!(
    struct Irqs {}
);

/// Configure system clock with HSE and PLL
fn configure_clock() -> Config {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: Hertz(8_000_000),
            mode: HseMode::Bypass,
        });
        config.rcc.pll = Some(Pll {
            src: PllSource::HSE,
            prediv: PllPreDiv::DIV2,
            mul: PllMul::MUL12,
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV1;
    }
    config
}

/// Initialize PWM for TIM3 (LED 1: PA6, PA7, PB0)
fn setup_pwm_tim3(p: &mut Peripherals) -> (SimplePwm<'static, TIM3>, u16) {
    let tim3 = unsafe { p.TIM3.clone_unchecked() };
    let pa6 = unsafe { p.PA6.clone_unchecked() };
    let pa7 = unsafe { p.PA7.clone_unchecked() };
    let pb0 = unsafe { p.PB0.clone_unchecked() };

    let ch1_pin = PwmPin::new(pa6, embassy_stm32::gpio::OutputType::PushPull);
    let ch2_pin = PwmPin::new(pa7, embassy_stm32::gpio::OutputType::PushPull);
    let ch3_pin = PwmPin::new(pb0, embassy_stm32::gpio::OutputType::PushPull);

    let mut pwm = SimplePwm::new(
        tim3,
        Some(ch1_pin),
        Some(ch2_pin),
        Some(ch3_pin),
        None,
        Hertz(1_000),
        Default::default(),
    );

    let max_duty = pwm.max_duty_cycle();

    // Enable all PWM channels
    pwm.ch1().enable();
    pwm.ch2().enable();
    pwm.ch3().enable();

    info!("LED 1 PWM configured on TIM3 (PA6, PA7, PB0)");

    (pwm, max_duty)
}

/// Initialize PWM for TIM1 (LED 2: PA8, PA9, PA10)
fn setup_pwm_tim1(p: &mut Peripherals) -> (SimplePwm<'static, TIM1>, u16) {
    let tim1 = unsafe { p.TIM1.clone_unchecked() };
    let pa8 = unsafe { p.PA8.clone_unchecked() };
    let pa9 = unsafe { p.PA9.clone_unchecked() };
    let pa10 = unsafe { p.PA10.clone_unchecked() };

    let ch1_pin = PwmPin::new(pa8, embassy_stm32::gpio::OutputType::PushPull);
    let ch2_pin = PwmPin::new(pa9, embassy_stm32::gpio::OutputType::PushPull);
    let ch3_pin = PwmPin::new(pa10, embassy_stm32::gpio::OutputType::PushPull);

    let mut pwm = SimplePwm::new(
        tim1,
        Some(ch1_pin),
        Some(ch2_pin),
        Some(ch3_pin),
        None,
        Hertz(1_000),
        Default::default(),
    );

    let max_duty = pwm.max_duty_cycle();

    // Enable all PWM channels
    pwm.ch1().enable();
    pwm.ch2().enable();
    pwm.ch3().enable();

    info!("LED 2 PWM configured on TIM1 (PA8, PA9, PA10)");

    (pwm, max_duty)
}

/// Configure user button with EXTI interrupt
fn setup_button(p: &mut Peripherals) -> ExtiInput<'static> {
    let pc13 = unsafe { p.PC13.clone_unchecked() };
    let exti13 = unsafe { p.EXTI13.clone_unchecked() };

    let button = ExtiInput::new(pc13, exti13, Pull::Up);
    info!("User button configured on PC13");

    button
}

/// Configure onboard LED (PA5 on Nucleo board)
fn setup_onboard_led(p: &mut Peripherals) -> Output<'static> {
    let pa5 = unsafe { p.PA5.clone_unchecked() };
    let led = Output::new(pa5, Level::Low, Speed::Low);
    info!("Onboard LED configured on PA5");

    led
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("=== RGB LED Rainbow Button Example ===");
    info!("Starting initialization...");

    // Initialize peripherals with clock configuration
    let config = configure_clock();
    let mut p = embassy_stm32::init(config);
    info!("Clock configured - system running at 48 MHz");

    // Setup hardware
    let button = setup_button(&mut p);
    let (pwm_tim3, max_duty_tim3) = setup_pwm_tim3(&mut p);
    let (pwm_tim1, max_duty_tim1) = setup_pwm_tim1(&mut p);
    let onboard_led = setup_onboard_led(&mut p);

    info!("All hardware initialized successfully");

    // Spawn async tasks
    spawner.spawn(button_task(button)).unwrap();
    spawner.spawn(app_logic_task(onboard_led)).unwrap();
    spawner
        .spawn(rgb_task(pwm_tim3, max_duty_tim3, pwm_tim1, max_duty_tim1))
        .unwrap();

    info!("=== System Ready ===");
    info!("LED 1: Rainbow animation (red -> green -> blue)");
    info!("LED 2: Off (will capture colors from LED 1)");
    info!("Press button to pause LED 1 and capture color to LED 2");
    info!("Press again to resume LED 1 animation");

    // Main task has no more work to do - all logic is in spawned tasks
    pending::<()>().await;
}
