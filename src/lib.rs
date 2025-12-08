#![no_std]

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

pub const COLOR_OFF: Srgb = Srgb::new(0.0, 0.0, 0.0);
