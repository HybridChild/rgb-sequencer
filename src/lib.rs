#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! `no_std` RGB LED animation library for embedded systems.
//!
//! Provides step-based and function-based color sequences with trait abstractions for LED hardware and timing.
//! Zero heap allocation, platform-independent, type-safe colors via `palette::Srgb<f32>`.
//!
//! Uses f32 extensively - performance varies by FPU availability. See CLAUDE.md for details.

// Re-export Srgb from palette for user convenience
pub use palette::Srgb;

pub mod command;
pub mod sequence;
pub mod sequencer;
pub mod time;
pub mod types;

pub use command::{SequencerAction, SequencerCommand};
pub use sequence::{RgbSequence, SequenceBuilder, StepPosition};
pub use sequencer::{RgbLed, RgbSequencer, SequencerError, SequencerState, ServiceTiming};
pub use time::{TimeDuration, TimeInstant, TimeSource};
pub use types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};

/// Black color (all channels off).
pub const COLOR_OFF: Srgb = Srgb::new(0.0, 0.0, 0.0);

/// Red color (full red channel).
pub const COLOR_RED: Srgb = Srgb::new(1.0, 0.0, 0.0);

/// Green color (full green channel).
pub const COLOR_GREEN: Srgb = Srgb::new(0.0, 1.0, 0.0);

/// Blue color (full blue channel).
pub const COLOR_BLUE: Srgb = Srgb::new(0.0, 0.0, 1.0);

/// White color (all channels full).
pub const COLOR_WHITE: Srgb = Srgb::new(1.0, 1.0, 1.0);

/// Yellow color (red + green).
pub const COLOR_YELLOW: Srgb = Srgb::new(1.0, 1.0, 0.0);

/// Cyan color (green + blue).
pub const COLOR_CYAN: Srgb = Srgb::new(0.0, 1.0, 1.0);

/// Magenta color (red + blue).
pub const COLOR_MAGENTA: Srgb = Srgb::new(1.0, 0.0, 1.0);

/// Orange color.
pub const COLOR_ORANGE: Srgb = Srgb::new(1.0, 0.647, 0.0);

/// Purple color.
pub const COLOR_PURPLE: Srgb = Srgb::new(0.502, 0.0, 0.502);
