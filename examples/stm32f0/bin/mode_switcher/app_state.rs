use rtt_target::rprintln;
use stm32f0xx_hal::prelude::*;

use rgb_sequencer::{RgbSequencer, ServiceTiming, TimeSource};
use stm32f0::time_source::{HalDuration, HalInstant, HalTimeSource};

use crate::button::ButtonDebouncer;
use crate::hardware_setup::{HardwareContext, Led1};
use crate::sequences::{
    create_breathing_sequence, create_flame_sequence, create_police_sequence,
    create_rainbow_sequence,
};

/// Type aliases for the sequencers
type Sequencer<'a> = RgbSequencer<'a, HalInstant, Led1, HalTimeSource, 16>;

/// Operating modes for the RGB LED
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Slow breathing white effect (using sine wave function)
    Breathing,
    /// Rainbow color cycle
    Rainbow,
    /// Red/Blue alternating police lights effect
    Police,
    /// Flickering flame effect
    Flame,
}

impl Mode {
    /// Get the next mode in the cycle
    pub fn next(&self) -> Self {
        match self {
            Mode::Breathing => Mode::Rainbow,
            Mode::Rainbow => Mode::Police,
            Mode::Police => Mode::Flame,
            Mode::Flame => Mode::Breathing,
        }
    }
}

/// Application state containing all runtime data
pub struct AppState<'a> {
    sequencer: Sequencer<'a>,
    button: crate::hardware_setup::Button,
    onboard_led: crate::hardware_setup::OnboardLed,
    button_debouncer: ButtonDebouncer,
    time_source: &'a HalTimeSource,
    current_mode: Mode,
}

impl<'a> AppState<'a> {
    /// Initialize the application with hardware and initial mode
    pub fn new(hw: HardwareContext, time_source: &'a HalTimeSource) -> Self {
        // Create sequencers
        let mut sequencer = RgbSequencer::new(hw.led_1, time_source);

        // Start with Rainbow mode
        let initial_mode = Mode::Rainbow;
        let sequence = create_rainbow_sequence();

        sequencer.load(sequence);
        sequencer.start().unwrap();

        rprintln!("Initial mode: {:?}", initial_mode);

        Self {
            sequencer,
            button: hw.button,
            onboard_led: hw.onboard_led,
            button_debouncer: ButtonDebouncer::new(200),
            time_source,
            current_mode: initial_mode,
        }
    }

    /// Load a new mode to the sequencer
    fn load_mode(&mut self, mode: Mode) {
        rprintln!("Switching to mode: {:?}", mode);

        let sequence = match mode {
            Mode::Breathing => create_breathing_sequence(), // Now uses function-based sine wave!
            Mode::Rainbow => create_rainbow_sequence(),
            Mode::Police => create_police_sequence(),
            Mode::Flame => create_flame_sequence(),
        };

        // Load and start the sequencer
        self.sequencer.load(sequence);
        self.sequencer.start().unwrap();

        // Update onboard LED indicator
        self.update_mode_indicator(mode);

        self.current_mode = mode;
    }

    /// Update the onboard LED to indicate the current mode
    fn update_mode_indicator(&mut self, mode: Mode) {
        match mode {
            Mode::Breathing => {
                // Mode 1: LED off
                self.onboard_led.set_low().unwrap();
            }
            Mode::Rainbow => {
                // Mode 2: LED on
                self.onboard_led.set_high().unwrap();
            }
            Mode::Police => {
                // Mode 3: LED off
                self.onboard_led.set_low().unwrap();
            }
            Mode::Flame => {
                // Mode 4: LED on
                self.onboard_led.set_high().unwrap();
            }
        }
    }

    /// Handle button press - cycle to next mode
    fn handle_button_press(&mut self) {
        rprintln!("Button press detected - cycling mode");
        let next_mode = self.current_mode.next();
        self.load_mode(next_mode);
    }

    /// Service sequencer and return timing
    fn service_sequencer(&mut self) -> ServiceTiming<HalDuration> {
        if self.sequencer.is_running() {
            match self.sequencer.service() {
                Ok(timing) => timing,
                Err(e) => {
                    rprintln!("RGB LED sequencer error: {:?}", e);
                    ServiceTiming::Complete
                }
            }
        } else {
            ServiceTiming::Complete
        }
    }

    /// Check for button press and handle it
    fn is_button_pressed(&mut self) -> bool {
        let button_is_low = self.button.is_low().unwrap();
        let current_time = self.time_source.now();

        self.button_debouncer
            .check_press(button_is_low, current_time.as_millis())
    }

    /// Sleep until next service time is needed
    fn sleep_until_next_service(&self, timing: ServiceTiming<HalDuration>) {
        match timing {
            ServiceTiming::Continuous => {
                // Continuous animation - target ~60fps (16ms)
                let current_time = self.time_source.now();
                let target_time = current_time.as_millis().wrapping_add(16);
                loop {
                    cortex_m::asm::wfi();
                    let now = self.time_source.now();
                    if now.as_millis().wrapping_sub(target_time) < 0x8000_0000 {
                        break;
                    }
                }
            }
            ServiceTiming::Delay(_delay) => {
                // Step transition - use WFI (interrupt will wake us for next step)
                cortex_m::asm::wfi();
            }
            ServiceTiming::Complete => {
                // RGB LED paused or completed - just sleep and let interrupts wake us
                cortex_m::asm::wfi();
            }
        }
    }

    /// Run the main application loop
    pub fn run(&mut self) -> ! {
        loop {
            // Check for button press
            if self.is_button_pressed() {
                self.handle_button_press();
            }

            // Service sequencer and sleep until next service needed
            let timing = self.service_sequencer();
            self.sleep_until_next_service(timing);
        }
    }
}
