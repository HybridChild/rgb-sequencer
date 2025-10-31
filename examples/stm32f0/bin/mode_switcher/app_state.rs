use rtt_target::rprintln;
use stm32f0xx_hal::prelude::*;

use stm32f0_examples::time_source::{HalTimeSource, HalInstant, HalDuration};
use rgb_sequencer::{RgbSequencer, SequencerState, TimeDuration, TimeSource};

use crate::button::ButtonDebouncer;
use crate::hardware_setup::{HardwareContext, Led1};
use crate::sequences::{create_breathing_sequence, create_rainbow_sequence, create_police_sequence};

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
}

impl Mode {
    /// Get the next mode in the cycle
    pub fn next(&self) -> Self {
        match self {
            Mode::Breathing => Mode::Rainbow,
            Mode::Rainbow => Mode::Police,
            Mode::Police => Mode::Breathing,
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
            Mode::Breathing => create_breathing_sequence(),  // Now uses function-based sine wave!
            Mode::Rainbow => create_rainbow_sequence(),
            Mode::Police => create_police_sequence(),
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
                // Mode 3: LED on
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

    /// Service sequencer and return the minimum delay needed
    fn service_sequencer(&mut self) -> Option<HalDuration> {
        let state = self.sequencer.get_state();
        
        let mut min_delay: Option<HalDuration> = None;
        
        // Service RGB LED
        if state == SequencerState::Running {
            match self.sequencer.service() {
                Ok(Some(delay)) => {
                    min_delay = Some(delay);
                }
                Ok(None) => {
                    rprintln!("RGB LED sequence completed");
                }
                Err(e) => {
                    rprintln!("RGB LED sequencer error: {:?}", e);
                }
            }
        }
        
        min_delay
    }

    /// Check for button press and handle it
    fn is_button_pressed(&mut self) -> bool {
        let button_is_low = self.button.is_low().unwrap();
        let current_time = self.time_source.now();

        self.button_debouncer.check_press(button_is_low, current_time.as_millis())
    }

    /// Sleep until next service time is needed
    fn sleep_until_next_service(&self, delay: Option<HalDuration>) {
        if let Some(delay) = delay {
            let delay_ms = delay.as_millis();
            if delay_ms > 0 {
                // For step transitions, use WFI
                cortex_m::asm::wfi();
            } else {
                // For linear transitions, target ~60fps (16ms)
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
        } else {
            // RGB LED paused or completed - just sleep and let interrupts wake us
            cortex_m::asm::wfi();
        }
    }

    /// Run the main application loop
    pub fn run(&mut self) -> ! {
        loop {
            // Check for button press
            if self.is_button_pressed() {
                self.handle_button_press();
            }

            // Service sequencers
            let delay = self.service_sequencer();
            
            // Sleep until next service needed
            self.sleep_until_next_service(delay);
        }
    }
}
