use defmt::info;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_stm32::peripherals::{TIM1, TIM3};
use embassy_time::{Duration, Timer};
use embassy_futures::select::{select, Either};
use palette::Srgb;
use rgb_sequencer::{RgbSequencer, RgbLed, SequencerState};

use crate::types::{RgbCommand, RGB_COMMAND_CHANNEL, EmbassyDuration, EmbassyInstant, EmbassyTimeSource, SEQUENCE_STEP_SIZE};

/// PWM-based RGB LED implementation for Embassy
pub struct EmbassyPwmRgbLed<'d, T: embassy_stm32::timer::GeneralInstance4Channel> {
    pwm: SimplePwm<'d, T>,
    max_duty: u16,
    common_anode: bool,
}

impl<'d, T: embassy_stm32::timer::GeneralInstance4Channel> EmbassyPwmRgbLed<'d, T> {
    pub fn new(pwm: SimplePwm<'d, T>, max_duty: u16, common_anode: bool) -> Self {
        Self {
            pwm,
            max_duty,
            common_anode,
        }
    }

    /// Convert float (0.0-1.0) to PWM duty cycle
    fn float_to_duty(&self, value: f32) -> u16 {
        let value_clamped = value.clamp(0.0, 1.0);
        let duty = (value_clamped * self.max_duty as f32) as u16;
        
        if self.common_anode {
            self.max_duty - duty
        } else {
            duty
        }
    }
}

impl<'d, T: embassy_stm32::timer::GeneralInstance4Channel> RgbLed for EmbassyPwmRgbLed<'d, T> {
    fn set_color(&mut self, color: Srgb) {
        let red_duty = self.float_to_duty(color.red);
        let green_duty = self.float_to_duty(color.green);
        let blue_duty = self.float_to_duty(color.blue);
        
        self.pwm.ch1().set_duty_cycle(red_duty);
        self.pwm.ch2().set_duty_cycle(green_duty);
        self.pwm.ch3().set_duty_cycle(blue_duty);
    }
}

#[embassy_executor::task]
pub async fn rgb_task(
    pwm_tim3: SimplePwm<'static, TIM3>,
    max_duty_tim3: u16,
    pwm_tim1: SimplePwm<'static, TIM1>,
    max_duty_tim1: u16,
) {
    info!("RGB ready");
    
    // Create LED wrappers (common anode = true)
    let led1 = EmbassyPwmRgbLed::new(pwm_tim3, max_duty_tim3, true);
    let led2 = EmbassyPwmRgbLed::new(pwm_tim1, max_duty_tim1, true);
    
    // Create time source
    let time_source = EmbassyTimeSource::new();
    
    // Create sequencers
    let mut sequencer1 = RgbSequencer::new(led1, &time_source);
    let mut sequencer2 = RgbSequencer::new(led2, &time_source);
    
    // Start with a short delay, will be updated after first service
    let mut next_service_delay = Duration::from_millis(16);
    
    loop {
        // Wait for either a command or the next service time
        match select(
            RGB_COMMAND_CHANNEL.receive(),
            Timer::after(next_service_delay)
        ).await {
            Either::First(command) => {
                match command {
                    RgbCommand::LoadCoordinated(sequence) => {
                        // Load the same sequence on both LEDs
                        sequencer1.load(sequence.clone());
                        sequencer2.load(sequence);
                        
                        // Start both sequencers
                        let _ = sequencer1.start();
                        let _ = sequencer2.start();
                        
                        // Immediately service to get first colors and timing
                        next_service_delay = service_both_sequencers(&mut sequencer1, &mut sequencer2);
                    }
                }
            }
            Either::Second(_) => {
                // Time to service the sequencers
                let state1 = sequencer1.get_state();
                let state2 = sequencer2.get_state();
                
                // Only service if at least one is running
                if state1 == SequencerState::Running || state2 == SequencerState::Running {
                    next_service_delay = service_both_sequencers(&mut sequencer1, &mut sequencer2);
                } else {
                    // Both idle/stopped - wait for commands
                    next_service_delay = Duration::from_secs(3600);
                }
            }
        }
    }
}

/// Service both sequencers and return the minimum delay until next service
fn service_both_sequencers(
    sequencer1: &mut RgbSequencer<'_, EmbassyInstant, EmbassyPwmRgbLed<'_, TIM3>, EmbassyTimeSource, SEQUENCE_STEP_SIZE>,
    sequencer2: &mut RgbSequencer<'_, EmbassyInstant, EmbassyPwmRgbLed<'_, TIM1>, EmbassyTimeSource, SEQUENCE_STEP_SIZE>,
) -> Duration {
    let mut min_delay: Option<EmbassyDuration> = None;
    
    // Service sequencer 1 if running
    if sequencer1.get_state() == SequencerState::Running {
        if let Ok(Some(delay)) = sequencer1.service() {
            min_delay = Some(match min_delay {
                None => delay,
                Some(current_min) => {
                    if delay.0.as_millis() < current_min.0.as_millis() {
                        delay
                    } else {
                        current_min
                    }
                }
            });
        }
    }
    
    // Service sequencer 2 if running
    if sequencer2.get_state() == SequencerState::Running {
        if let Ok(Some(delay)) = sequencer2.service() {
            min_delay = Some(match min_delay {
                None => delay,
                Some(current_min) => {
                    if delay.0.as_millis() < current_min.0.as_millis() {
                        delay
                    } else {
                        current_min
                    }
                }
            });
        }
    }
    
    // Convert to Embassy Duration
    match min_delay {
        Some(delay) if delay.0.as_millis() == 0 => {
            // Linear transition - service at ~60fps
            Duration::from_millis(16)
        }
        Some(delay) => {
            // Step transition - use the delay
            delay.0
        }
        None => {
            // Both sequences complete - wait indefinitely
            Duration::from_secs(3600)
        }
    }
}
