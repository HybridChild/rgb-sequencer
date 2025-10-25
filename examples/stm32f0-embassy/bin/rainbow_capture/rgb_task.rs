use defmt::info;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_stm32::peripherals::{TIM1, TIM3};
use embassy_time::{Duration, Timer};
use embassy_futures::select::{select, Either};
use heapless::Vec;
use palette::Srgb;
use rgb_sequencer::{RgbSequencer, RgbLed};

use crate::types::{RgbCommand, RGB_COMMAND_CHANNEL, EmbassyDuration, EmbassyInstant, EmbassyTimeSource, SEQUENCE_STEP_SIZE, LedId};

// ============================================================================
// PWM-based RGB LED implementation for Embassy
// ============================================================================

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

// ============================================================================
// Enum to unify different LED types
// ============================================================================

/// Enum that can hold either TIM1 or TIM3 based LED.
/// 
/// This is the key to storing sequencers with different timer types
/// in the same collection without heap allocation!
pub enum AnyLed<'d> {
    Tim1(EmbassyPwmRgbLed<'d, TIM1>),
    Tim3(EmbassyPwmRgbLed<'d, TIM3>),
}

impl<'d> RgbLed for AnyLed<'d> {
    fn set_color(&mut self, color: Srgb) {
        match self {
            AnyLed::Tim1(led) => led.set_color(color),
            AnyLed::Tim3(led) => led.set_color(color),
        }
    }
}

// ============================================================================
// Heterogeneous Collection with Individual LED Control
// ============================================================================

/// Collection of sequencers with different LED types (TIM1 and TIM3).
/// 
/// All sequencers have the same type: RgbSequencer<..., AnyLed, ...>
/// This allows them to be stored in a Vec and accessed individually by index,
/// while maintaining zero-cost abstraction.
struct HeterogeneousCollection<'t, const CAPACITY: usize> {
    sequencers: Vec<
        RgbSequencer<'t, EmbassyInstant, AnyLed<'t>, EmbassyTimeSource, SEQUENCE_STEP_SIZE>,
        CAPACITY
    >,
    time_source: &'t EmbassyTimeSource,
}

impl<'t, const CAPACITY: usize> HeterogeneousCollection<'t, CAPACITY> {
    /// Create a new empty collection
    fn new(time_source: &'t EmbassyTimeSource) -> Self {
        Self {
            sequencers: Vec::new(),
            time_source,
        }
    }
    
    /// Add a TIM1-based LED to the collection
    fn push_tim1(&mut self, led: EmbassyPwmRgbLed<'t, TIM1>) -> Result<usize, ()> {
        let any_led = AnyLed::Tim1(led);
        let sequencer = RgbSequencer::new(any_led, self.time_source);
        self.sequencers.push(sequencer).map_err(|_| ())?;
        Ok(self.sequencers.len() - 1)
    }
    
    /// Add a TIM3-based LED to the collection
    fn push_tim3(&mut self, led: EmbassyPwmRgbLed<'t, TIM3>) -> Result<usize, ()> {
        let any_led = AnyLed::Tim3(led);
        let sequencer = RgbSequencer::new(any_led, self.time_source);
        self.sequencers.push(sequencer).map_err(|_| ())?;
        Ok(self.sequencers.len() - 1)
    }
    
    /// Get a mutable reference to a sequencer by LED ID
    fn get_mut(&mut self, led_id: LedId) -> Option<&mut RgbSequencer<'t, EmbassyInstant, AnyLed<'t>, EmbassyTimeSource, SEQUENCE_STEP_SIZE>> {
        let index = match led_id {
            LedId::Led1 => 0,
            LedId::Led2 => 1,
        };
        self.sequencers.get_mut(index)
    }
    
    /// Service all running sequencers and return minimum delay
    fn service_all(&mut self) -> Option<EmbassyDuration> {
        let mut min_delay: Option<EmbassyDuration> = None;
        
        for sequencer in self.sequencers.iter_mut() {
            if !sequencer.is_running() {
                continue;
            }
            
            match sequencer.service() {
                Ok(Some(delay)) => {
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
                Ok(None) => {
                    info!("Sequence completed");
                }
                Err(e) => {
                    info!("Sequencer error: {:?}", e);
                }
            }
        }
        
        min_delay
    }
    
    /// Get number of LEDs in collection
    fn _len(&self) -> usize {
        self.sequencers.len()
    }
}

// ============================================================================
// RGB Task
// ============================================================================

#[embassy_executor::task]
pub async fn rgb_task(
    pwm_tim3: SimplePwm<'static, TIM3>,
    max_duty_tim3: u16,
    pwm_tim1: SimplePwm<'static, TIM1>,
    max_duty_tim1: u16,
) {
    info!("RGB task started");
    
    // Create LED wrappers (common anode = true)
    let led_tim3 = EmbassyPwmRgbLed::new(pwm_tim3, max_duty_tim3, true);
    let led_tim1 = EmbassyPwmRgbLed::new(pwm_tim1, max_duty_tim1, true);
    
    // Create time source
    let time_source = EmbassyTimeSource::new();
    
    // Create collection that can hold up to 4 LEDs
    let mut collection: HeterogeneousCollection<4> = HeterogeneousCollection::new(&time_source);
    
    // Add LEDs to collection in order: LED1 (TIM3), LED2 (TIM1)
    collection.push_tim3(led_tim3).unwrap();
    collection.push_tim1(led_tim1).unwrap();
    
    info!("Collection created with 2 LEDs");
    
    // Start with a short delay, will be updated after first service
    let mut next_service_delay = Duration::from_millis(16);
    
    loop {
        // Wait for either a command or the next service time
        match select(
            RGB_COMMAND_CHANNEL.receive(),
            Timer::after(next_service_delay)
        ).await {
            Either::First(command) => {
                handle_command(command, &mut collection);
                
                // Recalculate next service delay after command
                next_service_delay = service_and_get_delay(&mut collection);
            }
            Either::Second(_) => {
                // Time to service the sequencers
                next_service_delay = service_and_get_delay(&mut collection);
            }
        }
    }
}

/// Handle incoming commands
fn handle_command(
    command: RgbCommand,
    collection: &mut HeterogeneousCollection<4>,
) {
    match command {
        RgbCommand::Load { led_id, sequence } => {
            info!("Loading sequence on {:?}", led_id);
            if let Some(sequencer) = collection.get_mut(led_id) {
                sequencer.load(sequence);
            } else {
                info!("Invalid LED ID: {:?}", led_id);
            }
        }
        RgbCommand::Start { led_id } => {
            info!("Starting {:?}", led_id);
            if let Some(sequencer) = collection.get_mut(led_id) {
                if let Err(e) = sequencer.start() {
                    info!("Start error on {:?}: {:?}", led_id, e);
                }
            } else {
                info!("Invalid LED ID: {:?}", led_id);
            }
        }
        RgbCommand::Pause { led_id } => {
            info!("Pausing {:?}", led_id);
            if let Some(sequencer) = collection.get_mut(led_id) {
                if let Err(e) = sequencer.pause() {
                    info!("Pause error on {:?}: {:?}", led_id, e);
                }
            } else {
                info!("Invalid LED ID: {:?}", led_id);
            }
        }
        RgbCommand::Resume { led_id } => {
            info!("Resuming {:?}", led_id);
            if let Some(sequencer) = collection.get_mut(led_id) {
                if let Err(e) = sequencer.resume() {
                    info!("Resume error on {:?}: {:?}", led_id, e);
                }
            } else {
                info!("Invalid LED ID: {:?}", led_id);
            }
        }
        RgbCommand::GetColor { led_id, response } => {
            if let Some(sequencer) = collection.get_mut(led_id) {
                let color = sequencer.current_color();
                info!("Color query for {:?}: R={} G={} B={}", 
                      led_id, color.red, color.green, color.blue);
                response.signal(color);
            } else {
                info!("Invalid LED ID: {:?}", led_id);
            }
        }
    }
}

/// Service all sequencers and return the appropriate delay.
fn service_and_get_delay(collection: &mut HeterogeneousCollection<4>) -> Duration {
    match collection.service_all() {
        Some(delay) if delay.0.as_millis() == 0 => {
            // Linear transition - service at ~60fps
            Duration::from_millis(16)
        }
        Some(delay) => {
            // Step transition - use the delay
            delay.0
        }
        None => {
            // All sequences complete - wait indefinitely
            Duration::from_secs(3600)
        }
    }
}
