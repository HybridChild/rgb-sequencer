use rtt_target::rprintln;
use stm32f0xx_hal::prelude::*;

use stm32f0_examples::time_source::{HalTimeSource, HalInstant, HalDuration};
use rgb_sequencer::{RgbSequencer, SequencerState, TimeDuration, TimeSource};

use crate::button::ButtonDebouncer;
use crate::hardware_setup::{HardwareContext, Led1, Led2};
use crate::sequences::{create_breathing_sequence, create_rainbow_sequence, create_police_sequence};

/// Type aliases for the sequencers
type Sequencer1<'a> = RgbSequencer<'a, HalInstant, Led1, HalTimeSource, 16>;
type Sequencer2<'a> = RgbSequencer<'a, HalInstant, Led2, HalTimeSource, 16>;

/// Operating modes for the RGB LEDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Slow breathing white effect on both LEDs
    Breathing,
    /// Rainbow color cycle on both LEDs
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
    sequencer_1: Sequencer1<'a>,
    sequencer_2: Sequencer2<'a>,
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
        let mut sequencer_1 = RgbSequencer::new(hw.led_1, time_source);
        let mut sequencer_2 = RgbSequencer::new(hw.led_2, time_source);

        // Start with Rainbow mode
        let initial_mode = Mode::Rainbow;
        let sequence = create_rainbow_sequence();
        
        sequencer_1.load(sequence.clone());
        sequencer_1.start().unwrap();
        
        sequencer_2.load(sequence);
        sequencer_2.start().unwrap();

        rprintln!("Initial mode: {:?}", initial_mode);
        rprintln!("Both LEDs synchronized in rainbow mode");

        Self {
            sequencer_1,
            sequencer_2,
            button: hw.button,
            onboard_led: hw.onboard_led,
            button_debouncer: ButtonDebouncer::new(200),
            time_source,
            current_mode: initial_mode,
        }
    }

    /// Load a new mode on both LEDs (coordinated)
    fn load_mode(&mut self, mode: Mode) {
        rprintln!("Switching to mode: {:?}", mode);
        
        let sequence = match mode {
            Mode::Breathing => create_breathing_sequence(),
            Mode::Rainbow => create_rainbow_sequence(),
            Mode::Police => create_police_sequence(),
        };
        
        // Load and start on both LEDs
        self.sequencer_1.load(sequence.clone());
        self.sequencer_1.start().unwrap();
        
        self.sequencer_2.load(sequence);
        self.sequencer_2.start().unwrap();
        
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

    /// Service both sequencers and return the minimum delay needed
    fn service_sequencers(&mut self) -> Option<HalDuration> {
        let state_1 = self.sequencer_1.get_state();
        let state_2 = self.sequencer_2.get_state();
        
        let mut min_delay: Option<HalDuration> = None;
        
        // Service LED 1
        if state_1 == SequencerState::Running {
            match self.sequencer_1.service() {
                Ok(Some(delay)) => {
                    min_delay = Some(Self::min_duration(min_delay, delay));
                }
                Ok(None) => {
                    rprintln!("LED 1 sequence completed");
                }
                Err(e) => {
                    rprintln!("LED 1 sequencer error: {:?}", e);
                }
            }
        }
        
        // Service LED 2
        if state_2 == SequencerState::Running {
            match self.sequencer_2.service() {
                Ok(Some(delay)) => {
                    min_delay = Some(Self::min_duration(min_delay, delay));
                }
                Ok(None) => {
                    rprintln!("LED 2 sequence completed");
                }
                Err(e) => {
                    rprintln!("LED 2 sequencer error: {:?}", e);
                }
            }
        }
        
        min_delay
    }

    /// Helper to find minimum duration
    fn min_duration(current: Option<HalDuration>, new: HalDuration) -> HalDuration {
        match current {
            None => new,
            Some(curr) => {
                if new.as_millis() < curr.as_millis() {
                    new
                } else {
                    curr
                }
            }
        }
    }

    /// Check for button press and handle it
    fn check_button(&mut self) {
        let button_is_low = self.button.is_low().unwrap();
        let current_time = self.time_source.now();
        if self.button_debouncer.check_press(button_is_low, current_time.as_millis()) {
            self.handle_button_press();
        }
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
            // Both paused - just sleep and let interrupts wake us
            cortex_m::asm::wfi();
        }
    }

    /// Run the main application loop
    pub fn run(&mut self) -> ! {
        loop {
            // Check for button press
            self.check_button();

            // Service sequencers
            let delay = self.service_sequencers();
            
            // Sleep until next service needed
            self.sleep_until_next_service(delay);
        }
    }
}
