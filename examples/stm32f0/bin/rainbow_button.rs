#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

use stm32f0xx_hal::{
    pac,
    prelude::*,
    pwm,
    time::Hertz,
};

use palette::{Srgb, FromColor, Hsv};

use stm32f0_examples::rgb_led::PwmRgbLed;
use stm32f0_examples::time_source::{HalTimeSource, HalInstant, HalDuration};

use rgb_sequencer::{RgbSequencer, RgbSequence, TransitionStyle, LoopCount, TimeDuration, TimeSource, SequencerState};

/// SysTick interrupt handler - called every 1ms
/// 
/// This increments the global millisecond counter used by the time source.
#[cortex_m_rt::exception]
fn SysTick() {
    stm32f0_examples::time_source::tick();
}

#[entry]
fn main() -> ! {
    // Initialize RTT
    rtt_init_print!();
    rprintln!("Starting RGB LED Sequencer with SysTick timing...");

    // Get access to the device peripherals
    let mut dp = pac::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();

    // Configure the system clock
    let mut rcc = dp.RCC.configure().freeze(&mut dp.FLASH);
    let sysclk_freq = rcc.clocks.sysclk();
    rprintln!("System clock: {} Hz", sysclk_freq.0);

    // Configure SysTick to fire every 1ms
    cp.SYST.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    cp.SYST.set_reload((sysclk_freq.0 / 1_000) - 1);
    cp.SYST.clear_current();
    cp.SYST.enable_counter();
    cp.SYST.enable_interrupt();
    rprintln!("SysTick configured for 1ms interrupts");

    // Split GPIO ports
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);

    // Configure button pin (PC13 - User button on Nucleo board)
    let button = cortex_m::interrupt::free(|cs| {
        gpioc.pc13.into_pull_up_input(cs)
    });
    rprintln!("Button configured on PC13");

    // Configure PWM pins for RGB LED
    // Red:   PA6 (TIM3_CH1, AF1)
    // Green: PA7 (TIM3_CH2, AF1)
    // Blue:  PB0 (TIM3_CH3, AF1)
    let pins = cortex_m::interrupt::free(|cs| {
        (
            gpioa.pa6.into_alternate_af1(cs),
            gpioa.pa7.into_alternate_af1(cs),
            gpiob.pb0.into_alternate_af1(cs),
        )
    });
    rprintln!("PWM pins configured");

    // Configure PWM at 1kHz frequency
    let pwm_freq = Hertz(1_000);
    let (red_pwm, green_pwm, blue_pwm) = 
        pwm::tim3(dp.TIM3, pins, &mut rcc, pwm_freq);
    
    // Get the PWM channel types for our LED type annotation
    type RedPwm = pwm::PwmChannels<stm32f0xx_hal::pac::TIM3, pwm::C1>;
    type GreenPwm = pwm::PwmChannels<stm32f0xx_hal::pac::TIM3, pwm::C2>;
    type BluePwm = pwm::PwmChannels<stm32f0xx_hal::pac::TIM3, pwm::C3>;
    
    rprintln!("PWM configured");

    // Create the LED wrapper that implements RgbLed trait
    let led = PwmRgbLed::new(red_pwm, green_pwm, blue_pwm, true);

    // Create the time source (reads from SysTick counter)
    let time_source = HalTimeSource::new();

    // Create the sequencer with our LED and time source
    // The const generic <16> means this sequence can have up to 16 steps
    let mut sequencer = RgbSequencer::<HalInstant, PwmRgbLed<RedPwm, GreenPwm, BluePwm>, HalTimeSource, 16>::new(
        led,
        &time_source
    );

    // Build a rainbow sequence that cycles through the full color spectrum
    let sequence = RgbSequence::new()
        .step(Srgb::from_color(Hsv::new(0.0, 1.0, 1.0)), HalDuration::from_millis(4000), TransitionStyle::Linear)
        .step(Srgb::from_color(Hsv::new(120.0, 1.0, 1.0)), HalDuration::from_millis(4000), TransitionStyle::Linear)
        .step(Srgb::from_color(Hsv::new(240.0, 1.0, 1.0)), HalDuration::from_millis(4000), TransitionStyle::Linear)
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap();

    rprintln!("Rainbow sequence built, cycling through full color spectrum");

    // Load and start the sequence
    sequencer.load(sequence);
    sequencer.start().unwrap();

    rprintln!("Sequence started! Press the user button to pause/resume.");

    // Button debouncing state
    let mut button_pressed = false;
    let mut last_button_time = 0u32;

    // Main loop - service the sequencer and check button
    loop {
        // Get current time for debouncing
        let current_time = time_source.now();
        
        // Check button state (active low on Nucleo boards)
        let button_is_low = button.is_low().unwrap();
        
        if button_is_low && !button_pressed {
            // Check if enough time has passed since last button press (200ms debounce)
            let time_diff = current_time.as_millis().wrapping_sub(last_button_time);
            if time_diff >= 200 {
                // Button just pressed (falling edge)
                button_pressed = true;
                last_button_time = current_time.as_millis();
                
                // Toggle pause/resume based on current state
                match sequencer.get_state() {
                    SequencerState::Running => {
                        rprintln!("Pausing sequence");
                        if let Err(e) = sequencer.pause() {
                            rprintln!("Pause error: {:?}", e);
                        }
                    }
                    SequencerState::Paused => {
                        rprintln!("Resuming sequence");
                        if let Err(e) = sequencer.resume() {
                            rprintln!("Resume error: {:?}", e);
                        }
                    }
                    _ => {
                        rprintln!("Cannot pause/resume from state: {:?}", sequencer.get_state());
                    }
                }
            }
        } else if !button_is_low && button_pressed {
            // Button released
            button_pressed = false;
        }

        // Only service the sequencer if not paused
        if sequencer.get_state() == SequencerState::Running {
            // Update the LED based on current time
            match sequencer.service() {
                Ok(Some(next_service_delay)) => {
                    // The sequencer tells us when it next needs to be serviced
                    let delay_ms = next_service_delay.as_millis();
                    if delay_ms > 0 {
                        // For step transitions, we can sleep until the next step
                        // Use WFI to save power while waiting
                        cortex_m::asm::wfi();
                    } else {
                        // For linear transitions, service at a reasonable frame rate
                        // Sleep for ~16ms (60fps) using WFI
                        // The SysTick will wake us up every 1ms, so we just need to
                        // check time and loop until 16ms has passed
                        let target_time = current_time.as_millis().wrapping_add(16);
                        loop {
                            cortex_m::asm::wfi();
                            let now = time_source.now();
                            if now.as_millis().wrapping_sub(target_time) < 0x8000_0000 {
                                break;
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Sequence completed (won't happen with Infinite loop)
                    rprintln!("Sequence completed");
                    break;
                }
                Err(e) => {
                    rprintln!("Sequencer error: {:?}", e);
                    break;
                }
            }
        } else {
            // When paused, just sleep and let interrupts wake us
            cortex_m::asm::wfi();
        }
    }

    // If we exit the loop, just wait forever
    loop {
        cortex_m::asm::wfi();
    }
}
