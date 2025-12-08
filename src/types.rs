//! Core types for sequence construction.

use crate::time::TimeDuration;
use palette::Srgb;

/// How to transition to a step's target color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionStyle {
    /// Instantly jump to target color, hold for duration.
    Step,

    /// Smoothly interpolate over duration.
    Linear,
}

/// How many times a sequence should repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopCount {
    /// Repeat a specific number of times.
    Finite(u32),

    /// Repeat indefinitely.
    Infinite,
}

impl Default for LoopCount {
    fn default() -> Self {
        LoopCount::Finite(1)
    }
}

/// A single step in an RGB sequence.
#[derive(Debug, Clone, Copy)]
pub struct SequenceStep<D: TimeDuration> {
    /// Target color.
    pub color: Srgb,

    /// Step duration.
    pub duration: D,

    /// Transition style.
    pub transition: TransitionStyle,
}

impl<D: TimeDuration> SequenceStep<D> {
    /// Creates a new sequence step.
    pub fn new(color: Srgb, duration: D, transition: TransitionStyle) -> Self {
        Self {
            color,
            duration,
            transition,
        }
    }
}

/// Sequence validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceError {
    /// No steps provided.
    EmptySequence,

    /// Zero duration with Linear transition.
    ZeroDurationWithLinear,
}

impl core::fmt::Display for SequenceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SequenceError::EmptySequence => {
                write!(f, "sequence must have at least one step")
            }
            SequenceError::ZeroDurationWithLinear => {
                write!(f, "zero-duration steps must use Step transition")
            }
        }
    }
}
