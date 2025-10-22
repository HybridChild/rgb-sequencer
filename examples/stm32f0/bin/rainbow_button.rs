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

use rgb_sequencer::{RgbSequencer, RgbSequence, TransitionStyle, LoopCount, SequencerState, TimeDuration, TimeSource, COLOR_OFF};

/// SysTick interrupt handler - called every 1ms
#[cortex_m_rt::exception]
fn SysTick() {
    stm32f0_examples::time_source::tick();
}

type Led1 = PwmRgbLed<
    pwm::PwmChannels<pac::TIM3, pwm::C1>,
    pwm::PwmChannels<pac::TIM3, pwm::C2>,
    pwm::PwmChannels<pac::TIM3, pwm::C3>,
>;

type Led2 = PwmRgbLed<
    pwm::PwmChannels<pac::TIM1, pwm::C1>,
    pwm::PwmChannels<pac::TIM1, pwm::C2>,
    pwm::PwmChannels<pac::TIM1, pwm::C3>,
>;

type Sequencer1<'a> = RgbSequencer<'a, HalInstant, Led1, HalTimeSource, 16>;
type Sequencer2<'a> = RgbSequencer<'a, HalInstant, Led2, HalTimeSource, 16>;

/// Initialize the system clock and SysTick timer
fn init_clock_and_systick(
    cfgr: stm32f0xx_hal::rcc::CFGR,
    flash: &mut pac::FLASH,
    syst: &mut cortex_m::peripheral::SYST,
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
fn setup_led1_pwm(
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
fn setup_led2_pwm(
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

/// Create a rainbow sequence that cycles through the full color spectrum
fn create_rainbow_sequence() -> RgbSequence<HalDuration, 16> {    
    RgbSequence::new()
        .step(
            Srgb::from_color(Hsv::new(0.0, 1.0, 1.0)),
            HalDuration::from_millis(4000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::from_color(Hsv::new(120.0, 1.0, 1.0)),
            HalDuration::from_millis(4000),
            TransitionStyle::Linear,
        )
        .step(
            Srgb::from_color(Hsv::new(240.0, 1.0, 1.0)),
            HalDuration::from_millis(4000),
            TransitionStyle::Linear,
        )
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

/// Handle button press - toggle pause/resume for both sequencers
fn handle_button_press(sequencer_1: &mut Sequencer1<'_>, sequencer_2: &mut Sequencer2<'_>) {
    match sequencer_1.get_state() {
        SequencerState::Running => {
            rprintln!("Pause LED 1 sequence and capture current color to LED 2");
            if let Err(e) = sequencer_1.pause() {
                rprintln!("Pause error LED 1: {:?}", e);
            }

            let new_color = sequencer_1.current_color();
            let new_sequence = RgbSequence::new().step(new_color, HalDuration::from_millis(0), TransitionStyle::Step).build().unwrap();
            sequencer_2.load(new_sequence);
            sequencer_2.start().unwrap();
        }
        SequencerState::Paused => {
            rprintln!("Resume LED 1 sequence");
            if let Err(e) = sequencer_1.resume() {
                rprintln!("Resume error LED 1: {:?}", e);
            }
        }
        _ => {
            rprintln!("Cannot pause/resume from state: {:?}", sequencer_1.get_state());
        }
    }
}

/// Service both sequencers and return the minimum delay needed
fn service_both_sequencers(
    sequencer_1: &mut Sequencer1<'_>,
    sequencer_2: &mut Sequencer2<'_>,
) -> Option<HalDuration> {
    let state_1 = sequencer_1.get_state();
    let state_2 = sequencer_2.get_state();
    
    let mut min_delay: Option<HalDuration> = None;
    
    // Service LED 1
    if state_1 == SequencerState::Running {
        match sequencer_1.service() {
            Ok(Some(delay)) => {
                min_delay = Some(match min_delay {
                    None => delay,
                    Some(current_min) => {
                        if delay.as_millis() < current_min.as_millis() {
                            delay
                        } else {
                            current_min
                        }
                    }
                });
            }
            Ok(None) => {
                rprintln!("LED 1 sequence completed");
            }
            Err(e) => {
                rprintln!("LED 1 sequencer error: {:?}", e);
            }
        }
    }
    
    // Service LED 2
    if state_2 == SequencerState::Running {
        match sequencer_2.service() {
            Ok(Some(delay)) => {
                min_delay = Some(match min_delay {
                    None => delay,
                    Some(current_min) => {
                        if delay.as_millis() < current_min.as_millis() {
                            delay
                        } else {
                            current_min
                        }
                    }
                });
            }
            Ok(None) => {
                rprintln!("LED 2 sequence completed");
            }
            Err(e) => {
                rprintln!("LED 2 sequencer error: {:?}", e);
            }
        }
    }
    
    min_delay
}

/// Sleep for the specified duration or frame rate
fn sleep_until_next_service(
    delay: Option<HalDuration>,
    current_time: HalInstant,
    time_source: &HalTimeSource,
) {
    if let Some(delay) = delay {
        let delay_ms = delay.as_millis();
        if delay_ms > 0 {
            // For step transitions, use WFI
            cortex_m::asm::wfi();
        } else {
            // For linear transitions, target ~60fps (16ms)
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
}

/// Simple button debouncer
struct ButtonDebouncer {
    pressed: bool,
    last_press_time: u32,
    debounce_ms: u32,
}

impl ButtonDebouncer {
    fn new(debounce_ms: u32) -> Self {
        Self {
            pressed: false,
            last_press_time: 0,
            debounce_ms,
        }
    }
    
    /// Check if button was just pressed (returns true on falling edge)
    fn check_press(&mut self, button_is_low: bool, current_time_ms: u32) -> bool {
        if button_is_low && !self.pressed {
            let time_diff = current_time_ms.wrapping_sub(self.last_press_time);
            if time_diff >= self.debounce_ms {
                self.pressed = true;
                self.last_press_time = current_time_ms;
                return true;
            }
        } else if !button_is_low && self.pressed {
            self.pressed = false;
        }
        false
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Starting RGB LED Sequencer with TWO LEDs...");

    let mut dp = pac::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();

    // Initialize clock and timing
    let mut rcc = init_clock_and_systick(
        dp.RCC.configure(),
        &mut dp.FLASH,
        &mut cp.SYST,
    );

    // Split GPIO ports
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);

    // Configure button (PC13 - User button on Nucleo board)
    let button = cortex_m::interrupt::free(|cs| {
        gpioc.pc13.into_pull_up_input(cs)
    });
    rprintln!("Button configured on PC13");

    // Setup LEDs
    let led_1 = setup_led1_pwm(gpioa.pa6, gpioa.pa7, gpiob.pb0, dp.TIM3, &mut rcc);
    let led_2 = setup_led2_pwm(gpioa.pa8, gpioa.pa9, gpioa.pa10, dp.TIM1, &mut rcc);

    // Create time source
    let time_source = HalTimeSource::new();

    // Create sequencers
    let mut sequencer_1 = RgbSequencer::new(led_1, &time_source);
    let mut sequencer_2 = RgbSequencer::new(led_2, &time_source);

    // Create and load sequences
    let sequence_1 = create_rainbow_sequence();  // 12 second cycle
    let sequence_2 = RgbSequence::new().step(COLOR_OFF, HalDuration::from_millis(0), TransitionStyle::Step).build().unwrap();
    
    sequencer_1.load(sequence_1);
    sequencer_1.start().unwrap();
    
    sequencer_2.load(sequence_2);
    sequencer_2.start().unwrap();

    rprintln!("Both sequences started!");
    rprintln!("Press the user button to pause/resume both LEDs.");

    let mut button_debouncer = ButtonDebouncer::new(200);

    // Main loop
    loop {
        let current_time = time_source.now();
        let button_is_low = button.is_low().unwrap();
        
        // Handle button press
        if button_debouncer.check_press(button_is_low, current_time.as_millis()) {
            handle_button_press(&mut sequencer_1, &mut sequencer_2);
        }

        // Service sequencers
        let delay = service_both_sequencers(&mut sequencer_1, &mut sequencer_2);
        
        // Sleep appropriately
        if delay.is_some() {
            sleep_until_next_service(delay, current_time, &time_source);
        } else {
            // Both paused - just sleep and let interrupts wake us
            cortex_m::asm::wfi();
        }
    }
}
