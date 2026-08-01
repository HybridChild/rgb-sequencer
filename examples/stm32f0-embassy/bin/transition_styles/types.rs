use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use rgb_sequencer::{SequencerCommand16, TransitionStyle};
use stm32f0_embassy::time_wrapper::EmbassyDuration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum TransitionMode {
    Step,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl TransitionMode {
    pub fn next(&self) -> Self {
        match self {
            TransitionMode::Step => TransitionMode::Linear,
            TransitionMode::Linear => TransitionMode::EaseIn,
            TransitionMode::EaseIn => TransitionMode::EaseOut,
            TransitionMode::EaseOut => TransitionMode::EaseInOut,
            TransitionMode::EaseInOut => TransitionMode::Step,
        }
    }

    pub fn to_transition_style(&self) -> TransitionStyle {
        match self {
            TransitionMode::Step => TransitionStyle::Step,
            TransitionMode::Linear => TransitionStyle::Linear,
            TransitionMode::EaseIn => TransitionStyle::EaseIn,
            TransitionMode::EaseOut => TransitionStyle::EaseOut,
            TransitionMode::EaseInOut => TransitionStyle::EaseInOut,
        }
    }

    pub fn blink_count(&self) -> u8 {
        match self {
            TransitionMode::Step => 0,
            TransitionMode::Linear => 1,
            TransitionMode::EaseIn => 2,
            TransitionMode::EaseOut => 3,
            TransitionMode::EaseInOut => 4,
        }
    }
}

pub type LedId = ();

pub static BUTTON_SIGNAL: Signal<ThreadModeRawMutex, ()> = Signal::new();

pub static RGB_COMMAND_CHANNEL: Channel<
    ThreadModeRawMutex,
    SequencerCommand16<LedId, EmbassyDuration>,
    2,
> = Channel::new();
