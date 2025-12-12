use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use rgb_sequencer::SequencerCommand;

// Re-export the time types from the library
pub use stm32f0_embassy::time_wrapper::{EmbassyDuration, EmbassyTimeSource};

/// Brightness levels that can be selected
#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum BrightnessLevel {
    /// Full brightness (100%)
    Full,
    /// High brightness (75%)
    High,
    /// Medium brightness (50%)
    Medium,
    /// Low brightness (25%)
    Low,
    /// Very dim (10%)
    Dim,
}

impl BrightnessLevel {
    /// Get the next brightness level in the cycle
    pub fn next(&self) -> Self {
        match self {
            BrightnessLevel::Full => BrightnessLevel::High,
            BrightnessLevel::High => BrightnessLevel::Medium,
            BrightnessLevel::Medium => BrightnessLevel::Low,
            BrightnessLevel::Low => BrightnessLevel::Dim,
            BrightnessLevel::Dim => BrightnessLevel::Full,
        }
    }

    /// Get the brightness value (0.0-1.0)
    pub fn value(&self) -> f32 {
        match self {
            BrightnessLevel::Full => 1.0,
            BrightnessLevel::High => 0.75,
            BrightnessLevel::Medium => 0.5,
            BrightnessLevel::Low => 0.25,
            BrightnessLevel::Dim => 0.1,
        }
    }
}

/// Signal from button_task to app_logic_task when button is pressed
pub static BUTTON_SIGNAL: Signal<ThreadModeRawMutex, ()> = Signal::new();

/// Channel for sending commands from app_logic_task to rgb_task
/// Uses () as LED ID for single-LED scenario
pub static RGB_COMMAND_CHANNEL: Channel<
    ThreadModeRawMutex,
    SequencerCommand<(), EmbassyDuration, 8>,
    2,
> = Channel::new();
