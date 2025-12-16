//! Transition Styles Example
//!
//! Demonstrates all 6 TransitionStyle variants. Press the user button to cycle through modes.
//! The onboard LED blinks to indicate the current mode:
//! - Solid ON: Step, 1 blink: Linear, 2 blinks: EaseIn, 3 blinks: EaseOut, 4 blinks: EaseInOut, 5 blinks: EaseOutIn

#![no_std]
#![no_main]

use core::future::pending;
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::bind_interrupts;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::peripherals::TIM3;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::{Config, Peripherals};
use {defmt_rtt as _, panic_probe as _};

mod app_logic_task;
mod blink_task;
mod button_task;
mod rgb_task;
mod sequences;
mod types;

use app_logic_task::app_logic_task;
use blink_task::blink_task;
use button_task::button_task;
use rgb_task::rgb_task;

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

/// Initialize PWM for TIM3 (RGB LED 1: PA6, PA7, PB0)
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

    pwm.ch1().enable();
    pwm.ch2().enable();
    pwm.ch3().enable();

    (pwm, max_duty)
}

/// Configure user button with EXTI interrupt
fn setup_button(p: &mut Peripherals) -> ExtiInput<'static> {
    let pc13 = unsafe { p.PC13.clone_unchecked() };
    let exti13 = unsafe { p.EXTI13.clone_unchecked() };

    ExtiInput::new(pc13, exti13, Pull::Up)
}

/// Configure onboard LED
fn setup_onboard_led(p: &mut Peripherals) -> Output<'static> {
    let pa5 = unsafe { p.PA5.clone_unchecked() };
    Output::new(pa5, Level::Low, Speed::Low)
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");

    let config = configure_clock();
    let mut p = embassy_stm32::init(config);

    let button = setup_button(&mut p);
    let (pwm_tim3, max_duty_tim3) = setup_pwm_tim3(&mut p);
    let onboard_led = setup_onboard_led(&mut p);

    spawner.spawn(button_task(button)).unwrap();
    spawner.spawn(blink_task(onboard_led)).unwrap();
    spawner.spawn(app_logic_task()).unwrap();
    spawner.spawn(rgb_task(pwm_tim3, max_duty_tim3)).unwrap();

    info!("Ready!");

    pending::<()>().await;
}
