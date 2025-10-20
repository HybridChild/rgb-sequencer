use crate::sequence::RgbSequence;
use crate::time::TimeDuration;

/// An action that can be performed on a sequencer.
///
/// This enum represents all the operations you can perform on an `RgbSequencer`.
/// Each variant corresponds to a method on the sequencer. Actions are typically
/// paired with an LED identifier in a `SequencerCommand` for routing in multi-LED
/// systems.
///
/// # Type Parameters
/// * `D` - The duration type used by sequences
/// * `N` - Maximum number of steps in sequences
#[derive(Debug, Clone)]
pub enum SequencerAction<D: TimeDuration, const N: usize> {
    /// Load a new sequence into the sequencer.
    ///
    /// Replaces any existing sequence and transitions to `Loaded` state.
    /// Can be called from any state.
    Load(RgbSequence<D, N>),

    /// Start the loaded sequence from the beginning.
    ///
    /// Must be called from `Loaded` state. Transitions to `Running` state.
    Start,

    /// Stop the sequence and turn off the LED.
    ///
    /// The sequence remains loaded. Transitions to `Loaded` state.
    /// Can be called from `Running`, `Paused`, or `Complete` states.
    Stop,

    /// Pause the sequence at the current color.
    ///
    /// Must be called from `Running` state. Transitions to `Paused` state.
    Pause,

    /// Resume a paused sequence.
    ///
    /// Adjusts timing to account for pause duration. Must be called from
    /// `Paused` state. Transitions to `Running` state.
    Resume,

    /// Restart the sequence from the beginning.
    ///
    /// Can be called from `Running`, `Paused`, or `Complete` states.
    /// Resets timing and begins execution from the first step.
    Restart,

    /// Clear the sequence and turn off the LED.
    ///
    /// Removes the loaded sequence and transitions to `Idle` state.
    /// Can be called from any state.
    Clear,
}

/// A command that targets a specific LED with an action.
///
/// This struct pairs an LED identifier with a sequencer action, providing a
/// complete routing and operation specification for command-based control.
/// Commands are typically sent through channels or queues to a service task
/// that manages multiple LEDs.
///
/// # Type Parameters
/// * `Id` - The LED identifier type (e.g., `usize`, enum, custom type)
/// * `D` - The duration type used by sequences
/// * `N` - Maximum number of steps in sequences
#[derive(Debug, Clone)]
pub struct SequencerCommand<Id, D: TimeDuration, const N: usize> {
    /// The identifier of the LED to target with this command.
    pub led_id: Id,

    /// The action to perform on the targeted LED.
    pub action: SequencerAction<D, N>,
}

impl<Id, D: TimeDuration, const N: usize> SequencerCommand<Id, D, N> {
    /// Creates a new command targeting a specific LED.
    ///
    /// # Arguments
    /// * `led_id` - The identifier of the LED to control
    /// * `action` - The action to perform
    pub fn new(led_id: Id, action: SequencerAction<D, N>) -> Self {
        Self { led_id, action }
    }
}
