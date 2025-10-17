#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

//! A `no_std`-compatible library for controlling RGB LEDs through timed color sequences.
//!
//! This crate provides the building blocks for defining and executing RGB LED animations
//! on embedded systems. Each sequence consists of multiple steps with colors, durations,
//! and transition styles, and can be configured to loop finitely or infinitely.
//!
//! # Core Concepts
//!
//! - **`RgbSequence`**: Defines a complete animation sequence with steps, loops, and landing color
//! - **`SequenceStep`**: A single color + duration + transition within a sequence
//! - **`TransitionStyle`**: How to animate to a color (instant `Step` or smooth `Linear`)
//! - **`LoopCount`**: How many times to repeat (`Finite(n)` or `Infinite`)
//! - **`RgbSequencer`**: Controls a single RGB LED through timed color sequences
//! - **`RgbLed`**: Trait to implement for your LED hardware
//! - **`TimeSource`**: Trait to implement for your timing system
//!
//! Colors are represented as `Srgb<f32>` (0.0-1.0 range) internally for accurate interpolation.
//! Users can convert to other formats in their `RgbLed` implementation as needed.
//!
//! See the repository README for complete usage examples.

// Re-export Srgb from palette for user convenience
pub use palette::Srgb;

pub const COLOR_OFF: Srgb = Srgb::new(0.0, 0.0, 0.0);

pub mod time;
pub mod types;
pub mod sequence;
pub mod sequencer;

pub use sequence::{RgbSequence, SequenceBuilder};
pub use types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};
pub use time::{TimeDuration, TimeInstant};
pub use sequencer::{RgbSequencer, RgbLed, TimeSource, SequencerState, SequencerError};

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
