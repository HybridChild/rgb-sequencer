use rtt_target::rprintln;
use stm32f0xx_hal::prelude::*;

use stm32f0_examples::time_source::{HalTimeSource, HalInstant, HalDuration};
use rgb_sequencer::{RgbSequencer, RgbSequence, ServiceTiming, TransitionStyle, SequencerState, TimeDuration, TimeSource, COLOR_OFF};

use crate::button::ButtonDebouncer;
use crate::hardware_setup::{HardwareContext, Led1, Led2};
use crate::sequences::{create_rainbow_sequence, SEQUENCE_STEP_CAPACITY};

/// Type aliases for the sequencers
type Sequencer1<'a> = RgbSequencer<'a, HalInstant, Led1, HalTimeSource, SEQUENCE_STEP_CAPACITY>;
type Sequencer2<'a> = RgbSequencer<'a, HalInstant, Led2, HalTimeSource, SEQUENCE_STEP_CAPACITY>;

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

    /// Service both sequencers and return the most urgent timing
    fn service_sequencers(&mut self) -> ServiceTiming<HalDuration> {
        let state_1 = self.sequencer_1.get_state();
        let state_2 = self.sequencer_2.get_state();

        let mut result = ServiceTiming::Complete;

        // Service LED 1
        if state_1 == SequencerState::Running {
            match self.sequencer_1.service() {
                Ok(timing) => {
                    result = Self::most_urgent(result, timing);
                    if matches!(timing, ServiceTiming::Complete) {
                        rprintln!("LED 1 sequence completed");
                    }
                }
                Err(e) => {
                    rprintln!("LED 1 sequencer error: {:?}", e);
                }
            }
        }

        // Service LED 2
        if state_2 == SequencerState::Running {
            match self.sequencer_2.service() {
                Ok(timing) => {
                    result = Self::most_urgent(result, timing);
                    if matches!(timing, ServiceTiming::Complete) {
                        rprintln!("LED 2 sequence completed");
                    }
                }
                Err(e) => {
                    rprintln!("LED 2 sequencer error: {:?}", e);
                }
            }
        }

        result
    }

    /// Helper to find the most urgent timing between two ServiceTimings
    fn most_urgent(a: ServiceTiming<HalDuration>, b: ServiceTiming<HalDuration>) -> ServiceTiming<HalDuration> {
        match (a, b) {
            // Continuous is always most urgent
            (ServiceTiming::Continuous, _) | (_, ServiceTiming::Continuous) => ServiceTiming::Continuous,
            // Between two delays, choose the shorter one
            (ServiceTiming::Delay(d1), ServiceTiming::Delay(d2)) => {
                if d1.as_millis() < d2.as_millis() {
                    ServiceTiming::Delay(d1)
                } else {
                    ServiceTiming::Delay(d2)
                }
            }
            // If one is Delay and other is Complete, use Delay
            (ServiceTiming::Delay(d), _) | (_, ServiceTiming::Delay(d)) => ServiceTiming::Delay(d),
            // Both Complete
            _ => ServiceTiming::Complete,
        }
    }

    /// Check for button press and handle it
    fn is_button_pressed(&mut self) -> bool {
        let button_is_low = self.button.is_low().unwrap();
        let current_time = self.time_source.now();
        
        self.button_debouncer.check_press(button_is_low, current_time.as_millis())
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
            ServiceTiming::Delay(_) => {
                // Step transition - use WFI (interrupt will wake us for next step)
                cortex_m::asm::wfi();
            }
            ServiceTiming::Complete => {
                // Both paused or complete - just sleep and let interrupts wake us
                cortex_m::asm::wfi();
            }
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
            let timing = self.service_sequencers();

            // Sleep until next service needed
            self.sleep_until_next_service(timing);
        }
    }
}
