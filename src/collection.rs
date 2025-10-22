use crate::command::SequencerAction;
use crate::sequencer::{RgbLed, RgbSequencer, SequencerError, SequencerState, TimeSource};
use crate::time::{TimeDuration, TimeInstant};
use palette::Srgb;

/// An identifier for an LED within a sequencer collection.
///
/// This is a simple wrapper around `usize` that provides type safety for LED
/// identifiers. Users specify LED IDs when adding sequencers to a collection,
/// and use these IDs to target specific LEDs with commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LedId(pub usize);

impl From<usize> for LedId {
    fn from(id: usize) -> Self {
        LedId(id)
    }
}

impl From<LedId> for usize {
    fn from(id: LedId) -> Self {
        id.0
    }
}

/// Errors that can occur during collection operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionError {
    /// The specified LED ID does not exist in the collection.
    InvalidLedId(LedId),

    /// Attempted to add a sequencer with an ID that already exists.
    DuplicateLedId(LedId),

    /// The collection is full and cannot accept more sequencers.
    CollectionFull,

    /// The LED ID exceeds the collection's capacity.
    LedIdOutOfBounds { id: LedId, capacity: usize },

    /// A sequencer operation failed.
    SequencerError(SequencerError),
}

impl core::fmt::Display for CollectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CollectionError::InvalidLedId(id) => {
                write!(f, "LED ID {} does not exist in collection", id.0)
            }
            CollectionError::DuplicateLedId(id) => {
                write!(f, "LED ID {} already exists in collection", id.0)
            }
            CollectionError::CollectionFull => {
                write!(f, "collection is full, cannot add more sequencers")
            }
            CollectionError::LedIdOutOfBounds { id, capacity } => {
                write!(
                    f,
                    "LED ID {} exceeds collection capacity of {}",
                    id.0, capacity
                )
            }
            CollectionError::SequencerError(err) => {
                write!(f, "sequencer error: {}", err)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CollectionError {}

impl From<SequencerError> for CollectionError {
    fn from(err: SequencerError) -> Self {
        CollectionError::SequencerError(err)
    }
}

/// Manages a collection of RGB sequencers for coordinated multi-LED control.
///
/// This is a convenience wrapper that handles routing commands to individual
/// sequencers and provides efficient batch servicing of all LEDs. Each sequencer
/// in the collection is identified by a user-specified `LedId`.
///
/// All sequencers in a collection can run different animations (both step-based
/// and function-based) while maintaining a homogeneous type signature. This allows
/// for flexible multi-LED control without heap allocation.
///
/// # Type Parameters
/// * `'t` - Lifetime of the time source reference
/// * `I` - Time instant type
/// * `L` - LED implementation type (must be same for all LEDs in collection)
/// * `T` - Time source implementation type
/// * `N` - Maximum number of steps in sequences
/// * `MAX_LEDS` - Maximum number of LEDs this collection can hold
pub struct SequencerCollection<'t, I: TimeInstant, L: RgbLed, T: TimeSource<I>, const N: usize, const MAX_LEDS: usize> {
    sequencers: [Option<RgbSequencer<'t, I, L, T, N>>; MAX_LEDS],
    time_source: &'t T,
}

impl<'t, I, L, T, const N: usize, const MAX_LEDS: usize>
    SequencerCollection<'t, I, L, T, N, MAX_LEDS>
where
    I: TimeInstant,
    I::Duration: TimeDuration,
    L: RgbLed,
    T: TimeSource<I>,
{
    /// Creates a new empty sequencer collection.
    ///
    /// # Arguments
    /// * `time_source` - Reference to the time source used by all sequencers
    pub fn new(time_source: &'t T) -> Self {
        Self {
            sequencers: core::array::from_fn(|_| None),
            time_source,
        }
    }

    /// Adds a sequencer to the collection with the specified LED ID.
    ///
    /// The LED is moved into a new sequencer which is stored in the collection.
    /// The provided ID is used to reference this LED in future commands.
    ///
    /// # Arguments
    /// * `id` - The identifier to use for this LED
    /// * `led` - The LED hardware implementation
    ///
    /// # Errors
    /// * `DuplicateLedId` - A sequencer with this ID already exists
    /// * `LedIdOutOfBounds` - The ID exceeds the collection's capacity
    pub fn add_sequencer(&mut self, id: LedId, led: L) -> Result<(), CollectionError> {
        let idx = id.0;

        if idx >= MAX_LEDS {
            return Err(CollectionError::LedIdOutOfBounds {
                id,
                capacity: MAX_LEDS,
            });
        }

        if self.sequencers[idx].is_some() {
            return Err(CollectionError::DuplicateLedId(id));
        }

        self.sequencers[idx] = Some(RgbSequencer::new(led, self.time_source));
        Ok(())
    }

    /// Routes a command to the specified sequencer.
    ///
    /// # Arguments
    /// * `id` - The LED identifier
    /// * `action` - The action to perform
    ///
    /// # Returns
    /// * `Ok(Some(duration))` - Time until the sequencer needs service
    /// * `Ok(None)` - Action complete, no timing information
    /// * `Err` - Invalid LED ID or sequencer operation failed
    pub fn handle_command(
        &mut self,
        id: LedId,
        action: SequencerAction<I::Duration, N>,
    ) -> Result<Option<I::Duration>, CollectionError> {
        let idx = id.0;

        if idx >= MAX_LEDS {
            return Err(CollectionError::InvalidLedId(id));
        }

        let sequencer = self.sequencers[idx]
            .as_mut()
            .ok_or(CollectionError::InvalidLedId(id))?;

        Ok(sequencer.handle_action(action)?)
    }

    /// Services all sequencers in the collection and returns optimal sleep duration.
    ///
    /// This method calls `service()` on each sequencer that exists in the collection,
    /// updating their LEDs as needed. It then aggregates the timing information from
    /// all sequencers to determine when the next service call should occur.
    ///
    /// Works seamlessly with both step-based and function-based sequences, automatically
    /// handling the different timing requirements of each.
    ///
    /// # Returns
    /// * `Some(Duration::ZERO)` - At least one sequencer has a continuous animation
    ///   (linear transition or function-based). Service again at your desired frame
    ///   rate (e.g., 16ms for 60fps).
    /// * `Some(duration)` - All active sequencers are in step transitions. Sleep for
    ///   this duration before the next service call.
    /// * `None` - All sequencers are complete or idle. No further servicing needed
    ///   until a new command is received.
    ///
    /// # Errors
    /// Returns an error if any sequencer's service operation fails. In practice,
    /// this should only happen if a sequencer is in an invalid state, which
    /// represents a programming error.
    pub fn service_all(&mut self) -> Result<Option<I::Duration>, CollectionError> {
        let mut min_duration: Option<I::Duration> = None;
        let mut has_zero = false;

        for sequencer_opt in &mut self.sequencers {
            if let Some(sequencer) = sequencer_opt {
                // Only service if the sequencer is in Running state
                if sequencer.get_state() == SequencerState::Running {
                    match sequencer.service()? {
                        Some(duration) => {
                            if duration.as_millis() == 0 {
                                has_zero = true;
                            } else {
                                match min_duration {
                                    None => min_duration = Some(duration),
                                    Some(current_min) => {
                                        if duration.as_millis() < current_min.as_millis() {
                                            min_duration = Some(duration);
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            // Sequencer completed, no contribution to timing
                        }
                    }
                }
            }
        }

        if has_zero {
            Ok(Some(I::Duration::ZERO))
        } else {
            Ok(min_duration)
        }
    }

    /// Returns the current state of the specified sequencer.
    ///
    /// # Errors
    /// Returns `InvalidLedId` if the LED does not exist in the collection.
    pub fn get_state(&self, id: LedId) -> Result<SequencerState, CollectionError> {
        let idx = id.0;

        if idx >= MAX_LEDS {
            return Err(CollectionError::InvalidLedId(id));
        }

        let sequencer = self.sequencers[idx]
            .as_ref()
            .ok_or(CollectionError::InvalidLedId(id))?;

        Ok(sequencer.get_state())
    }

    /// Returns the current color being displayed on the specified LED.
    ///
    /// # Errors
    /// Returns `InvalidLedId` if the LED does not exist in the collection.
    pub fn get_current_color(&self, id: LedId) -> Result<Srgb, CollectionError> {
        let idx = id.0;

        if idx >= MAX_LEDS {
            return Err(CollectionError::InvalidLedId(id));
        }

        let sequencer = self.sequencers[idx]
            .as_ref()
            .ok_or(CollectionError::InvalidLedId(id))?;

        Ok(sequencer.current_color())
    }

    /// Returns the number of sequencers currently in the collection.
    pub fn len(&self) -> usize {
        self.sequencers.iter().filter(|s| s.is_some()).count()
    }

    /// Returns true if the collection contains no sequencers.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if the collection contains a sequencer with the given ID.
    pub fn contains(&self, id: LedId) -> bool {
        let idx = id.0;
        idx < MAX_LEDS && self.sequencers[idx].is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::RgbSequence;
    use crate::time::{TimeDuration, TimeInstant};
    use crate::types::{LoopCount, TransitionStyle};
    use palette::Srgb;

    // Mock Duration type
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct TestDuration(u64);

    impl TimeDuration for TestDuration {
        const ZERO: Self = TestDuration(0);

        fn as_millis(&self) -> u64 {
            self.0
        }

        fn from_millis(millis: u64) -> Self {
            TestDuration(millis)
        }

        fn saturating_sub(self, other: Self) -> Self {
            TestDuration(self.0.saturating_sub(other.0))
        }
    }

    // Mock Instant type
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct TestInstant(u64);

    impl TimeInstant for TestInstant {
        type Duration = TestDuration;

        fn duration_since(&self, earlier: Self) -> Self::Duration {
            TestDuration(self.0 - earlier.0)
        }

        fn checked_add(self, duration: Self::Duration) -> Option<Self> {
            Some(TestInstant(self.0 + duration.0))
        }

        fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
            self.0.checked_sub(duration.0).map(TestInstant)
        }
    }

    // Mock LED
    struct MockLed {
        color: Srgb,
    }

    impl MockLed {
        fn new() -> Self {
            Self {
                color: Srgb::new(0.0, 0.0, 0.0),
            }
        }
    }

    impl RgbLed for MockLed {
        fn set_color(&mut self, color: Srgb) {
            self.color = color;
        }
    }

    // Mock time source
    struct MockTimeSource {
        current_time: core::cell::Cell<TestInstant>,
    }

    impl MockTimeSource {
        fn new() -> Self {
            Self {
                current_time: core::cell::Cell::new(TestInstant(0)),
            }
        }

        fn advance(&self, duration: TestDuration) {
            let current = self.current_time.get();
            self.current_time.set(TestInstant(current.0 + duration.0));
        }
    }

    impl TimeSource<TestInstant> for MockTimeSource {
        fn now(&self) -> TestInstant {
            self.current_time.get()
        }
    }

    const RED: Srgb = Srgb::new(1.0, 0.0, 0.0);
    const GREEN: Srgb = Srgb::new(0.0, 1.0, 0.0);
    const BLUE: Srgb = Srgb::new(0.0, 0.0, 1.0);

    #[test]
    fn can_create_empty_collection() {
        let timer = MockTimeSource::new();
        let collection = SequencerCollection::<TestInstant, MockLed, MockTimeSource, 8, 4>::new(&timer);
        assert_eq!(collection.len(), 0);
        assert!(collection.is_empty());
    }

    #[test]
    fn can_add_sequencers() {
        let timer = MockTimeSource::new();
        let mut collection = SequencerCollection::<TestInstant, MockLed, MockTimeSource, 8, 4>::new(&timer);

        let led0 = MockLed::new();
        let led1 = MockLed::new();

        collection.add_sequencer(LedId(0), led0).unwrap();
        collection.add_sequencer(LedId(1), led1).unwrap();

        assert_eq!(collection.len(), 2);
        assert!(!collection.is_empty());
        assert!(collection.contains(LedId(0)));
        assert!(collection.contains(LedId(1)));
        assert!(!collection.contains(LedId(2)));
    }

    #[test]
    fn rejects_duplicate_led_id() {
        let timer = MockTimeSource::new();
        let mut collection = SequencerCollection::<TestInstant, MockLed, MockTimeSource, 8, 4>::new(&timer);

        let led0 = MockLed::new();
        let led1 = MockLed::new();

        collection.add_sequencer(LedId(0), led0).unwrap();
        let result = collection.add_sequencer(LedId(0), led1);

        assert!(matches!(result, Err(CollectionError::DuplicateLedId(_))));
    }

    #[test]
    fn rejects_led_id_out_of_bounds() {
        let timer = MockTimeSource::new();
        let mut collection = SequencerCollection::<TestInstant, MockLed, MockTimeSource, 8, 4>::new(&timer);

        let led = MockLed::new();
        let result = collection.add_sequencer(LedId(10), led);

        assert!(matches!(result, Err(CollectionError::LedIdOutOfBounds { .. })));
    }

    #[test]
    fn can_handle_commands() {
        let timer = MockTimeSource::new();
        let mut collection = SequencerCollection::<TestInstant, MockLed, MockTimeSource, 8, 4>::new(&timer);

        let led = MockLed::new();
        collection.add_sequencer(LedId(0), led).unwrap();

        let sequence = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .build()
            .unwrap();

        collection
            .handle_command(LedId(0), SequencerAction::Load(sequence))
            .unwrap();

        assert_eq!(collection.get_state(LedId(0)).unwrap(), SequencerState::Loaded);

        collection
            .handle_command(LedId(0), SequencerAction::Start)
            .unwrap();

        assert_eq!(collection.get_state(LedId(0)).unwrap(), SequencerState::Running);
    }

    #[test]
    fn service_all_aggregates_timing() {
        let timer = MockTimeSource::new();
        let mut collection = SequencerCollection::<TestInstant, MockLed, MockTimeSource, 8, 4>::new(&timer);

        // Add three LEDs with different step durations
        collection.add_sequencer(LedId(0), MockLed::new()).unwrap();
        collection.add_sequencer(LedId(1), MockLed::new()).unwrap();
        collection.add_sequencer(LedId(2), MockLed::new()).unwrap();

        let seq0 = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(5000), TransitionStyle::Step)
            .build()
            .unwrap();

        let seq1 = RgbSequence::<TestDuration, 8>::new()
            .step(GREEN, TestDuration(2000), TransitionStyle::Step)
            .build()
            .unwrap();

        let seq2 = RgbSequence::<TestDuration, 8>::new()
            .step(BLUE, TestDuration(3000), TransitionStyle::Step)
            .build()
            .unwrap();

        collection.handle_command(LedId(0), SequencerAction::Load(seq0)).unwrap();
        collection.handle_command(LedId(1), SequencerAction::Load(seq1)).unwrap();
        collection.handle_command(LedId(2), SequencerAction::Load(seq2)).unwrap();

        collection.handle_command(LedId(0), SequencerAction::Start).unwrap();
        collection.handle_command(LedId(1), SequencerAction::Start).unwrap();
        collection.handle_command(LedId(2), SequencerAction::Start).unwrap();

        // service_all should return the minimum duration (2000ms from LED 1)
        let duration = collection.service_all().unwrap();
        assert_eq!(duration, Some(TestDuration(2000)));
    }

    #[test]
    fn service_all_returns_zero_for_linear_transitions() {
        let timer = MockTimeSource::new();
        let mut collection = SequencerCollection::<TestInstant, MockLed, MockTimeSource, 8, 4>::new(&timer);

        collection.add_sequencer(LedId(0), MockLed::new()).unwrap();

        let seq = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(1000), TransitionStyle::Linear)
            .build()
            .unwrap();

        collection.handle_command(LedId(0), SequencerAction::Load(seq)).unwrap();
        collection.handle_command(LedId(0), SequencerAction::Start).unwrap();

        let duration = collection.service_all().unwrap();
        assert_eq!(duration, Some(TestDuration::ZERO));
    }

    #[test]
    fn service_all_returns_zero_for_function_based() {
        let timer = MockTimeSource::new();
        let mut collection = SequencerCollection::<TestInstant, MockLed, MockTimeSource, 8, 4>::new(&timer);

        collection.add_sequencer(LedId(0), MockLed::new()).unwrap();

        fn test_fn(base: Srgb, _elapsed: TestDuration) -> Srgb {
            base
        }

        fn continuous(_elapsed: TestDuration) -> Option<TestDuration> {
            Some(TestDuration::ZERO)
        }

        let seq = RgbSequence::<TestDuration, 8>::from_function(RED, test_fn, continuous);

        collection.handle_command(LedId(0), SequencerAction::Load(seq)).unwrap();
        collection.handle_command(LedId(0), SequencerAction::Start).unwrap();

        let duration = collection.service_all().unwrap();
        assert_eq!(duration, Some(TestDuration::ZERO));
    }

    #[test]
    fn service_all_returns_none_when_all_complete() {
        let timer = MockTimeSource::new();
        let mut collection = SequencerCollection::<TestInstant, MockLed, MockTimeSource, 8, 4>::new(&timer);

        collection.add_sequencer(LedId(0), MockLed::new()).unwrap();

        let seq = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(100), TransitionStyle::Step)
            .loop_count(LoopCount::Finite(1))
            .build()
            .unwrap();

        collection.handle_command(LedId(0), SequencerAction::Load(seq)).unwrap();
        collection.handle_command(LedId(0), SequencerAction::Start).unwrap();

        // Advance time past sequence completion
        timer.advance(TestDuration(200));
        collection.service_all().unwrap();

        // Should return None since sequence is complete
        let duration = collection.service_all().unwrap();
        assert_eq!(duration, None);
    }

    #[test]
    fn collection_handles_mixed_sequence_types() {
        let timer = MockTimeSource::new();
        let mut collection = SequencerCollection::<TestInstant, MockLed, MockTimeSource, 8, 4>::new(&timer);

        collection.add_sequencer(LedId(0), MockLed::new()).unwrap();
        collection.add_sequencer(LedId(1), MockLed::new()).unwrap();

        // LED 0: Step-based sequence
        let step_seq = RgbSequence::<TestDuration, 8>::new()
            .step(RED, TestDuration(1000), TransitionStyle::Step)
            .build()
            .unwrap();

        // LED 1: Function-based sequence
        fn pulse(base: Srgb, _elapsed: TestDuration) -> Srgb {
            base
        }
        fn continuous(_elapsed: TestDuration) -> Option<TestDuration> {
            Some(TestDuration::ZERO)
        }
        let func_seq = RgbSequence::<TestDuration, 8>::from_function(BLUE, pulse, continuous);

        collection.handle_command(LedId(0), SequencerAction::Load(step_seq)).unwrap();
        collection.handle_command(LedId(1), SequencerAction::Load(func_seq)).unwrap();

        collection.handle_command(LedId(0), SequencerAction::Start).unwrap();
        collection.handle_command(LedId(1), SequencerAction::Start).unwrap();

        // Should return ZERO because LED 1 has continuous animation
        let duration = collection.service_all().unwrap();
        assert_eq!(duration, Some(TestDuration::ZERO));
    }
}
