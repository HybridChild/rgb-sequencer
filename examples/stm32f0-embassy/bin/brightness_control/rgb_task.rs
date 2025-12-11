use defmt::info;
use embassy_futures::select::{Either, select};
use embassy_stm32::peripherals::TIM3;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_time::{Duration, Timer};
use palette::Srgb;
use rgb_sequencer::{
    LoopCount, RgbLed, RgbSequence8, RgbSequencer8, ServiceTiming, TransitionStyle,
};

use crate::types::{EmbassyDuration, EmbassyTimeSource, RGB_COMMAND_CHANNEL, RgbCommand};

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
// Rainbow Sequence Creation
// ============================================================================

/// Create a colorful rainbow sequence that cycles through hues
fn create_rainbow_sequence() -> RgbSequence8<EmbassyDuration> {
    use rgb_sequencer::colors::hue;

    RgbSequence8::builder()
        .step(
            hue(0.0), // Red
            EmbassyDuration(Duration::from_millis(2000)),
            TransitionStyle::Linear,
        )
        .unwrap()
        .step(
            hue(60.0), // Yellow
            EmbassyDuration(Duration::from_millis(2000)),
            TransitionStyle::Linear,
        )
        .unwrap()
        .step(
            hue(120.0), // Green
            EmbassyDuration(Duration::from_millis(2000)),
            TransitionStyle::Linear,
        )
        .unwrap()
        .step(
            hue(180.0), // Cyan
            EmbassyDuration(Duration::from_millis(2000)),
            TransitionStyle::Linear,
        )
        .unwrap()
        .step(
            hue(240.0), // Blue
            EmbassyDuration(Duration::from_millis(2000)),
            TransitionStyle::Linear,
        )
        .unwrap()
        .step(
            hue(300.0), // Magenta
            EmbassyDuration(Duration::from_millis(2000)),
            TransitionStyle::Linear,
        )
        .unwrap()
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap()
}

#[embassy_executor::task]
pub async fn rgb_task(pwm: SimplePwm<'static, TIM3>, max_duty: u16) {
    info!("Starting RGB task...");

    // Create LED wrapper (common anode = true)
    let led = EmbassyPwmRgbLed::new(pwm, max_duty, true);

    // Create time source
    static TIME_SOURCE: EmbassyTimeSource = EmbassyTimeSource;

    // Create sequencer
    let mut sequencer = RgbSequencer8::new(led, &TIME_SOURCE);

    // Load and start the rainbow sequence
    let sequence = create_rainbow_sequence();
    info!("Loading rainbow sequence...");
    sequencer.load_and_start(sequence).unwrap();

    info!("RGB task ready - running rainbow sequence");

    loop {
        // Select between receiving commands and servicing the sequencer
        match select(
            RGB_COMMAND_CHANNEL.receive(),
            Timer::after_millis(16), // ~60 FPS
        )
        .await
        {
            Either::First(command) => {
                // Handle command
                match command {
                    RgbCommand::SetBrightness(level) => {
                        let brightness_value = level.value();
                        info!("Setting brightness to {}", brightness_value);
                        sequencer.set_brightness(brightness_value);
                    }
                }
            }
            Either::Second(_) => {
                // Service the sequencer
                if sequencer.is_running() {
                    match sequencer.service() {
                        Ok(ServiceTiming::Continuous) => {
                            // Continue at current frame rate
                        }
                        Ok(ServiceTiming::Delay(delay)) => {
                            // Could optimize by sleeping for the delay,
                            // but we keep it simple at 60 FPS for demo
                            let _ = delay;
                        }
                        Ok(ServiceTiming::Complete) => {
                            info!("Sequence completed");
                        }
                        Err(e) => {
                            info!("Service error: {:?}", e);
                        }
                    }
                }
            }
        }
    }
}
