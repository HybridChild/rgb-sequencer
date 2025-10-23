use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use rgb_sequencer::RgbSequence;

// Re-export the time types from the library
pub use stm32f0_embassy::time_wrapper::{EmbassyDuration, EmbassyInstant};

/// Operating modes for the RGB LEDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
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

/// Commands that can be sent to the RGB task
pub enum RgbCommand {
    /// Load a new sequence on both LEDs (coordinated)
    LoadCoordinated(RgbSequence<EmbassyDuration, SEQUENCE_STEP_SIZE>),
}

/// Signal from button_task to app_logic_task when button is pressed
pub static BUTTON_SIGNAL: Signal<ThreadModeRawMutex, ()> = Signal::new();

/// Channel for sending commands from app_logic_task to rgb_task
pub static RGB_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, RgbCommand, 2> = Channel::new();

/// Maximum number of steps that can be stored in a sequence
pub static SEQUENCE_STEP_SIZE: usize = 8;
