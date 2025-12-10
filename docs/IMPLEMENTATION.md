# Implementation Details

Technical documentation for contributors and maintainers. Explains internal algorithms, design decisions, and implementation patterns.

## Overview

The library implements two distinct animation approaches with a unified sequencer interface:

1. **Step-based sequences** - Pre-defined color waypoints stored in fixed-capacity vectors
2. **Function-based sequences** - Algorithmic color computation via user-provided functions

Both approaches evaluate to `(Srgb, Option<Duration>)` tuples at runtime (when calling `RgbSequence::evaluate()`), enabling consistent handling by the sequencer.

## Color Interpolation

### Interpolation Algorithm

The library interpolates colors in the sRGB color space using `palette::Srgb<f32>` with component values in the 0.0-1.0 range. This approach prioritizes computational efficiency over perceptual accuracy, making it suitable for embedded targets with hardware floating-point units.

**Algorithm Overview:**

1. **Calculate normalized progress** within the current step:
   ```rust
   let progress = (time_in_step_ms as f32) / (step_duration_ms as f32);
   let progress = progress.clamp(0.0, 1.0);  // Ensure [0.0, 1.0] range
   ```

2. **Apply easing function** to transform progress curve:
   ```rust
   let eased_progress = apply_easing(progress, transition_style);
   ```

3. **Interpolate** between source and target colors:
   ```rust
   let result = previous_color.mix(target_color, eased_progress);
   ```

The `mix()` method from `palette` performs component-wise linear interpolation:
```rust
result.red   = previous.red   + (target.red   - previous.red)   * eased_progress
result.green = previous.green + (target.green - previous.green) * eased_progress
result.blue  = previous.blue  + (target.blue  - previous.blue)  * eased_progress
```

**Design Trade-offs:**

- **sRGB color space**: Perceptually non-uniform but computationally simple (3 multiplies, 3 adds)
- **Linear RGB interpolation**: May produce darker midpoints (e.g., red→green passes through brown), but avoids expensive gamma correction or LAB color space conversions
- **f32 precision**: Sufficient for LED control while enabling hardware FPU acceleration on Cortex-M4F/M7/M33

For targets without FPU, consider using `TransitionStyle::Step` to avoid interpolation overhead entirely.

### Easing Functions

The library supports five transition styles that modify the interpolation progress curve:

**`TransitionStyle::Step`**
No interpolation. Returns target color immediately, holds for duration.

**`TransitionStyle::Linear`**
Linear interpolation with constant velocity: `f(t) = t`

**`TransitionStyle::EaseIn`** (Quadratic)
Slow start, accelerating toward end: `f(t) = t²`

**`TransitionStyle::EaseOut`** (Quadratic)
Fast start, decelerating toward end: `f(t) = t × (2 - t)`

**`TransitionStyle::EaseInOut`** (Quadratic)
Slow start and end, fast middle:
```rust
if t < 0.5 {
    f(t) = 2 × t²
} else {
    f(t) = -1 + (4 - 2 × t) × t
}
```

All easing functions use quadratic interpolation for computational efficiency. More complex easing (cubic, sinusoidal, etc.) can be implemented via function-based sequences.

### Source Color Selection

For all interpolating transitions (`Linear`, `EaseIn`, `EaseOut`, `EaseInOut`), determining the interpolation source color follows three cases:

**Case 1: Smooth Entry (First Step, First Loop)**
```rust
if step_index == 0
   && current_loop == 0
   && start_color.is_some()
   && matches!(transition,
       TransitionStyle::Linear | TransitionStyle::EaseIn
       | TransitionStyle::EaseOut | TransitionStyle::EaseInOut)
{
    previous_color = start_color  // Smooth entry from initial state
}
```

Allows sequences to gracefully transition from LED's current state (or OFF) into the first step on initial playback. Example:

```rust
// LED is currently off (0,0,0)
.start_color(Srgb::new(0.0, 0.0, 0.0))  // Start from black
.step(Srgb::new(1.0, 0.0, 0.0), ms(1000), TransitionStyle::Linear)?  // Fade to red
```

**Case 2: Wrap-Around (First Step, Subsequent Loops)**
```rust
else if step_index == 0 {
    previous_color = last_step.color  // Seamless looping
}
```

Creates seamless wrap-around by transitioning from sequence end back to start. The last step's color becomes the interpolation source for the first step's transition.

**Case 3: Sequential (All Other Steps)**
```rust
else {
    previous_color = steps[step_index - 1].color  // Standard progression
}
```

Standard sequential interpolation from one step to the next.

## Sequence Evaluation

### Step Position Calculation

The core algorithm uses modulo arithmetic for efficient looping without iteration tracking:

```rust
let elapsed_ms = elapsed.as_millis();
let loop_duration_ms = sequence.loop_duration.as_millis();

// Which iteration? (0-indexed)
let current_loop = elapsed_ms / loop_duration_ms;

// Position within current iteration
let time_in_loop = elapsed_ms % loop_duration_ms;
```

**Example:** With loop_duration = 1000ms and elapsed = 2500ms:
- `current_loop = 2` (third iteration, 0-indexed)
- `time_in_loop = 500ms` (halfway through current iteration)

This works identically for both `Finite` and `Infinite` loop counts. Finite sequences check completion separately before this calculation.

### Finding Active Step

Once `time_in_loop` is known, the implementation walks through steps sequentially, accumulating durations:

```rust
let mut accumulated_time = 0ms;

for (step_idx, step) in steps.iter().enumerate() {
    let step_end_time = accumulated_time + step.duration;

    if time_in_loop < step_end_time {
        // Found it! time_in_loop falls within this step
        let time_in_step = time_in_loop - accumulated_time;
        let time_until_end = step_end_time - time_in_loop;
        return StepPosition { ... };
    }

    accumulated_time = step_end_time;
}
```

**Example:** Steps of 100ms, 200ms, 150ms create ranges:
- Step 0: [0, 100)ms
- Step 1: [100, 300)ms
- Step 2: [300, 450)ms

If `time_in_loop = 250ms`, the loop identifies Step 1 with `time_in_step = 150ms` and `time_until_end = 50ms`.

### Edge Cases

#### Zero-Duration Sequences

When all steps have zero duration (`loop_duration = 0`):
- At `elapsed = 0`: Shows first step color
- At `elapsed > 0`: Sequence complete, shows last step color (or landing color if set)

```rust
// Two zero-duration steps
let seq1 = RgbSequence::builder()
    .step(RED, Duration::ZERO, TransitionStyle::Step)?
    .step(BLUE, Duration::ZERO, TransitionStyle::Step)?
    .build()?;

// One step + landing color - equivalent behavior
let seq2 = RgbSequence::builder()
    .step(RED, Duration::ZERO, TransitionStyle::Step)?
    .landing_color(BLUE)
    .build()?;

// Both behave identically:
// t=0:  (RED, Some(0))
// t>0:  (BLUE, None) - complete
```

**Note:** Zero-duration only valid with `TransitionStyle::Step`.

#### Time Beyond Loop End

If `time_in_loop` exceeds total loop duration (possible with floating-point rounding), the fallback returns the last step at its end position. This ensures valid positions even with timing imprecision.

## Function-Based Sequences

### Function Signatures

**Color Function:**
```rust
fn(base_color: Srgb, elapsed: D) -> Srgb
```
Receives base color and elapsed time, returns current color. Allows reusing same function with different base colors.

**Timing Function:**
```rust
fn(elapsed: D) -> Option<D>
```
Returns next service time:
- `Some(Duration::ZERO)` → continuous updates
- `Some(duration)` → wait this long
- `None` → animation complete

### Evaluation Path

Function-based sequences bypass step evaluation entirely:

```rust
pub fn evaluate(&self, elapsed: D) -> (Srgb, Option<D>) {
    if let (Some(color_fn), Some(timing_fn)) = (self.color_fn, self.timing_fn) {
        let base = self.start_color.unwrap_or(COLOR_OFF);
        return (color_fn(base, elapsed), timing_fn(elapsed));
    }

    // ... step-based evaluation
}
```

Direct function invocation avoids all step lookup overhead.

## Pause/Resume Timing Compensation

The pause/resume mechanism maintains perfect timing continuity as if the pause never occurred.

### Algorithm

```rust
// On pause
pause_start_time = time_source.now();
state = Paused;

// On resume
let pause_duration = time_source.now() - pause_start_time;
start_time = start_time + pause_duration;  // Shift reference point
state = Running;
```

The sequence's `start_time` reference point shifts forward by the pause duration, effectively removing the paused period from elapsed time calculations.

### Timer Overflow Handling

On 32-bit systems with wrapping timers, `checked_add` may fail:

```rust
let old_start = self.start_time.unwrap();
self.start_time = Some(
    old_start.checked_add(pause_duration)
        .unwrap_or(old_start)  // Graceful degradation
);
```

If overflow occurs while the sequencer is paused, the implementation retains the original start time. The sequence jumps forward to account for both pause and overflow duration, but the system remains stable.

## Service Timing Hints

The sequencer returns timing hints via `ServiceTiming` to enable power-efficient operation without busy-waiting.

### Hint Types

**`Continuous`**
Returned during interpolating transitions:
- `TransitionStyle::Linear`
- `TransitionStyle::EaseIn`
- `TransitionStyle::EaseOut`
- `TransitionStyle::EaseInOut`
- Function-based sequences with `timing_fn` returning `Some(Duration::ZERO)`

Application should service at desired frame rate (e.g., 16ms for 60 FPS).

**`Delay(duration)`**
Returned during static color holds:
- `TransitionStyle::Step`
- Function-based sequences with `timing_fn` returning `Some(duration)`

Application should sleep exactly `duration` before next service call. The LED won't change until this time expires.

**`Complete`**
Returned when sequence finishes:
- Finite sequences complete all loops
- Function-based sequences with `timing_fn` returning `None`

No further servicing needed. LED displays landing color or last step color.

## State Machine

The sequencer implements strict state validation to prevent invalid operations.

### State Definitions

| State | Meaning | LED State |
|-------|---------|-----------|
| `Idle` | No sequence loaded | Off |
| `Loaded` | Sequence ready to start | Off |
| `Running` | Sequence executing | Animated |
| `Paused` | Execution suspended | Frozen at current color |
| `Complete` | Finite sequence finished | Landing color or last step |

### Transition Table

| From | Method | To | Notes |
|------|--------|-----|-------|
| `Idle` | `load()` | `Loaded` | Accepts sequence |
| `Loaded` | `start()` | `Running` | Begins execution |
| `Running` | `pause()` | `Paused` | Records pause time |
| `Paused` | `resume()` | `Running` | Applies timing compensation |
| `Running` | `service()` | `Complete` | Only for finite sequences |
| `Running` | `stop()` | `Loaded` | Turns LED off |
| `Paused` | `stop()` | `Loaded` | Turns LED off |
| `Complete` | `stop()` | `Loaded` | Turns LED off |
| `Running` | `restart()` | `Running` | Resets to beginning |
| `Paused` | `restart()` | `Running` | Resets to beginning |
| `Complete` | `restart()` | `Running` | Resets to beginning |
| Any | `load()` | `Loaded` | Replaces sequence |
| Any | `clear()` | `Idle` | Removes sequence, LED off |

Invalid transitions return `SequencerError::InvalidState` with expected and actual states.

## LED Update Optimization

The sequencer maintains `current_color: Srgb` and uses epsilon-based comparison before hardware writes:

```rust
let (new_color, timing) = sequence.evaluate(elapsed);

// Compare using epsilon threshold (0.001) to handle f32 imprecision
if !colors_approximately_equal(new_color, self.current_color) {
    self.led.set_color(new_color);
    self.current_color = new_color;
}
```

This optimization:
- Reduces unnecessary hardware writes during static color holds
- Uses epsilon comparison (threshold: 0.001) to handle floating-point rounding errors
- Prevents spurious LED updates from imperceptible color differences (<0.1%)
- Particularly valuable for slow SPI/I2C LED drivers
- Enables safe repeated `service()` calls without time advancement

## Validation and Safety

### Sequence Builder Validation

The builder validates at `build()` time:

1. **Empty Sequence Check**
   ```rust
   if steps.is_empty() {
       return Err(SequenceError::EmptySequence);
   }
   ```

2. **Zero Duration with Interpolating Transitions**
   ```rust
   for step in steps {
       if step.duration == 0 && matches!(step.transition, Linear | EaseIn | EaseOut | EaseInOut) {
           return Err(SequenceError::ZeroDurationWithLinear);
       }
   }
   ```
   All interpolating transitions require non-zero duration. Zero-duration steps only valid with `Step` transition.

### Capacity Enforcement

Capacity `N` is const generic, checked at compile time for type safety. Runtime check during `step()`:

```rust
if steps.push(step).is_err() {
    panic!("sequence capacity ({}) exceeded", N);
}
```

Panic includes helpful message indicating the capacity limit. Users must specify correct `N` upfront.

## Memory Layout

### Step-Based Sequences

```rust
struct RgbSequence<D, const N: usize> {
    steps: Vec<SequenceStep<D>, N>,  // heapless::Vec, inline storage
    loop_count: LoopCount,            // 8 bytes (4 bytes enum tag + 4 bytes u32)
    start_color: Option<Srgb>,        // 16 bytes (4 bytes tag + 12 bytes RGB)
    landing_color: Option<Srgb>,      // 16 bytes
    loop_duration: D,                 // Duration size (e.g., 8 bytes)
    color_fn: Option<fn(...)>,        // 8 bytes (function pointer)
    timing_fn: Option<fn(...)>,       // 8 bytes
}
```

`heapless::Vec` stores elements inline with fixed capacity. Total size scales with `N`.

### Function-Based Sequences

For function-based sequences, `N=0` is allowed:

```rust
RgbSequence::<Duration, 0>::from_function(base, color_fn, timing_fn)
```

The `steps` vector remains empty, relying solely on function pointers. Minimal memory overhead.

### Pre-Computed Values

The builder caches `loop_duration` at build time:

```rust
let total_ms: u64 = steps.iter().map(|s| s.duration.as_millis()).sum();
let loop_duration = D::from_millis(total_ms);
```

Avoids repeated summation during every `evaluate()` call.
