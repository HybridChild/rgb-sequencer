/// Simple button debouncer for handling button press detection
pub struct ButtonDebouncer {
    pressed: bool,
    last_press_time: u32,
    debounce_ms: u32,
}

impl ButtonDebouncer {
    /// Create a new button debouncer
    ///
    /// # Arguments
    /// * `debounce_ms` - Minimum time between presses in milliseconds
    pub fn new(debounce_ms: u32) -> Self {
        Self {
            pressed: false,
            last_press_time: 0,
            debounce_ms,
        }
    }

    /// Check if button was just pressed (returns true on falling edge)
    ///
    /// # Arguments
    /// * `button_is_low` - Current state of the button (true if pressed)
    /// * `current_time_ms` - Current time in milliseconds
    ///
    /// # Returns
    /// `true` if a valid button press was detected (after debounce period)
    pub fn check_press(&mut self, button_is_low: bool, current_time_ms: u32) -> bool {
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
