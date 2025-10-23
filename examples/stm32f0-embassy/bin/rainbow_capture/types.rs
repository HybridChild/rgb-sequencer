use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use rgb_sequencer::RgbSequence;
use palette::Srgb;

// Re-export the time types from the library
pub use stm32f0_embassy::time_wrapper::{EmbassyDuration, EmbassyInstant};

/// Which LED to target with a command
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum LedId {
    Led1,
    Led2,
}

/// Commands that can be sent to the RGB task
pub enum RgbCommand {
    /// Load a new sequence on a specific LED
    Load {
        led_id: LedId,
        sequence: RgbSequence<EmbassyDuration, SEQUENCE_STEP_SIZE>,
    },
    
    /// Start a specific LED's sequence
    Start {
        led_id: LedId,
    },
    
    /// Pause a specific LED's sequence
    Pause {
        led_id: LedId,
    },
    
    /// Resume a specific LED's sequence
    Resume {
        led_id: LedId,
    },
    
    /// Query the current color of a specific LED
    /// Response will be sent via the provided signal
    GetColor {
        led_id: LedId,
        response: &'static Signal<ThreadModeRawMutex, Srgb>,
    },
}

/// Signal from button_task to app_logic_task when button is pressed
pub static BUTTON_SIGNAL: Signal<ThreadModeRawMutex, ()> = Signal::new();

/// Channel for sending commands from app_logic_task to rgb_task
pub static RGB_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, RgbCommand, 4> = Channel::new();

/// Signal for receiving color query responses
pub static COLOR_RESPONSE_SIGNAL: Signal<ThreadModeRawMutex, Srgb> = Signal::new();

/// Maximum number of steps that can be stored in a sequence
pub const SEQUENCE_STEP_SIZE: usize = 8;
