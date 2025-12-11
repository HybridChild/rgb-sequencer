# Features

## Step-Based Sequences

Step-based sequences define animations as a series of color waypoints with explicit durations and transition styles.

### Basic Step Construction

```rust
let sequence = RgbSequence::builder()
    .step(Srgb::new(1.0, 0.0, 0.0), Duration::from_millis(1000), TransitionStyle::Step)?
    .step(Srgb::new(0.0, 1.0, 0.0), Duration::from_millis(500), TransitionStyle::Linear)?
    .build()?;
```

### Transition Styles

- **`TransitionStyle::Step`**: Instantly jumps to the target color and holds it for the duration. Perfect for discrete animations like blinking or status indicators.

- **`TransitionStyle::Linear`**: Smoothly interpolates from the previous color to the target color over the step's duration using linear RGB interpolation. Ideal for smooth fades and color transitions.

### Zero-Duration Steps

For steps with `TransitionStyle::Step`, setting zero-duration is allowed and serves as a color waypoint:

```rust
let sequence = RgbSequence::builder()
    .step(Srgb::new(1.0, 1.0, 0.0), Duration::from_millis(0), TransitionStyle::Step)?       // Yellow waypoint
    .step(Srgb::new(0.0, 0.0, 0.0), Duration::from_millis(1000), TransitionStyle::Linear)?  // Fade to black
    .loop_count(LoopCount::Infinite)
    .build()?;
```

This creates a sequence that on each loop iteration, will jump to yellow and then smoothly transition to black (off). Zero-duration steps with `TransitionStyle::Linear` are invalid and will be rejected during sequence building.

### Start Color for Smooth Entry

The `start_color()` method allows you to define a color to interpolate from at the very beginning of the sequence.

```rust
let sequence = RgbSequence::builder()
    .start_color(Srgb::new(0.0, 0.0, 0.0))  // Start from black
    .step(Srgb::new(1.0, 0.0, 0.0), Duration::from_millis(2000), TransitionStyle::Linear)?  // Fade to red
    .step(Srgb::new(0.0, 0.0, 1.0), Duration::from_millis(2000), TransitionStyle::Linear)?  // Fade to blue
    .loop_count(LoopCount::Infinite)
    .build()?;
```

**Behavior:**
- **First loop**: Uses `start_color` for interpolation to first step (black → red)
- **Subsequent loops**: Uses last step's color for interpolation to first step (blue → red)

This is particularly useful for creating smooth entry animations into looping sequences without affecting loop-to-loop transitions. The `start_color` only affects the first step if `TransitionStyle != Step`.

### Landing Color for Completion

For finite sequences, you can specify a `landing_color` to display after all loops complete:

```rust
let sequence = RgbSequence::builder()
    .step(Srgb::new(1.0, 0.0, 0.0), Duration::from_millis(500), TransitionStyle::Step)?  // Red
    .step(Srgb::new(0.0, 1.0, 0.0), Duration::from_millis(500), TransitionStyle::Step)?  // Green
    .loop_count(LoopCount::Finite(3))
    .landing_color(Srgb::new(0.0, 0.0, 1.0))  // Turn blue when done
    .build()?;
```

**Behavior:**
- The sequence blinks red/green 3 times
- After completion, the LED turns blue and stays blue

If no `landing_color` is specified, the LED holds the last step's color. The `landing_color` is ignored for infinite sequences.

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

If no Loop Count is specified, the sequence will default to `LoopCount::Finite(1)`

## Choosing Sequence Capacity

Sequences use a const generic parameter `N` to determine maximum step capacity at compile time. For convenience, the library provides type aliases for common sizes:

**Type Aliases:**
```rust
// Sequences
RgbSequence4<D>   // Up to 4 steps
RgbSequence8<D>   // Up to 8 steps
RgbSequence16<D>  // Up to 16 steps

// Sequencers
RgbSequencer4<'t, I, L, T>   // Up to 4 steps
RgbSequencer8<'t, I, L, T>   // Up to 8 steps
RgbSequencer16<'t, I, L, T>  // Up to 16 steps
```

**Guidelines:**
- **4 steps**: Simple patterns (blink, pulse, 2-3 color cycles)
- **8 steps**: Most animations (multi-color sequences, basic effects)
- **16 steps**: Complex sequences (rainbow cycles, elaborate shows)
- **32+ steps**: Use explicit `RgbSequence<D, N>` for data-driven animations
- **0 steps**: Function-based sequences (`RgbSequence<D, 0>`) - no step storage needed

**Examples:**
```rust
// Using type alias for step-based sequence
let sequence = RgbSequence8::builder()
    .step(red, ms(500), TransitionStyle::Linear)?
    .step(blue, ms(500), TransitionStyle::Linear)?
    .build()?;

// Function-based sequence needs no step storage
let sequence = RgbSequence::<Duration, 0>::from_function(
    white,
    breathing_effect,
    continuous_timing,
);

// Custom capacity for elaborate sequences
let sequence = RgbSequence::<_, 32>::builder()
    // ... 32 color waypoints
    .build()?;
```

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
        base.blue * brightness
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
- Simple sequences like setting static colors or blinking
- You only need instant color changes or linear transitions
- You have a fixed set of color waypoints
- Your animation fits naturally into discrete stages

Use function-based sequences when:
- You need smooth mathematical animations (sine waves, easing functions)
- You want algorithmic patterns that don't fit into discrete steps
- You want to reuse the same animation logic with different colors
- Your animation depends on complex calculations

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
match sequencer.state() {
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

For function-based sequences, `service()` calls the [timing function](#2-timing-function-fnduration---optionduration) internally and forwards its return value.

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

See [Embassy Rainbow Capture example](../examples/stm32f0-embassy/README.md) for a complete implementation.

### Pattern 3: Using Commands for Multi-LED Control

For task-based systems, you can use the command-based control pattern to route commands to multiple sequencers. See the [Command-Based Control](#command-based-control) section below for details and examples.

## Command-Based Control

For task-based systems (Embassy, RTOS, async runtimes), the command-based pattern decouples control logic from LED servicing by using message passing. This pattern works for both single and multi-LED scenarios.

### Core Concept

The `SequencerCommand<ID, D, N>` type packages an action with a target identifier:

```rust
pub struct SequencerCommand<ID, D, const N: usize> {
    pub led_id: ID,
    pub action: SequencerAction<D, N>,
}
```

`SequencerAction` represents operations like `Load`, `Start`, `Pause`, `Resume`, `Stop`, etc.

### Single LED Example

Decouple button handling from LED servicing:

```rust
use rgb_sequencer::{SequencerCommand, SequencerAction};

// Use () as the LED ID for single-LED scenarios
static COMMAND_CHANNEL: Channel<SequencerCommand<(), Duration, 8>, 4> = Channel::new();

// Button handler task
#[embassy_executor::task]
async fn button_task() {
    loop {
        button.wait_for_press().await;

        // Send pause command
        COMMAND_CHANNEL.send(SequencerCommand::new(
            (),  // Single LED, no ID needed
            SequencerAction::Pause,
        )).await;
    }
}

// RGB servicing task
#[embassy_executor::task]
async fn rgb_task(led: MyLed, timer: &'static MyTimer) {
    let mut sequencer = RgbSequencer8::new(led, timer);

    loop {
        select! {
            command = COMMAND_CHANNEL.receive() => {
                sequencer.handle_action(command.action)?;
            }
            _ = Timer::after(Duration::from_millis(16)) => {
                if sequencer.is_running() {
                    sequencer.service()?;
                }
            }
        }
    }
}
```

### Multi-LED Example

Route commands to different LEDs:

```rust
use rgb_sequencer::{SequencerCommand, SequencerAction};

// Define LED identifiers
enum LedId { Led1, Led2, Led3 }

// Create command channel
static COMMAND_CHANNEL: Channel<SequencerCommand<LedId, Duration, 8>, 4> = Channel::new();

// Control task - sends commands
#[embassy_executor::task]
async fn control_task() {
    // Load different sequences on different LEDs
    COMMAND_CHANNEL.send(SequencerCommand::new(
        LedId::Led1,
        SequencerAction::Load(rainbow_sequence),
    )).await;

    COMMAND_CHANNEL.send(SequencerCommand::new(
        LedId::Led2,
        SequencerAction::Load(pulse_sequence),
    )).await;

    // Start all LEDs
    for led_id in [LedId::Led1, LedId::Led2, LedId::Led3] {
        COMMAND_CHANNEL.send(SequencerCommand::new(
            led_id,
            SequencerAction::Start,
        )).await;
    }
}

// RGB task - handles commands and services sequencers
#[embassy_executor::task]
async fn rgb_task(/* ... */) {
    let mut sequencers = [sequencer1, sequencer2, sequencer3];

    loop {
        select! {
            command = COMMAND_CHANNEL.receive() => {
                let sequencer = match command.led_id {
                    LedId::Led1 => &mut sequencers[0],
                    LedId::Led2 => &mut sequencers[1],
                    LedId::Led3 => &mut sequencers[2],
                };
                sequencer.handle_action(command.action)?;
            }
            _ = Timer::after(Duration::from_millis(16)) => {
                for sequencer in &mut sequencers {
                    if sequencer.is_running() {
                        sequencer.service()?;
                    }
                }
            }
        }
    }
}
```

### Benefits

- **Separation of concerns**: Control logic separate from LED servicing
- **Task safety**: Commands cross task boundaries safely
- **Flexibility**: Easy to add new control inputs (buttons, UART, network)
- **Testability**: Control logic can be tested independently

See [Embassy Mode Switcher example](../examples/stm32f0-embassy/README.md) for a complete implementation.

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

### Event Detection with Position Tracking

For step-based sequences, you can detect when the sequencer enters a new step or starts a new loop iteration:

```rust
// Simple event detection using current_position()
let mut last_position = None;

loop {
    sequencer.service()?;

    let current = sequencer.current_position();
    if current != last_position {
        if let Some((step, loop_num)) = current {
            println!("Entered step {} in loop {}", step, loop_num);
            // Trigger event, play sound, update UI, etc.
        }
        last_position = current;
    }

    sleep_ms(16);
}

// Detailed timing using find_step_position()
if let Some(sequence) = sequencer.current_sequence() {
    let elapsed = sequencer.elapsed_time().unwrap();

    if let Some(pos) = sequence.find_step_position(elapsed) {
        println!("Step {}: {}ms in, {}ms remaining",
            pos.step_index,
            pos.time_in_step.as_millis(),
            pos.time_until_step_end.as_millis()
        );
    }
}
```

These methods are useful for:
- **Event detection**: Trigger actions when entering specific steps (play sounds, update UI, log events)
- **Color capture**: Getting the current color to create derived sequences
- **Synchronization**: Coordinating multiple LEDs based on elapsed time or step position
- **Progress tracking**: Displaying sequence progress with precise timing information
- **Debugging**: Inspecting sequence state during development

Note: `current_position()` returns `None` for function-based sequences since they don't have discrete steps.

## Global Brightness Control

The sequencer supports global brightness control, allowing you to dim or brighten all colors without modifying the sequence itself.

### Basic Usage

```rust
let mut sequencer = RgbSequencer8::new(led, &timer);
sequencer.load(sequence);

// Set brightness to 50%
sequencer.set_brightness(0.5);

sequencer.start()?;
```

### Brightness Range

- **`1.0`** (default): Full brightness
- **`0.5`**: 50% brightness
- **`0.0`**: LED off (black)
- Values are automatically clamped to 0.0-1.0 range

```rust
// These are automatically clamped
sequencer.set_brightness(2.5);   // Becomes 1.0 (full)
sequencer.set_brightness(-0.5);  // Becomes 0.0 (off)

// Query current brightness
let current = sequencer.brightness();  // Returns 0.0-1.0
```

Brightness can be changed at any time, including during playback:

### How It Works

Brightness is applied as a multiplier to all color channels after sequence evaluation:

```rust
// Evaluated sequence color: Srgb::new(1.0, 0.5, 0.0) (orange)
// Brightness: 0.5
// Actual LED color: Srgb::new(0.5, 0.25, 0.0) (dimmed orange)
```

This means:
- Brightness affects all sequences uniformly (step-based and function-based)
- Works with all transition styles
- Timing is unaffected - only colors are scaled
- Minimal performance overhead (single multiplication per channel)

### Use Cases

- Night Mode
- Battery Saving
- Ambient Light Adaptation
- Fade In/Out Effects
