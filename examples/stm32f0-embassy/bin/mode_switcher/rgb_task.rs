use defmt::info;
use embassy_futures::select::{Either, select};
use embassy_stm32::peripherals::TIM3;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_time::{Duration, Timer};
use palette::Srgb;
use rgb_sequencer::{RgbLed, RgbSequencer8, ServiceTiming};

use crate::types::{EmbassyInstant, EmbassyTimeSource, RGB_COMMAND_CHANNEL};

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
// RGB Task
// ============================================================================

#[embassy_executor::task]
pub async fn rgb_task(pwm_tim3: SimplePwm<'static, TIM3>, max_duty_tim3: u16) {
    info!("RGB task started");

    // Create LED wrapper (common anode = true)
    let led_1 = EmbassyPwmRgbLed::new(pwm_tim3, max_duty_tim3, true);

    // Create time source
    let time_source = EmbassyTimeSource::new();

    // Create sequencer
    let mut sequencer = RgbSequencer8::<EmbassyInstant, _, _>::new(led_1, &time_source);

    info!("Sequencer created");

    // Start with a short delay, will be updated after first service
    let mut next_service_delay = Duration::from_millis(16);

    loop {
        // Wait for either a command or the next service time
        match select(
            RGB_COMMAND_CHANNEL.receive(),
            Timer::after(next_service_delay),
        )
        .await
        {
            Either::First(command) => {
                info!("Handling command");

                // Use the library's handle_action method!
                // The Load action will load and start the sequence
                match command.action {
                    rgb_sequencer::SequencerAction8::Load(seq) => {
                        if let Err(e) = sequencer.load_and_start(seq) {
                            info!("Load and start error: {:?}", e);
                        }
                    }
                    _ => {
                        if let Err(e) = sequencer.handle_action(command.action) {
                            info!("Action error: {:?}", e);
                        }
                    }
                }

                // Service to set color and get delay
                next_service_delay = service_and_get_delay(&mut sequencer);
            }
            Either::Second(_) => {
                // Time to service the sequencer
                next_service_delay = service_and_get_delay(&mut sequencer);
            }
        }
    }
}

/// Service the sequencer and return the appropriate delay.
fn service_and_get_delay(
    sequencer: &mut RgbSequencer8<EmbassyInstant, EmbassyPwmRgbLed<TIM3>, EmbassyTimeSource>,
) -> Duration {
    if !sequencer.is_running() {
        return Duration::from_secs(3600);
    }

    match sequencer.service() {
        Ok(ServiceTiming::Continuous) => {
            // Linear transition - service at ~60fps
            Duration::from_millis(16)
        }
        Ok(ServiceTiming::Delay(delay)) => {
            // Step transition - use the delay
            delay.0
        }
        Ok(ServiceTiming::Complete) => {
            info!("Sequence completed");
            // Sequence complete - wait indefinitely - One hour for simplicity
            Duration::from_secs(3600)
        }
        Err(e) => {
            info!("Sequencer error: {:?}", e);
            Duration::from_secs(3600)
        }
    }
}
