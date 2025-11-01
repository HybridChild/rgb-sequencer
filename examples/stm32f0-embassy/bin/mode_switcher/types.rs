use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use rgb_sequencer::RgbSequence;

// Re-export the time types from the library
pub use stm32f0_embassy::time_wrapper::{EmbassyDuration, EmbassyInstant, EmbassyTimeSource};

/// Operating modes for the RGB LED
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Mode {
    /// Rainbow color cycle
    Rainbow,
    /// Red/Blue alternating police lights effect
    Police,
    /// Slow breathing white effect
    Breathing,
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
    /// Load a new sequence
    Load(RgbSequence<EmbassyDuration, SEQUENCE_STEP_CAPACITY>),
}

/// Signal from button_task to app_logic_task when button is pressed
pub static BUTTON_SIGNAL: Signal<ThreadModeRawMutex, ()> = Signal::new();

/// Channel for sending commands from app_logic_task to rgb_task
pub static RGB_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, RgbCommand, 2> = Channel::new();

/// Maximum number of steps that can be stored in a sequence
pub const SEQUENCE_STEP_CAPACITY: usize = 8;
