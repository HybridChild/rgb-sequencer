use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use rgb_sequencer::SequencerCommand;

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

/// Since mode_switcher only has one LED, we use a unit LED ID
pub type LedId = ();

/// Signal from button_task to app_logic_task when button is pressed
pub static BUTTON_SIGNAL: Signal<ThreadModeRawMutex, ()> = Signal::new();

/// Channel for sending commands from app_logic_task to rgb_task
/// Uses the library's SequencerCommand type with 8-step capacity
pub static RGB_COMMAND_CHANNEL: Channel<
    ThreadModeRawMutex,
    SequencerCommand<LedId, EmbassyDuration, 8>,
    2,
> = Channel::new();
