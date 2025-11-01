#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

//! # Core Concepts
//!
//! - **`RgbSequence`**: Defines a complete animation sequence with steps, loops, start color and landing color
//! - **`SequenceStep`**: The building blocks for RgbSequence`s. Contains a color, a duration and a `TransitionStyle`
//! - **`TransitionStyle`**: How to animate to a color (instant `Step` or smooth `Linear`)
//! - **`LoopCount`**: How many times to repeat (`Finite(n)` or `Infinite`)
//! - **`RgbSequencer`**: Owns a single `RgbLed` and controls it through timed `RgbSequence`s
//! - **`RgbLed`**: Trait to implement for your LED hardware
//! - **`TimeSource`**: Trait to implement for your timing system
//! - **`StepPosition`**: Information about the position within a sequence for a given point in time
//! - **`SequencerAction`**: Commands that can be sent to control `RgbSequencer`s
//!
//! The library uses `palette::Srgb<f32>` (0.0-1.0 range) for all color operations and interpolation.
//! When implementing `RgbLed` for your hardware, convert these values to your device's
//! native format (e.g., 8-bit integers, PWM duty cycles).

// Re-export Srgb from palette for user convenience
pub use palette::Srgb;

pub mod time;
pub mod types;
pub mod sequence;
pub mod sequencer;
pub mod command;

pub use sequence::{RgbSequence, SequenceBuilder, StepPosition};
pub use types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};
pub use time::{TimeDuration, TimeInstant, TimeSource};
pub use sequencer::{RgbSequencer, RgbLed, SequencerState, SequencerError};
pub use command::{SequencerAction, SequencerCommand};

pub const COLOR_OFF: Srgb = Srgb::new(0.0, 0.0, 0.0);
