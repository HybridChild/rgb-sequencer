# Features

## Step-Based Sequences

Step-based sequences define animations as a series of color waypoints with explicit durations and transition styles.

### Basic Step Construction

```rust
let sequence = RgbSequence::builder()
    .step(Srgb::new(1.0, 0.0, 0.0), Duration::from_millis(1000), TransitionStyle::Step)
    .step(Srgb::new(0.0, 1.0, 0.0), Duration::from_millis(500), TransitionStyle::Linear)
    .build()?;
```

### Transition Styles

- **`TransitionStyle::Step`**: Instantly jumps to the target color and holds it for the duration. Perfect for discrete animations like blinking or status indicators.

- **`TransitionStyle::Linear`**: Smoothly interpolates from the previous color to the target color over the step's duration using linear RGB interpolation. Ideal for smooth fades and color transitions.

### Zero-Duration Steps

For steps with `TransitionStyle::Step`, setting zero-duration is allowed and serves as a color waypoint:

```rust
let sequence = RgbSequence::builder()
    .step(Srgb::new(1.0, 1.0, 0.0), Duration::from_millis(0), TransitionStyle::Step)       // Yellow waypoint
    .step(Srgb::new(0.0, 0.0, 0.0), Duration::from_millis(1000), TransitionStyle::Linear)  // Fade to black
    .loop_count(LoopCount::Infinite)
    .build()?;
```

This creates a sequence that on each loop iteration, will jump to yellow and then smoothly transition to black (off).

**Important**: Zero-duration steps with `TransitionStyle::Linear` are invalid and will be rejected during sequence building.

### Start Color for Smooth Entry

The `start_color()` method allows you to define a color to interpolate from at the very beginning of the sequence.

```rust
let sequence = RgbSequence::builder()
    .start_color(Srgb::new(0.0, 0.0, 0.0))  // Start from black
    .step(Srgb::new(1.0, 0.0, 0.0), Duration::from_millis(2000), TransitionStyle::Linear)  // Fade to red
    .step(Srgb::new(0.0, 0.0, 1.0), Duration::from_millis(2000), TransitionStyle::Linear)  // Fade to blue
    .loop_count(LoopCount::Infinite)
    .build()?;
```

**Behavior:**
- **First loop**: Uses `start_color` for interpolation to first step (black → red)
- **Subsequent loops**: Uses last step's color for interpolation to first step (blue → red)

This is particularly useful for creating smooth entry animations into looping sequences without affecting the loop-to-loop transitions.

**Note**: `start_color` only affects the first step if it uses `TransitionStyle::Linear`. For `TransitionStyle::Step`, the start color is ignored.

### Landing Color for Completion

For finite sequences, you can specify a `landing_color` to display after all loops complete:

```rust
let sequence = RgbSequence::builder()
    .step(Srgb::new(1.0, 0.0, 0.0), Duration::from_millis(500), TransitionStyle::Step)  // Red
    .step(Srgb::new(0.0, 1.0, 0.0), Duration::from_millis(500), TransitionStyle::Step)  // Green
    .loop_count(LoopCount::Finite(3))
    .landing_color(Srgb::new(0.0, 0.0, 1.0))  // Turn blue when done
    .build()?;
```

**Behavior:**
- The sequence blinks red/green 3 times
- After completion, the LED turns blue and stays blue

**Note**: If no `landing_color` is specified, the LED holds the last step's color

**Note**: `landing_color` is ignored for infinite sequences.

### Loop Count

Control how many times a sequence repeats:

```rust
// Run once and stop
.loop_count(LoopCount::Finite(1))

// Run 5 times and stop
.loop_count(LoopCount::Finite(5))

// Run forever
.loop_count(LoopCount::Infinite)
```

**Note**: If no Loop Count is specified, the sequence will default to `LoopCount::Finite(1)`

## Function-Based Sequences

Function-based sequences use custom functions to compute colors algorithmically based on elapsed time. This enables mathematical animations, procedural patterns, and dynamic effects that would be difficult to express with discrete steps.

### Creating a Function-Based Sequence

```rust
// Define base color 
let white = Srgb::new(1.0, 1.0, 1.0)

// Define color function
fn breathing_effect(base_color: Srgb, elapsed: Duration) -> Srgb {
    // Calculate breathing cycle (4 seconds)
    let time_in_cycle = (elapsed.as_millis() % 4000) as f32 / 4000.0;
    let angle = time_in_cycle * 2.0 * core::f32::consts::PI;
    
    // Sine wave brightness (10% to 100%)
    let brightness = 0.1 + 0.45 * (1.0 + libm::sinf(angle));
    
    Srgb::new(
        base_color.red * brightness,
        base_color.green * brightness,
        base_color.blue * brightness,
    )
}

// Define timing function
fn continuous_timing(_elapsed: Duration) -> Option<Duration> {
    Some(Duration::ZERO)  // Update every frame
}

// Create sequence
let sequence = RgbSequence::from_function(
    white,
    breathing_effect,
    continuous_timing,
);
```

### The Two Functions

Function-based sequences requires two custom function definitions:

#### 1. Color Function: `fn(Srgb, Duration) -> Srgb`

Computes the LED color for a given elapsed time:
- **First parameter**: The base color
- **Second parameter**: Time elapsed since sequence started
- **Returns**: The color to display at this time

This design allows the same function to be reused with different base colors:

```rust
let red = Srgb::new(1.0, 0.0, 0.0);
let blue = Srgb::new(0.0, 0.0, 1.0);

// Same function, different colors
let red_pulse = RgbSequence::from_function(
    red,
    breathing_effect,
    continuous_timing,
);

let blue_pulse = RgbSequence::from_function(
    blue,
    breathing_effect,
    continuous_timing,
);
```

#### 2. Timing Function: `fn(Duration) -> Option<Duration>`

Tells the sequencer when it needs to be serviced again:
- **Parameter**: Time elapsed since sequence started
- **Returns**: The duration until next service at this time
  - `Some(Duration::ZERO)` - Continuous animation, call service() at your desired frame rate
  - `Some(duration)` - Static color period - the LED needs updating after this duration
  - `None` - Animation complete - Sequence is done - No further service is needed

Example with completion:

```rust
fn timed_pulse(elapsed: Duration) -> Option<Duration> {
    if elapsed.as_millis() < 5000 {
        Some(Duration::ZERO)  // Animate for 5 seconds
    } else {
        None  // Then complete
    }
}
```

### Role of Base Color

For function-based sequences, the "base color" passed to `from_function()` serves as the color that gets passed to your color function. It's not used for interpolation like in step-based sequences—instead, it's available for your function to modulate, blend, use as a reference or ignore.

This allows for flexible color-agnostic functions:

```rust
// Organic fire flicker using multiple sine waves
fn fire_flicker(base: Srgb, elapsed: Duration) -> Srgb {
    let t = elapsed.as_millis() as f32 / 1000.0;
    
    // Combine multiple frequencies for organic look
    let flicker1 = libm::sinf(t * 7.0);
    let flicker2 = libm::sinf(t * 13.0) * 0.5;
    let flicker3 = libm::sinf(t * 23.0) * 0.25;
    
    let combined = (flicker1 + flicker2 + flicker3) / 1.75;
    let brightness = 0.7 + 0.3 * combined;
    
    Srgb::new(
        base.red * brightness,
        base.green * brightness,
        base.blue * brightnes
    )
}

let dim_red = RgbSequence::from_function(
    Srgb::new(1.0, 0.0, 0.0),
    fire_flicker,
    continuous_timing,
);
```

### Step-based vs. Function-based Sequences

Use step-based sequences when:
- Simple stuff like just setting a static color or blinking
- You only need instant color changes or linear transitions
- You have a fixed set of color waypoints
- Your animation fits naturally into discrete stages

Use function-based sequences when:
- You need smooth mathematical animations (sine waves, easing functions)
- You want algorithmic patterns that don't fit into discrete steps
- You want to reuse the same animation logic with different colors
- Your animation depends on complex calculations

## Servicing the Sequencer

The `service()` method is the heart of the sequencer. It calculates the appropriate color for the current time, updates its LED and tells you when to call it again.

### Understanding the Return Value

```rust
let led = MyLed::new();
let timer = MyTimer::new();
let mut sequencer = RgbSequencer::<_, _, _, 8>::new(led, &timer);

sequencer.load(sequence);
sequencer.start().unwrap();

loop {
    match sequencer.service() {
        Ok(ServiceTiming::Continuous) => {
            // Continuous color change in progress
            // Sleep for your desired frame rate (e.g., 16ms for ~60fps)
            sleep_ms(16);
        }
        Ok(ServiceTiming::Delay(duration)) => {
            // Holding a static color
            // Sleep for this exact duration
            sleep_ms(duration.as_millis());
        }
        Ok(ServiceTiming::Complete) => {
            // Finite sequence completed
            // No more servicing needed
            break;
        }
        Err(e) => {
            // Error (e.g., called from wrong state)
            handle_error(e);
        }
    }
}
```

**Note**: For function-based sequences, the `service()` method will call the [timing function](#2-timing-function-fnduration---optionduration) internally and forward its return value.

### Multi-LED Servicing

When managing multiple LEDs, coordinate timing across all sequencers:

```rust
use rgb_sequencer::ServiceTiming;

let mut has_continuous = false;
let mut min_delay = None;
let mut all_complete = true;

for sequencer in sequencers.iter_mut() {
    match sequencer.service() {
        Ok(ServiceTiming::Continuous) => {
            has_continuous = true;
            all_complete = false;
        }
        Ok(ServiceTiming::Delay(delay)) => {
            all_complete = false;
            min_delay = Some(match min_delay {
                None => delay,
                Some(current) if delay < current => delay,
                Some(current) => current,
            });
        }
        Ok(ServiceTiming::Complete) => {
            // This sequencer is done
        }
        Err(_) => {
            // Handle error
        }
    }
}

if has_continuous {
    sleep_ms(16);  // Frame rate for any continuous animations
} else if let Some(delay) = min_delay {
    sleep_ms(delay.as_millis());  // Sleep until next step change
} else if all_complete {
    break;  // All sequences done
}
```

## State Machine

The sequencer implements a state machine that validates operation preconditions and prevents invalid state transitions.

### States

- **`Idle`**: No sequence loaded, LED is off
- **`Loaded`**: Sequence loaded but not started, LED is off
- **`Running`**: Sequence actively executing, LED displays animated colors
- **`Paused`**: Sequence paused at current color
- **`Complete`**: Finite sequence finished, LED displays landing color or last step color

### Sequencer operations and resulting State changes

| Method      | Required State                     | Result State            |
|-------------|------------------------------------|-------------------------|
| `load()`    | Any                                | `Loaded`                |
| `start()`   | `Loaded`                           | `Running`               |
| `service()` | `Running`                          | `Running` or `Complete` |
| `pause()`   | `Running`                          | `Paused`                |
| `resume()`  | `Paused`                           | `Running`               |
| `restart()` | `Running`, `Paused`, or `Complete` | `Running`               |
| `stop()`    | `Running`, `Paused`, or `Complete` | `Loaded`                |
| `clear()`   | Any                                | `Idle`                  |

Calling a method from an invalid state returns `Err(SequencerError::InvalidState)`.

### Checking State

```rust
match sequencer.get_state() {
    SequencerState::Running => {
        // Safe to call service(), pause(), stop(), restart()
    }
    SequencerState::Paused => {
        // Safe to call resume(), stop(), restart()
    }
    SequencerState::Complete => {
        // Sequence finished, safe to call restart(), stop(), clear()
    }
    // ... handle other states
}

// Convenience methods
if sequencer.is_running() {
    sequencer.service()?;
}

if sequencer.is_paused() {
    sequencer.resume()?;
}
```

## Pause and Resume with Timing Compensation

The pause/resume functionality maintains perfect timing continuity, as if the pause never occurred.

### How It Works

When you call `pause()`:
1. The current time is recorded as `pause_start_time`
2. The LED stays at the current color
3. State transitions to `Paused`

When you call `resume()`:
1. The pause duration is calculated: `now - pause_start_time`
2. The sequence's `start_time` is adjusted forward by the pause duration
3. State transitions to `Running`
4. The sequence continues from exactly where it left off

### Use Cases

- Interactive color capture (pause to "freeze" a color)
- User-controlled animation playback
- Event-driven synchronization across multiple LEDs

## Multi-LED Control

The library supports multiple patterns for controlling multiple LEDs independently.

### Pattern 1: Separate Sequencers

The simplest approach—create a sequencer for each LED:

```rust
let mut sequencer_1 = RgbSequencer::new(led_1, &timer);
let mut sequencer_2 = RgbSequencer::new(led_2, &timer);

sequencer_1.load(rainbow_sequence);
sequencer_2.load(pulse_sequence);

sequencer_1.start()?;
sequencer_2.start()?;

loop {
    let timing_1 = sequencer_1.service()?;
    let timing_2 = sequencer_2.service()?;

    // Combine timing from both sequencers
    match (timing_1, timing_2) {
        (ServiceTiming::Complete, ServiceTiming::Complete) => break,  // Both done

        (ServiceTiming::Continuous, _) | (_, ServiceTiming::Continuous) => {
            sleep_ms(16);  // Either has continuous animation
        }

        (ServiceTiming::Delay(d1), ServiceTiming::Delay(d2)) => {
            sleep_ms(d1.min(d2).as_millis());  // Sleep until first change
        }

        (ServiceTiming::Delay(d), ServiceTiming::Complete) |
        (ServiceTiming::Complete, ServiceTiming::Delay(d)) => {
            sleep_ms(d.as_millis());  // One still running
        }
    }
}
```

Here's a more explanatory version:

---

### Pattern 2: Heterogeneous Collections (Advanced)

When you have multiple LEDs connected to different hardware peripherals (e.g., one LED on TIM1, another on TIM3), you face a type system challenge: each LED has a different concrete type (`PwmRgbLed<TIM1>` vs `PwmRgbLed<TIM3>`), which means you can't store them in the same `Vec` or array.

The solution is an **enum wrapper** that unifies different LED types under a single type while maintaining zero-cost abstraction:

```rust
// Wrapper enum for different LED types
pub enum AnyLed<'d> {
    Tim1(PwmRgbLed<'d, TIM1>),
    Tim3(PwmRgbLed<'d, TIM3>),
}

impl<'d> RgbLed for AnyLed<'d> {
    fn set_color(&mut self, color: Srgb) {
        match self {
            AnyLed::Tim1(led) => led.set_color(color),
            AnyLed::Tim3(led) => led.set_color(color),
        }
    }
}

// Now all sequencers have the same type and can be stored together!
let mut sequencers: Vec<RgbSequencer<_, AnyLed, _, 8>, 4> = Vec::new();

sequencers.push(RgbSequencer::new(AnyLed::Tim1(led_1), &timer))?;
sequencers.push(RgbSequencer::new(AnyLed::Tim3(led_2), &timer))?;

// Access individual LEDs by index
for (i, sequencer) in sequencers.iter_mut().enumerate() {
    sequencer.load(get_sequence_for_led(i));
    sequencer.start()?;
}
```

See `examples/stm32f0-embassy/bin/rainbow_capture` for a complete implementation.

### Pattern 3: Command-Based Control

For task-based systems (like Embassy), use the `SequencerCommand` type for message passing:

```rust
use rgb_sequencer::{SequencerCommand, SequencerAction};

// Define LED identifiers
enum LedId { Led1, Led2 }

// Create command channel
static COMMAND_CHANNEL: Channel<SequencerCommand<LedId, Duration, 8>, 4> = Channel::new();

// Send commands from control task
COMMAND_CHANNEL.send(SequencerCommand::new(
    LedId::Led1,
    SequencerAction::Load(sequence),
)).await;

COMMAND_CHANNEL.send(SequencerCommand::new(
    LedId::Led2,
    SequencerAction::Pause,
)).await;

// Handle commands in RGB task
let command = COMMAND_CHANNEL.receive().await;
if let Some(sequencer) = get_sequencer_mut(command.led_id) {
    sequencer.handle_action(command.action)?;
}
```

See `examples/stm32f0-embassy/bin/mode_switcher` for a complete implementation.

## Querying Sequencer State

Beyond checking the state machine, you can query other aspects of a sequencer:

```rust
// Get the current LED color
let color = sequencer.current_color();

// Get elapsed time (if running)
if let Some(elapsed) = sequencer.elapsed_time() {
    println!("Sequence has been running for {}ms", elapsed.as_millis());
}

// Get a reference to the loaded sequence
if let Some(sequence) = sequencer.current_sequence() {
    let steps = sequence.step_count();
    let duration = sequence.loop_duration();
    // ... inspect sequence properties
}

// Check if a finite sequence has completed 
if sequence.has_completed(elapsed) {
    // Do something when done
}
```

These methods are useful for:
- **Color capture**: Getting the current color to create derived sequences
- **Synchronization**: Coordinating multiple LEDs based on elapsed time
- **UI feedback**: Displaying sequence progress to users
- **Debugging**: Inspecting sequence state during development

## Choosing Sequence Capacity

The const generic parameter `N` determines how many steps a sequence can hold:

```rust
RgbSequencer<MyInstant, MyLed, MyTimer, 8>   // Up to 8 steps
RgbSequencer<MyInstant, MyLed, MyTimer, 16>  // Up to 16 steps
RgbSequencer<MyInstant, MyLed, MyTimer, 32>  // Up to 32 steps
```

### Guidelines

- **Start with 8**: Sufficient for most simple animations (blinks, pulses, basic cycles)
- **Use 16**: For complex multi-color sequences (rainbow cycles, multi-stage effects)
- **Use 32+**: For elaborate shows or data-driven animations
- **Function-based**: Don't need any steps — they compute colors algorithmically
