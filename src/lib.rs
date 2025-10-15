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
//!
//! # Example
//!
//! ```ignore
//! use rgb_sequencer::{RgbSequence, TransitionStyle, LoopCount};
//! use palette::Rgb;
//! use embassy_time::Duration;
//!
//! // Define a sequence that fades between colors
//! let sequence = RgbSequence::<_, 16>::new()
//!     .step(Rgb::new(255, 0, 0), Duration::from_millis(500), TransitionStyle::Linear)
//!     .step(Rgb::new(0, 0, 255), Duration::from_millis(500), TransitionStyle::Linear)
//!     .loop_count(LoopCount::Infinite)
//!     .build()
//!     .unwrap();
//!
//! // Calculate color at any point in time
//! let current_time = Duration::from_millis(250);
//! let color = sequence.color_at(current_time);
//! ```

pub mod time;
pub mod types;
pub mod sequence;

// Re-export Rgb from palette for user convenience
pub use palette::rgb::Rgb;

// Re-export main types for convenient access
pub use sequence::{RgbSequence, SequenceBuilder};
pub use types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};
pub use time::{TimeDuration, TimeInstant};

// Users should implement TimeInstant and TimeDuration for their chosen time library.
// See the trait definitions in the `time` module for requirements.
//
// Example implementations for common time libraries:
//
// For embassy_time:
// ```
// impl TimeDuration for embassy_time::Duration { /* ... */ }
// impl TimeInstant for embassy_time::Instant { /* ... */ }
// ```
//
// For std::time:
// ```
// impl TimeDuration for std::time::Duration { /* ... */ }
// impl TimeInstant for std::time::Instant { /* ... */ }
// ```
//
// For fugit:
// ```
// impl<const NOM: u32, const DENOM: u32> TimeDuration for fugit::Duration<u64, NOM, DENOM> { /* ... */ }
// impl<const NOM: u32, const DENOM: u32> TimeInstant for fugit::Instant<u64, NOM, DENOM> { /* ... */ }
// ```

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
