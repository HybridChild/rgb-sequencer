#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! `no_std` RGB LED animation library for embedded systems.
//!
//! Provides step-based and function-based color sequences with trait abstractions for LED hardware and timing.
//! Zero heap allocation, platform-independent, and entirely free of floating-point math.
//!
//! # Core Types
//!
//! - **`Rgb`** - 16-bit-per-channel color (`0..=65535`)
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
//! All color math is fixed-point integer arithmetic - no soft-float emulation on any target.

pub mod color;
pub mod colors;
pub mod command;
mod fixed;
pub mod sequence;
pub mod sequencer;
pub mod time;
pub mod types;

pub use color::Rgb;

pub use command::{SequencerAction, SequencerCommand};
pub use sequence::{RgbSequence, SequenceBuilder, StepPosition};
pub use sequencer::{
    DEFAULT_COLOR_EPSILON, Position, RgbLed, RgbSequencer, SequencerError, SequencerState,
    ServiceTiming,
};
pub use time::{TimeDuration, TimeInstant, TimeSource};
pub use types::{LoopCount, SequenceError, SequenceStep, TransitionStyle};

/// Full-scale channel value.
pub const FULL: u16 = u16::MAX;

/// Black color (all channels off).
pub const BLACK: Rgb = Rgb::new(0, 0, 0);

/// Red color (full red channel).
pub const RED: Rgb = Rgb::new(FULL, 0, 0);

/// Green color (full green channel).
pub const GREEN: Rgb = Rgb::new(0, FULL, 0);

/// Blue color (full blue channel).
pub const BLUE: Rgb = Rgb::new(0, 0, FULL);

/// White color (all channels full).
pub const WHITE: Rgb = Rgb::new(FULL, FULL, FULL);

/// Yellow color (red + green).
pub const YELLOW: Rgb = Rgb::new(FULL, FULL, 0);

/// Cyan color (green + blue).
pub const CYAN: Rgb = Rgb::new(0, FULL, FULL);

/// Magenta color (red + blue).
pub const MAGENTA: Rgb = Rgb::new(FULL, 0, FULL);

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

/// Sequencer action with capacity for 4 steps.
pub type SequencerAction4<D> = SequencerAction<D, 4>;

/// Sequencer action with capacity for 8 steps.
pub type SequencerAction8<D> = SequencerAction<D, 8>;

/// Sequencer action with capacity for 16 steps.
pub type SequencerAction16<D> = SequencerAction<D, 16>;

/// Sequencer command with capacity for 4 steps.
pub type SequencerCommand4<Id, D> = SequencerCommand<Id, D, 4>;

/// Sequencer command with capacity for 8 steps.
pub type SequencerCommand8<Id, D> = SequencerCommand<Id, D, 8>;

/// Sequencer command with capacity for 16 steps.
pub type SequencerCommand16<Id, D> = SequencerCommand<Id, D, 16>;
