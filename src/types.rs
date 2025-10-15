use crate::time::TimeDuration;
use palette::rgb::Rgb;

/// Defines how a sequence step transitions to its target color.
///
/// The transition style determines how the sequencer interpolates from the previous
/// color state to this step's target color over the step's duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionStyle {
    /// Instantly jump to the target color with no interpolation.
    ///
    /// The step's duration determines how long to hold the color before moving
    /// to the next step. A duration of zero means an instant color change that
    /// holds indefinitely (useful for static colors or waypoints).
    Step,

    /// Smoothly interpolate from the previous color to the target color.
    ///
    /// Uses linear interpolation in RGB color space over the step's duration.
    /// Cannot be used with zero-duration steps as interpolation requires time.
    Linear,
}

/// Defines how many times a sequence should repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopCount {
    /// Repeat the sequence a specific number of times.
    ///
    /// After completing all loops, the sequence either transitions to the landing
    /// color (if specified) or holds the last step's color.
    Finite(u32),

    /// Repeat the sequence indefinitely.
    ///
    /// The sequence will never complete and the landing color is never used.
    Infinite,
}

impl Default for LoopCount {
    fn default() -> Self {
        LoopCount::Finite(1)
    }
}

/// A single step in an RGB sequence.
///
/// Each step defines a target color, how long the step lasts, and how to transition
/// to that color from the previous state.
#[derive(Debug, Clone, Copy)]
pub struct SequenceStep<D: TimeDuration> {
    /// The target RGB color for this step.
    pub color: Rgb<u8>,

    /// How long this step lasts, including transition time.
    ///
    /// For `Step` transitions, this is how long to hold the color.
    /// For `Linear` transitions, this is how long to spend interpolating to the color.
    /// Zero duration is only valid with `Step` transitions.
    pub duration: D,

    /// How to transition to this step's color from the previous state.
    pub transition: TransitionStyle,
}

impl<D: TimeDuration> SequenceStep<D> {
    /// Creates a new sequence step.
    ///
    /// # Arguments
    /// * `color` - The target RGB color for this step
    /// * `duration` - How long this step lasts
    /// * `transition` - How to transition to this color
    pub fn new(color: Rgb<u8>, duration: D, transition: TransitionStyle) -> Self {
        Self {
            color,
            duration,
            transition,
        }
    }
}

/// Errors that can occur when building or validating a sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceError {
    /// The sequence has no steps. At least one step is required.
    EmptySequence,

    /// A step with zero duration was combined with a Linear transition.
    ///
    /// Linear transitions require non-zero duration to perform interpolation.
    /// Use `Step` transition for zero-duration steps.
    ZeroDurationWithLinear,
}

impl core::fmt::Display for SequenceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SequenceError::EmptySequence => {
                write!(f, "sequence must have at least one step")
            }
            SequenceError::ZeroDurationWithLinear => {
                write!(
                    f,
                    "zero-duration steps must use Step transition, not Linear"
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SequenceError {}
