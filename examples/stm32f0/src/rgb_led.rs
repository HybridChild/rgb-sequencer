use embedded_hal::PwmPin;
use palette::Srgb;
use rgb_sequencer::RgbLed;

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

    /// Convert float (0.0-1.0) to PWM duty cycle
    /// Handles common anode inversion automatically
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

// Implement the RgbLed trait required by the sequencer
impl<R, G, B> RgbLed for PwmRgbLed<R, G, B>
where
    R: PwmPin<Duty = u16>,
    G: PwmPin<Duty = u16>,
    B: PwmPin<Duty = u16>,
{
    fn set_color(&mut self, color: Srgb) {
        // Convert 0.0-1.0 float values to duty cycles
        self.red.set_duty(self.float_to_duty(color.red));
        self.green.set_duty(self.float_to_duty(color.green));
        self.blue.set_duty(self.float_to_duty(color.blue));
    }
}
