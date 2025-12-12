//! Command-based control for sequencers.

use crate::sequence::RgbSequence;
use crate::time::TimeDuration;

/// Actions for controlling sequencers.
///
/// Each variant corresponds to a method on `RgbSequencer`. Use with `SequencerCommand`
/// for routing in multi-LED systems.
#[derive(Debug, Clone)]
pub enum SequencerAction<D: TimeDuration, const N: usize> {
    /// Load sequence (transitions to `Loaded` state).
    Load(RgbSequence<D, N>),
    /// Start loaded sequence (requires `Loaded` state).
    Start,
    /// Stop and turn off LED (keeps sequence loaded).
    Stop,
    /// Pause at current color (requires `Running` state).
    Pause,
    /// Resume from pause with timing compensation.
    Resume,
    /// Restart from beginning (from `Running`, `Paused`, or `Complete`).
    Restart,
    /// Clear sequence and turn off LED.
    Clear,
    /// Set brightness multiplier (0.0-1.0, clamped).
    SetBrightness(f32),
}

/// Command targeting a specific LED.
#[derive(Debug, Clone)]
pub struct SequencerCommand<Id, D: TimeDuration, const N: usize> {
    /// LED identifier.
    pub led_id: Id,
    /// Action to execute.
    pub action: SequencerAction<D, N>,
}

impl<Id, D: TimeDuration, const N: usize> SequencerCommand<Id, D, N> {
    /// Creates command.
    pub fn new(led_id: Id, action: SequencerAction<D, N>) -> Self {
        Self { led_id, action }
    }
}
