#![cfg_attr(not(feature = "std"), no_std)]

//! A `no_std`-compatible Rust library for controlling RGB LEDs through timed color sequences on embedded systems.
//!
//! **rgb-sequencer** provides a lightweight, flexible framework for creating and executing RGB LED animations
//! on resource-constrained embedded devices. Instead of manually managing timers, color interpolation, and LED
//! updates in your application code, you define high-level sequences and let the library handle the timing complexity.
//!
//! The library supports two animation approaches:
//!
//! 1. **Step-based sequences**: Define explicit color waypoints with durations and transition styles (instant or
//!    smooth linear interpolation). Perfect for discrete animations like police lights, status indicators, or
//!    scripted color shows. Support finite or infinite looping with configurable landing colors, and smooth
//!    entry animations via start colors.
//!
//! 2. **Function-based sequences**: Use custom functions to compute colors algorithmically based on elapsed time.
//!    Ideal for mathematical animations like sine wave breathing effects, HSV color wheels, or any procedurally
//!    generated pattern.
//!
//! Each `RgbSequencer` instance controls one LED independently through trait abstractions, allowing you to run
//! different animations on multiple LEDs simultaneously, pause and resume individual sequences, and query current
//! colors of individual LEDs.
//!
//! The library is built for embedded systems with
//! - Zero heap allocation (all storage uses fixed-capacity collections with compile-time sizing)
//! - Platform independence (abstracts LED control and timing systems through traits)
//! - Efficient timing (service timing hints enable power-efficient operation without busy-waiting)
//! - Type-safe colors (uses `palette::Srgb<f32>` for accurate color math and smooth transitions)
//!
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
//!
//! # Performance Considerations
//!
//! **IMPORTANT**: This library uses `f32` extensively for color math and interpolation. Performance
//! varies significantly by target:
//!
//! - **Hardware FPU (Fast)**: Cortex-M4F, M7, M33 (e.g., STM32F4, nRF52) - Excellent performance with hardware-accelerated f32.
//! - **No Hardware FPU (Slow)**: Cortex-M0/M0+, M3 (e.g., STM32F0, RP2040) - Software-emulated f32 is 10-100x slower.
//!
//! For non-FPU targets, prefer Step transitions over Linear and avoid math-heavy function-based sequences.
//! The library works on all targets but is optimized for microcontrollers with FPU.

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
