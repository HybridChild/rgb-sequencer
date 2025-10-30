use rtt_target::rprintln;
use stm32f0xx_hal::prelude::*;

use stm32f0_examples::time_source::{HalTimeSource, HalInstant, HalDuration};
use rgb_sequencer::{RgbSequencer, RgbSequence, TransitionStyle, SequencerState, TimeDuration, TimeSource, COLOR_OFF};

use crate::button::ButtonDebouncer;
use crate::hardware_setup::{HardwareContext, Led1, Led2};
use crate::sequences::{create_rainbow_sequence, SEQUENCE_STEP_SIZE};

/// Type aliases for the sequencers
type Sequencer1<'a> = RgbSequencer<'a, HalInstant, Led1, HalTimeSource, SEQUENCE_STEP_SIZE>;
type Sequencer2<'a> = RgbSequencer<'a, HalInstant, Led2, HalTimeSource, SEQUENCE_STEP_SIZE>;

/// Application state containing all runtime data
pub struct AppState<'a> {
    sequencer_1: Sequencer1<'a>,
    sequencer_2: Sequencer2<'a>,
    button: crate::hardware_setup::Button,
    button_debouncer: ButtonDebouncer,
    time_source: &'a HalTimeSource,
}

impl<'a> AppState<'a> {
    /// Initialize the application with hardware and sequences
    pub fn new(hw: HardwareContext, time_source: &'a HalTimeSource) -> Self {
        // Create sequencers
        let mut sequencer_1 = RgbSequencer::new(hw.led_1, time_source);
        let mut sequencer_2 = RgbSequencer::new(hw.led_2, time_source);

        // Create and load sequences
        let sequence_1 = create_rainbow_sequence();
        let sequence_2 = RgbSequence::new()
            .step(COLOR_OFF, HalDuration(0), TransitionStyle::Step)
            .build()
            .unwrap();
        
        sequencer_1.load(sequence_1);
        sequencer_1.start().unwrap();
        
        sequencer_2.load(sequence_2);
        sequencer_2.start().unwrap();

        rprintln!("Both sequences started!");
        rprintln!("LED 1: Rainbow animation (red -> green -> blue)");
        rprintln!("LED 2: Off (will capture colors when button pressed)");
        rprintln!("Press the user button to pause LED 1 and capture color to LED 2");

        Self {
            sequencer_1,
            sequencer_2,
            button: hw.button,
            button_debouncer: ButtonDebouncer::new(200),
            time_source,
        }
    }

    /// Handle button press - toggle pause/resume and color capture
    fn handle_button_press(&mut self) {
        match self.sequencer_1.get_state() {
            SequencerState::Running => {
                rprintln!("Pausing LED 1 and capturing color to LED 2");
                
                if let Err(e) = self.sequencer_1.pause() {
                    rprintln!("Pause error LED 1: {:?}", e);
                    return;
                }

                // Capture color and create smooth transition
                let old_color = self.sequencer_2.current_color();
                let captured_color = self.sequencer_1.current_color();
                
                rprintln!("Captured: R={:.2} G={:.2} B={:.2}", 
                         captured_color.red, captured_color.green, captured_color.blue);
                
                let transition_sequence = RgbSequence::new()
                    .start_color(old_color)
                    .step(captured_color, HalDuration(2000), TransitionStyle::Linear)
                    .build()
                    .unwrap();
                
                self.sequencer_2.load(transition_sequence);
                self.sequencer_2.start().unwrap();
            }
            SequencerState::Paused => {
                rprintln!("Resuming LED 1 animation");
                
                if let Err(e) = self.sequencer_1.resume() {
                    rprintln!("Resume error LED 1: {:?}", e);
                }
            }
            _ => {
                rprintln!("Cannot pause/resume from state: {:?}", self.sequencer_1.get_state());
            }
        }
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
            // Both paused - just sleep and let interrupts wake us
            cortex_m::asm::wfi();
        }
    }

    /// Run the main application loop
    pub fn run(&mut self) -> ! {
        rprintln!("=== System Ready ===");

        loop {
            // Check for button press
            if self.is_button_pressed() {
                self.handle_button_press();
            }

            // Service sequencers
            let delay = self.service_sequencers();
            
            // Sleep until next service needed
            self.sleep_until_next_service(delay);
        }
    }
}
