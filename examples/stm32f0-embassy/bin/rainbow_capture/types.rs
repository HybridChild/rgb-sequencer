use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use palette::Srgb;
use rgb_sequencer::SequencerCommand;

// Re-export the time types from the library
pub use stm32f0_embassy::time_wrapper::{EmbassyDuration, EmbassyInstant, EmbassyTimeSource};

/// Which LED to target with a command
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum LedId {
    Led1,
    Led2,
}

/// Extended command type that includes color queries
///
/// The library's SequencerCommand handles Load/Start/Pause/Resume/etc,
/// but we need an additional GetColor command for querying LED state.
pub enum ExtendedCommand {
    /// Standard sequencer command targeting a specific LED
    Sequencer(SequencerCommand<LedId, EmbassyDuration, SEQUENCE_STEP_CAPACITY>),

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
pub static RGB_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, ExtendedCommand, 4> = Channel::new();

/// Signal for receiving color query responses
pub static COLOR_RESPONSE_SIGNAL: Signal<ThreadModeRawMutex, Srgb> = Signal::new();

/// Maximum number of steps that can be stored in a sequence
pub const SEQUENCE_STEP_CAPACITY: usize = 8;
