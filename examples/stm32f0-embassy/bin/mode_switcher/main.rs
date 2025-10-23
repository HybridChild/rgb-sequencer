#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed, Pull};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::{bind_interrupts, Config};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

mod button_task;
mod rgb_task;
mod app_logic_task;
mod types;

use button_task::button_task;
use rgb_task::rgb_task;
use app_logic_task::app_logic_task;

// Bind interrupts for Embassy's time driver
bind_interrupts!(struct Irqs {});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting RGB Sequencer Embassy Example...");

    // Initialize STM32 peripherals with default config
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
    
    let p = embassy_stm32::init(config);
    info!("Peripherals initialized");

    // Configure user button (PC13) with interrupt
    let button_exti = embassy_stm32::exti::ExtiInput::new(p.PC13, p.EXTI13, Pull::Up);
    info!("Button configured on PC13");

    // Configure PWM for LED 1 (TIM3: PA6, PA7, PB0)
    let ch1_pin = PwmPin::new(p.PA6, embassy_stm32::gpio::OutputType::PushPull);
    let ch2_pin = PwmPin::new(p.PA7, embassy_stm32::gpio::OutputType::PushPull);
    let ch3_pin = PwmPin::new(p.PB0, embassy_stm32::gpio::OutputType::PushPull);
    
    let pwm_tim3 = SimplePwm::new(
        p.TIM3,
        Some(ch1_pin),
        Some(ch2_pin),
        Some(ch3_pin),
        None,
        Hertz(1_000),
        Default::default(),
    );
    
    let max_duty_tim3 = pwm_tim3.max_duty_cycle();
    
    info!("LED 1 PWM configured on TIM3 (PA6, PA7, PB0), max_duty: {}", max_duty_tim3);

    // Configure PWM for LED 2 (TIM1: PA8, PA9, PA10)
    let ch1_pin = PwmPin::new(p.PA8, embassy_stm32::gpio::OutputType::PushPull);
    let ch2_pin = PwmPin::new(p.PA9, embassy_stm32::gpio::OutputType::PushPull);
    let ch3_pin = PwmPin::new(p.PA10, embassy_stm32::gpio::OutputType::PushPull);
    
    let pwm_tim1 = SimplePwm::new(
        p.TIM1,
        Some(ch1_pin),
        Some(ch2_pin),
        Some(ch3_pin),
        None,
        Hertz(1_000),
        Default::default(),
    );
    
    let max_duty_tim1 = pwm_tim1.max_duty_cycle();
    
    info!("LED 2 PWM configured on TIM1 (PA8, PA9, PA10), max_duty: {}", max_duty_tim1);

    // Configure onboard LED (PA5) for mode indication
    let onboard_led = Output::new(p.PA5, Level::Low, Speed::Low);
    info!("Onboard LED configured on PA5");

    // Spawn tasks
    spawner.spawn(button_task(button_exti)).unwrap();
    info!("Button task spawned");

    spawner.spawn(app_logic_task(onboard_led)).unwrap();
    info!("App logic task spawned");

    spawner.spawn(rgb_task(pwm_tim3, max_duty_tim3, pwm_tim1, max_duty_tim1)).unwrap();
    info!("RGB task spawned");

    info!("All tasks spawned successfully!");
    info!("Press the user button to cycle through modes");
    
    // Main task can just sleep - everything is handled by spawned tasks
    loop {
        Timer::after_secs(60).await;
    }
}