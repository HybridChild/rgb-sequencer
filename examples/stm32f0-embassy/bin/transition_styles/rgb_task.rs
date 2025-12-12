use defmt::info;
use embassy_futures::select::{Either, select};
use embassy_stm32::peripherals::TIM3;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_time::{Duration, Timer};
use palette::Srgb;
use rgb_sequencer::{RgbLed, RgbSequencer, ServiceTiming};
use stm32f0_embassy::time_wrapper::{EmbassyInstant, EmbassyTimeSource};

use crate::types::RGB_COMMAND_CHANNEL;

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
pub async fn rgb_task(pwm_tim3: SimplePwm<'static, TIM3>, max_duty_tim3: u16) {
    info!("RGB task started");

    let led_1 = EmbassyPwmRgbLed::new(pwm_tim3, max_duty_tim3, true);
    let time_source = EmbassyTimeSource::new();
    let mut sequencer = RgbSequencer::<EmbassyInstant, _, _, 16>::new(led_1, &time_source);

    info!("Sequencer created");

    let mut next_service_delay = Duration::from_millis(16);

    loop {
        match select(
            RGB_COMMAND_CHANNEL.receive(),
            Timer::after(next_service_delay),
        )
        .await
        {
            Either::First(command) => {
                info!("Handling command");

                match command.action {
                    rgb_sequencer::SequencerAction16::Load(seq) => {
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

                next_service_delay = service_and_get_delay(&mut sequencer);
            }
            Either::Second(_) => {
                next_service_delay = service_and_get_delay(&mut sequencer);
            }
        }
    }
}

fn service_and_get_delay(
    sequencer: &mut RgbSequencer<EmbassyInstant, EmbassyPwmRgbLed<TIM3>, EmbassyTimeSource, 16>,
) -> Duration {
    if !sequencer.is_running() {
        return Duration::from_secs(3600);
    }

    match sequencer.service() {
        Ok(ServiceTiming::Continuous) => Duration::from_millis(16),
        Ok(ServiceTiming::Delay(delay)) => delay.0,
        Ok(ServiceTiming::Complete) => {
            info!("Sequence completed");
            Duration::from_secs(3600)
        }
        Err(e) => {
            info!("Sequencer error: {:?}", e);
            Duration::from_secs(3600)
        }
    }
}
