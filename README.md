# rgb-sequencer

A `no_std`-compatible Rust library for controlling RGB LEDs through timed color sequences on embedded systems.

# Overview

**rgb-sequencer** provides a lightweight, flexible framework for creating and executing RGB LED animations on resource-constrained embedded devices. Instead of manually managing timers, color interpolation, and LED updates in your application code, you define high-level sequences and let the library handle the timing complexity.

The library supports two animation approaches:

1. **Step-based sequences**: Define explicit color waypoints with durations and transition styles (instant or smooth linear interpolation). Perfect for discrete animations like police lights, status indicators, or scripted color shows. Support finite or infinite looping with configurable landing colors, and smooth entry animations via start colors.

2. **Function-based sequences**: Use custom functions to compute colors algorithmically based on elapsed time. Ideal for mathematical animations like sine wave breathing effects, HSV color wheels, or any procedurally generated pattern.

Each `RgbSequencer` instance controls one LED independently through trait abstractions, allowing you to:
- Run different animations on multiple LEDs simultaneously
- Pause and resume individual sequences
- Query current colors of individual LEDs

The library is built for embedded systems with:
- **Zero heap allocation**: All storage uses fixed-capacity collections with compile-time sizing
- **Platform independence**: Abstracts LED control and timing systems through traits
- **Efficient timing**: Service timing hints enable power-efficient operation without busy-waiting
- **Type-safe colors**: Uses `palette::Srgb<f32>` for accurate color math and smooth transitions

Whether you're building a status LED that breathes gently, a multi-LED notification system with synchronized animations, or an interactive light show that responds to user input, rgb-sequencer provides the building blocks while letting you focus on your application logic.

# Quick Start

## Add Dependency
```toml
[dependencies]
rgb-sequencer = "0.1"
palette = { version = "0.7.6", default-features = false, features = ["libm"] }
```

## Minimal Example
```rust
use rgb_sequencer::{RgbSequencer, RgbSequence, RgbLed, TimeSource, TransitionStyle, LoopCount};
use palette::Srgb;

// 1. Implement the RgbLed trait for your hardware
struct MyLed {
    // Your GPIO pins, PWM channels, etc.
}

impl RgbLed for MyLed {
    fn set_color(&mut self, color: Srgb) {
        // Convert 0.0-1.0 range to your hardware format
        // e.g., PWM duty cycles, 8-bit RGB values
    }
}

// 2. Implement the TimeSource trait for your timing system
struct MyTimer;
impl TimeSource<MyInstant> for MyTimer {
    fn now(&self) -> MyInstant {
        // Return current time from your timer
    }
}

// 3. Create a blinking sequence
let sequence = RgbSequence::new()
    .step(Srgb::new(1.0, 0.0, 0.0), Duration::from_millis(500), TransitionStyle::Step)  // Red
    .step(Srgb::new(0.0, 0.0, 0.0), Duration::from_millis(500), TransitionStyle::Step)  // Off
    .loop_count(LoopCount::Finite(3))                                                   // Loop 3 times
    .landing_color(Srgb::new(1.0, 1.0, 1.0))                                            // Land on white
    .build()?;

// 4. Create sequencer and start
let led = MyLed::new();
let timer = MyTimer::new();
let mut sequencer = RgbSequencer::<_, _, _, 8>::new(led, &timer);

sequencer.load(sequence);
sequencer.start().unwrap();

// 5. Service in your main loop
loop {
    if let Some(delay) = sequencer.service().unwrap() {
        if delay == TimeDuration::ZERO {
            // Linear transition - sleep for desired frame rate
            sleep_ms(FRAME_RATE_MS);
        } else {
            // Step transition - delay for the specified time
            sleep_ms(delay.as_millis());
        }
    } else {
        // Sequence complete
        break;
    }
}
```

# Features

## Step-Based Sequences

Step-based sequences define animations as a series of color waypoints with explicit durations and transition styles.

### Basic Step Construction

```rust
let sequence = RgbSequence::new()
    .step(Srgb::new(1.0, 0.0, 0.0), Duration::from_millis(1000), TransitionStyle::Step)
    .step(Srgb::new(0.0, 1.0, 0.0), Duration::from_millis(500), TransitionStyle::Linear)
    .build()?;
```

### Transition Styles

- **`TransitionStyle::Step`**: Instantly jumps to the target color and holds it for the duration. Perfect for discrete animations like blinking or status indicators.

- **`TransitionStyle::Linear`**: Smoothly interpolates from the previous color to the target color over the step's duration using linear RGB interpolation. Ideal for smooth fades and color transitions.

### Zero-Duration Steps

Zero-duration steps with `TransitionStyle::Step` are allowed and serve as color waypoints:

```rust
let sequence = RgbSequence::new()
    .step(Srgb::new(1.0, 1.0, 0.0), Duration::from_millis(0), TransitionStyle::Step)  // Yellow waypoint
    .step(Srgb::new(0.0, 0.0, 0.0), Duration::from_millis(1000), TransitionStyle::Linear)  // Fade to black
    .loop_count(LoopCount::Infinite)
    .build()?;
```

This creates a sequence that on each loop iteration, will jump to yellow and then smoothly transition to black (off).

**Important**: Zero-duration steps with `TransitionStyle::Linear` are invalid and will be rejected during sequence building.

### Start Color for Smooth Entry

The `start_color()` method allows you to define a color to interpolate from at the very beginning of the sequence.

```rust
let sequence = RgbSequence::new()
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
let sequence = RgbSequence::new()
    .step(Srgb::new(1.0, 0.0, 0.0), Duration::from_millis(500), TransitionStyle::Step)
    .step(Srgb::new(0.0, 1.0, 0.0), Duration::from_millis(500), TransitionStyle::Step)
    .loop_count(LoopCount::Finite(3))
    .landing_color(Srgb::new(0.0, 0.0, 1.0))  // Turn blue when done
    .build()?;
```

**Behavior:**
- The sequence blinks red/green 3 times
- After completion, the LED turns blue and stays blue
- If no `landing_color` is specified, the LED holds the last step's color

**Note**: `landing_color` is ignored for infinite sequences (they never complete).

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

## Function-Based Sequences

Function-based sequences use custom functions to compute colors algorithmically based on elapsed time. This enables mathematical animations, procedural patterns, and dynamic effects that would be difficult to express with discrete steps.

### Creating a Function-Based Sequence

```rust
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

fn continuous_timing(_elapsed: Duration) -> Option<Duration> {
    Some(Duration::ZERO)  // Update every frame
}

let sequence = RgbSequence::from_function(
    Srgb::new(1.0, 1.0, 1.0),  // Base color (white)
    breathing_effect,
    continuous_timing,
);
```

### The Two Functions

Function-based sequences requires two custom function definitions:

#### 1. Color Function: `fn(Srgb, Duration) -> Srgb`

Computes the LED color at any given time:
- **First parameter**: The base color (from `base_color` in `from_function()`)
- **Second parameter**: Time elapsed since sequence started
- **Returns**: The color to display at this moment

This design allows the same function to be reused with different base colors:

```rust
// Same function, different colors!
let red_pulse = RgbSequence::from_function(
    Srgb::new(1.0, 0.0, 0.0),  // Red base
    breathing_effect,
    continuous_timing,
);

let blue_pulse = RgbSequence::from_function(
    Srgb::new(0.0, 0.0, 1.0),  // Blue base
    breathing_effect,
    continuous_timing,
);
```

#### 2. Timing Function: `fn(Duration) -> Option<Duration>`

Tells the sequencer when it needs to be serviced again:
- **Parameter**: Time elapsed since sequence started
- **Returns**:
  - `Some(Duration::ZERO)` - Continuous animation, call service() at your desired frame rate
  - `Some(duration)` - Static period, no need to service until after this delay
  - `None` - Animation complete, sequence is done

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
// Darken any base color by 50%
fn half_brightness(base: Srgb, _elapsed: Duration) -> Srgb {
    Srgb::new(base.red * 0.5, base.green * 0.5, base.blue * 0.5)
}

let dim_red = RgbSequence::from_function(
    Srgb::new(1.0, 0.0, 0.0),
    half_brightness,
    continuous_timing,
);
```

### When to Use Function-Based Sequences

Use function-based sequences when:
- You need smooth mathematical animations (sine waves, easing functions)
- You want algorithmic patterns that don't fit into discrete steps
- You want to reuse the same animation logic with different colors
- Your animation depends on complex calculations

Use step-based sequences when:
- You have a fixed set of color waypoints
- You want simple, predictable animations
- You prefer declarative definitions over procedural code
- Your animation fits naturally into discrete stages

## Servicing the Sequencer

The `service()` method is the heart of the sequencer. It calculates the appropriate color for the current time, updates its LED and tells you when to call it again.

### Understanding the Return Value

```rust
loop {
    match sequencer.service() {
        Ok(Some(duration)) if duration == Duration::ZERO => {
            // Linear transition in progress - service continuously
            // Sleep for your desired frame rate (e.g., 16ms for ~60fps)
            sleep_ms(16);
        }
        Ok(Some(duration)) => {
            // Step transition - LED is holding a static color
            // Sleep for the exact duration before next service
            sleep_ms(duration.as_millis());
        }
        Ok(None) => {
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

**Note**: For function-based sequences, the `service()` method will call the timing function internally and forward its return value.

### Multi-LED Servicing

When managing multiple LEDs, find the minimum delay across all sequencers:

```rust
let mut min_delay = None;

for sequencer in sequencers.iter_mut() {
    if let Ok(Some(delay)) = sequencer.service() {
        min_delay = match min_delay {
            None => Some(delay),
            Some(current) if delay < current => Some(delay),
            Some(current) => Some(current),
        };
    }
}

if let Some(delay) = min_delay {
    if delay == Duration::ZERO {
        sleep_ms(16);  // Frame rate for linear transitions
    } else {
        sleep_ms(delay.as_millis());  // Exact delay for steps
    }
}
```

## State Machine

The sequencer uses a state machine to enforce correct operation sequences and prevent invalid operations.

### States

- **`Idle`**: No sequence loaded, LED is off
- **`Loaded`**: Sequence loaded but not started, LED is off
- **`Running`**: Sequence actively executing, LED displays animated colors
- **`Paused`**: Sequence paused at current color
- **`Complete`**: Finite sequence finished, LED displays landing color or last step color

### Valid State Transitions

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
2. The LED continues displaying the color from when pause was called
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
- Debugging and testing sequences

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
    let delay_1 = sequencer_1.service()?;
    let delay_2 = sequencer_2.service()?;
    
    // Sleep for minimum delay
    sleep(min(delay_1, delay_2));
}
```

### Pattern 2: Heterogeneous Collections (Advanced)

For managing LEDs with different hardware types (e.g., different PWM timers), use an enum wrapper:

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

// Now all sequencers have the same type!
let mut sequencers: Vec<RgbSequencer<_, AnyLed, _, 8>, 4> = Vec::new();

sequencers.push(RgbSequencer::new(AnyLed::Tim1(led_1), &timer))?;
sequencers.push(RgbSequencer::new(AnyLed::Tim3(led_2), &timer))?;
```

This enables individual LED access by index while maintaining type safety and zero-cost abstraction.

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
- **Function-based**: Don't need many steps—they compute colors algorithmically

### Memory Impact

Each step stores:
- `Srgb` (12 bytes: 3x f32)
- `Duration` (varies by implementation, typically 4-8 bytes)
- `TransitionStyle` (1 byte enum)
- Padding (depends on alignment)

Example: With 8-byte durations, a 16-step sequence uses approximately 320 bytes of stack memory.

**Tip**: Different sequencers can have different capacities. Choose based on each LED's animation complexity.

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

// Check if a finite sequence has completed at a given time
if sequence.has_completed(elapsed) {
    // Do something when done
}
```

These methods are useful for:
- **Color capture**: Getting the current color to create derived sequences
- **Synchronization**: Coordinating multiple LEDs based on elapsed time
- **UI feedback**: Displaying sequence progress to users
- **Debugging**: Inspecting sequence state during development

# License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/HybridChild/rgb-sequencer/blob/main/LICENSE) file for details.
