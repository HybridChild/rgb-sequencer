#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

//! # Core Concepts
//!
//! - **`RgbSequence`**: Defines a complete animation sequence with steps, loops, start color and landing color
//! - **`SequenceStep`**: A single color + duration + transition within a sequence
//! - **`TransitionStyle`**: How to animate to a color (instant `Step` or smooth `Linear`)
//! - **`LoopCount`**: How many times to repeat (`Finite(n)` or `Infinite`)
//! - **`RgbSequencer`**: Controls a single RGB LED through timed color sequences
//! - **`RgbLed`**: Trait to implement for your LED hardware
//! - **`TimeSource`**: Trait to implement for your timing system
//! - **`StepPosition`**: Information about the current position within a sequence
//! - **`SequencerAction`**: Commands that can be sent to control sequencers
//!
//! The library uses `Srgb<f32>` (0.0-1.0 range) for all color operations and interpolation.
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

#[cfg(test)]
mod tests {
    use super::*;
    
    // Basic compilation tests - actual functionality tests would go here
    #[test]
    fn types_compile() {
        let _ = TransitionStyle::Step;
        let _ = TransitionStyle::Linear;
        let _ = LoopCount::Finite(1);
        let _ = LoopCount::Infinite;
    }
}
