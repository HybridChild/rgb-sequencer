//! Command-based control for sequencers.

use crate::sequence::RgbSequence;
use crate::time::TimeDuration;

/// Actions for controlling sequencers.
#[derive(Debug, Clone)]
pub enum SequencerAction<D: TimeDuration, const N: usize> {
    /// Load sequence.
    Load(RgbSequence<D, N>),
    /// Start sequence.
    Start,
    /// Stop sequence.
    Stop,
    /// Pause sequence.
    Pause,
    /// Resume sequence.
    Resume,
    /// Restart sequence.
    Restart,
    /// Clear sequence.
    Clear,
}

/// Command targeting a specific LED.
#[derive(Debug, Clone)]
pub struct SequencerCommand<Id, D: TimeDuration, const N: usize> {
    pub led_id: Id,
    pub action: SequencerAction<D, N>,
}

impl<Id, D: TimeDuration, const N: usize> SequencerCommand<Id, D, N> {
    /// Creates command.
    pub fn new(led_id: Id, action: SequencerAction<D, N>) -> Self {
        Self { led_id, action }
    }
}
