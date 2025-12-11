#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! `no_std` RGB LED animation library for embedded systems.
//!
//! Provides step-based and function-based color sequences with trait abstractions for LED hardware and timing.
//! Zero heap allocation, platform-independent, type-safe colors via `palette::Srgb<f32>`.
//!
//! # Core Types
//!
//! - **`RgbSequence`** - Defines an animation (steps, loops, transitions, colors)
//! - **`RgbSequencer`** - Controls a single LED through sequences with state management
//! - **`RgbLed`** - Trait for LED hardware abstraction
//! - **`TimeSource`** - Trait for timing system abstraction
//! - **`TransitionStyle`** - How to animate between colors (Step, Linear, EaseIn/Out)
//!
//! # Color Helpers
//!
//! - **`colors`** module - HSV color space helpers for intuitive color creation
//!
//! Uses f32 extensively - performance varies by FPU availability.

// Re-export Srgb from palette for user convenience
pub use palette::Srgb;

pub mod colors;
pub mod command;
pub mod sequence;
pub mod sequencer;
pub mod time;
pub mod types;

pub use command::{SequencerAction, SequencerCommand};
pub use sequence::{RgbSequence, SequenceBuilder, StepPosition};
pub use sequencer::{
    Position, RgbLed, RgbSequencer, SequencerError, SequencerState, ServiceTiming,
};
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

// Type aliases for common sequencer capacities

/// RGB sequencer with capacity for 4 steps.
pub type RgbSequencer4<'t, I, L, T> = RgbSequencer<'t, I, L, T, 4>;

/// RGB sequencer with capacity for 8 steps.
pub type RgbSequencer8<'t, I, L, T> = RgbSequencer<'t, I, L, T, 8>;

/// RGB sequencer with capacity for 16 steps.
pub type RgbSequencer16<'t, I, L, T> = RgbSequencer<'t, I, L, T, 16>;

/// RGB sequence with capacity for 4 steps.
pub type RgbSequence4<D> = RgbSequence<D, 4>;

/// RGB sequence with capacity for 8 steps.
pub type RgbSequence8<D> = RgbSequence<D, 8>;

/// RGB sequence with capacity for 16 steps.
pub type RgbSequence16<D> = RgbSequence<D, 16>;
