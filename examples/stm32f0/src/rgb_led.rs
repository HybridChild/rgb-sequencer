use embedded_hal::PwmPin;
use rgb_sequencer::{Rgb, RgbLed};

/// RGB LED implementation for PWM-controlled LEDs
///
/// This wrapper implements the RgbLed trait required by the sequencer,
/// handling PWM duty cycle conversion and common anode/cathode logic.
pub struct PwmRgbLed<R, G, B>
where
    R: PwmPin<Duty = u16>,
    G: PwmPin<Duty = u16>,
    B: PwmPin<Duty = u16>,
{
    red: R,
    green: G,
    blue: B,
    max_duty: u16,
    common_anode: bool,
}

impl<R, G, B> PwmRgbLed<R, G, B>
where
    R: PwmPin<Duty = u16>,
    G: PwmPin<Duty = u16>,
    B: PwmPin<Duty = u16>,
{
    /// Create a new RGB LED controller
    ///
    /// # Arguments
    /// * `red` - PWM channel for red LED
    /// * `green` - PWM channel for green LED
    /// * `blue` - PWM channel for blue LED
    /// * `common_anode` - true for common anode LED (inverted logic), false for common cathode
    pub fn new(mut red: R, mut green: G, mut blue: B, common_anode: bool) -> Self {
        let max_duty = red.get_max_duty();

        // Enable all channels
        red.enable();
        green.enable();
        blue.enable();

        Self {
            red,
            green,
            blue,
            max_duty,
            common_anode,
        }
    }

    /// Convert a 0..=65535 channel value to a PWM duty cycle.
    /// Handles common anode inversion automatically.
    ///
    /// Integer math only: the product fits a u32 for any max_duty, so this
    /// costs a multiply and a divide rather than pulling in soft-float.
    fn channel_to_duty(&self, value: u16) -> u16 {
        let duty = (value as u32 * self.max_duty as u32 / 65535) as u16;

        if self.common_anode {
            self.max_duty - duty
        } else {
            duty
        }
    }
}

// Implement the RgbLed trait required by the sequencer
impl<R, G, B> RgbLed for PwmRgbLed<R, G, B>
where
    R: PwmPin<Duty = u16>,
    G: PwmPin<Duty = u16>,
    B: PwmPin<Duty = u16>,
{
    fn set_color(&mut self, color: Rgb) {
        // Convert 0..=65535 channel values to duty cycles
        self.red.set_duty(self.channel_to_duty(color.r));
        self.green.set_duty(self.channel_to_duty(color.g));
        self.blue.set_duty(self.channel_to_duty(color.b));
    }
}
