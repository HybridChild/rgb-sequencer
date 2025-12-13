# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Development Approach

**rgb-sequencer** is an embedded RGB LED animation library. The core functionality is complete and well-tested.

**Maintenance Philosophy:**
- **Stability over features** - Avoid unnecessary changes to working code
- **Lean documentation** - Keep docs concise and professional; eliminate redundancy
- **Follow established patterns** - See architecture sections below
- **Test thoroughly** - All tests must pass, test coverage for new features required
- **Performance-conscious** - Profile on non-FPU targets, prefer efficiency

**Before adding features:** Consider if it fits the library's scope (lightweight embedded RGB control). Propose significant changes via issue discussion before implementation.

---

## Documentation Standards

This repository maintains **professional, lean documentation**:

- **No redundancy** - Each concept explained once, in the right place
- **No verbosity** - Concise language, respect reader's time
- **No speculation** - Document what exists, not future possibilities
- **Proper distribution** - README for getting started, code comments for implementation details

**When updating docs:**
1. Remove redundant explanations across files
2. Use tables/lists instead of verbose prose
3. Keep README.md focused on quick start and core concepts
4. Include performance implications for embedded users

---

## Quick Reference

### Creating a Step-Based Sequence

**Pattern:** Use `SequenceBuilder` to construct sequences with explicit color waypoints.

```rust
let sequence = RgbSequence::<_, 3>::builder()
    .step(Srgb::new(1.0, 0.0, 0.0), ms(1000), TransitionStyle::Linear)?
    .step(Srgb::new(0.0, 1.0, 0.0), ms(1000), TransitionStyle::Linear)?
    .step(Srgb::new(0.0, 0.0, 1.0), ms(1000), TransitionStyle::Linear)?
    .loop_count(LoopCount::Infinite)
    .build()?;
```

**Key points:**
- `N` (capacity) must match number of steps at compile time
- Zero-duration steps only valid with `TransitionStyle::Step`
- First loop uses `start_color` if set, subsequent loops use step 0
- Landing color used only when finite loops complete

### Easing Functions

**Pattern:** Use easing functions for smoother, more natural-looking transitions.

```rust
let sequence = RgbSequence::<_, 3>::builder()
    .start_color(Srgb::new(0.0, 0.0, 0.0))
    .step(Srgb::new(1.0, 0.0, 0.0), ms(1000), TransitionStyle::EaseIn)?     // Slow start
    .step(Srgb::new(0.0, 1.0, 0.0), ms(1000), TransitionStyle::EaseOut)?    // Slow end
    .step(Srgb::new(0.0, 0.0, 1.0), ms(1000), TransitionStyle::EaseInOut)?  // Slow both
    .build()?;
```

**Available easing types:**
- `TransitionStyle::Step` - Instant color change, hold for duration
- `TransitionStyle::Linear` - Constant-speed interpolation
- `TransitionStyle::EaseIn` - Slow start, accelerating (quadratic)
- `TransitionStyle::EaseOut` - Fast start, decelerating (quadratic)
- `TransitionStyle::EaseInOut` - Slow start and end, fast middle (quadratic)

**Key points:**
- All easing functions use quadratic interpolation (computationally efficient)
- Easing requires non-zero duration (like Linear)
- Returns `ServiceTiming::Continuous` (requires frequent updates)
- Performance note: Uses f32 math, consider impact on non-FPU targets

### Creating a Function-Based Sequence

**Pattern:** Use custom functions for algorithmic animations.

```rust
fn breathing_color(base: Srgb, t: Milliseconds) -> Srgb {
    let phase = (t.as_millis() as f32 * 0.001).sin() * 0.5 + 0.5;
    Srgb::new(base.red * phase, base.green * phase, base.blue * phase)
}

fn breathing_timing(_t: Milliseconds) -> Option<Milliseconds> {
    Some(Milliseconds::from_millis(16)) // ~60 FPS
}

let sequence = RgbSequence::<Milliseconds, 0>::from_function(
    Srgb::new(1.0, 1.0, 1.0),
    breathing_color,
    breathing_timing,
);
```

**Key points:**
- Use `N=0` for function-based sequences (no step storage needed)
- Color function: `fn(base_color: Srgb, elapsed: D) -> Srgb`
- Timing function: `fn(elapsed: D) -> Option<D>` (returns delay or None for complete)
- Avoid complex math on non-FPU targets

### Implementing Traits

**`RgbLed`** - Abstract LED hardware control:
```rust
impl RgbLed for MyLed {
    fn set_color(&mut self, color: Srgb) {
        // Convert 0.0-1.0 range to hardware format (PWM, SPI, etc.)
        let r = (color.red * 255.0) as u8;
        // ...write to hardware
    }
}
```

**`TimeSource`** - Abstract time querying:
```rust
impl TimeSource for MyTimer {
    type Instant = Milliseconds;
    fn now(&self) -> Self::Instant { /* ... */ }
}
```

**`TimeDuration` + `TimeInstant`** - Time arithmetic (see time.rs for full trait definitions)

See README.md and examples for complete usage patterns.

---

## Critical Constraints

### `no_std` Environment
- **No heap allocation** - Use `heapless::Vec<T, N>` for sequences
- **Fixed capacity at compile time** - Specify `N` in `RgbSequence<D, N>`
- **Core dependencies only** - Check `default-features = false` for dependencies
- **Tests are also `no_std`** - Maintain consistency across library and tests

### Performance on Non-FPU Targets

**WARNING**: This library uses `f32` extensively for color math. Performance varies dramatically:

- **Cortex-M4F/M7/M33** (with FPU): Excellent performance, hardware-accelerated f32
- **Cortex-M0/M0+/M3** (no FPU): Software-emulated f32 is 10-100x slower
- **RP2040** (Cortex-M0+): No FPU, software emulation

**For non-FPU targets:**
- Prefer `TransitionStyle::Step` (no interpolation math)
- `Linear` is acceptable for simple transitions (single multiply/divide)
- Avoid `EaseIn/EaseOut/EaseInOut` (additional f32 operations)
- Avoid complex function-based sequences
- Use simple color patterns
- Profile your specific target

**For FPU targets:**
- Full flexibility with all transition types
- Easing functions add minimal overhead
- Mathematical function-based sequences work well
- HSV color wheels, sine wave breathing, etc.

### Static Allocation & Zero-Copy

- **Compile-time capacity** - `RgbSequence<D, N>` where `N` is max steps
- **Step-based**: Stores steps in `heapless::Vec<SequenceStep<D>, N>`
- **Function-based**: Zero storage (`N=0`), generates colors on-demand
- **Stack-based** - Sequences live on stack or as statics

### Color Handling

- **Color type**: Always `palette::Srgb` (f32 RGB in 0.0-1.0 range)
- **Hardware conversion**: Convert in `RgbLed::set_color()` to native format
- **Interpolation**: Linear RGB interpolation (perceptually incorrect but fast)
- **Const colors**: Use `const BLACK: Srgb = Srgb::new(0.0, 0.0, 0.0)`

---

## Core Architecture

### State Machine Pattern

The `RgbSequencer` uses a state machine with explicit state transitions:

```rust
pub enum SequencerState {
    Idle,       // No sequence loaded
    Loaded,     // Sequence loaded but not started
    Running,    // Active animation
    Paused,     // Animation paused (timing compensation on resume)
    Complete,   // Finite sequence finished
}
```

**Valid Transitions:**
- `Idle` → `Loaded` (via `load()`)
- `Loaded` → `Running` (via `start()`)
- `Running` → `Paused` (via `pause()`)
- `Paused` → `Running` (via `resume()` - with timing compensation)
- `Running` → `Complete` (finite sequence finishes)
- Any state → `Idle` (via `clear()`)
- `Loaded/Running/Paused/Complete` → `Running` (via `restart()`)

**Invalid operations return `SequencerError::InvalidState`**

### Builder Pattern for Sequences

Use **method chaining** for fluent sequence construction:

```rust
RgbSequence::builder()
    .step(color1, duration1, transition1)?  // Required: at least 1 step
    .step(color2, duration2, transition2)?  // Add more steps
    .loop_count(LoopCount::Finite(3))       // Optional: default is Finite(1)
    .start_color(start)                     // Optional: smooth entry
    .landing_color(landing)                 // Optional: smooth exit
    .build()?                               // Validates and returns Result
```

**Validation rules:**
- At least one step required
- Zero-duration steps must use `TransitionStyle::Step`
- Capacity `N` must match number of steps

### Trait-Based Abstraction

**Platform Independence via Traits:**

1. **`RgbLed`** - Hardware abstraction (PWM, SPI, WS2812, etc.)
2. **`TimeSource`** - Timing system (SysTick, HAL timers, Embassy, etc.)
3. **`TimeInstant`** - Instant in time with arithmetic
4. **`TimeDuration`** - Duration between instants

**Zero-cost abstraction:**
- Generics enable compile-time polymorphism
- No vtables, no dynamic dispatch
- Inline-friendly for embedded optimization

### Timing Compensation on Pause/Resume

When pausing/resuming, the sequencer **compensates for elapsed time**:

```rust
// On pause: Record pause time
pause_time = time_source.now();

// On resume: Adjust start time to skip paused duration
let paused_duration = time_source.now().duration_since(pause_time);
start_time = start_time.checked_add(paused_duration)?;
```

This ensures animations continue smoothly without jumps or drift.

### Service Timing Hints

The `service()` method returns timing hints for power efficiency:

```rust
pub enum ServiceTiming<D: TimeDuration> {
    Continuous,  // Call service() as frequently as possible (transitioning)
    Delay(D),    // Can delay this duration before next service() (holding color)
    Complete,    // Sequence finished, no more service() needed
}
```

Allows applications to sleep/yield appropriately instead of busy-waiting.

---

## Common Pitfalls

### ❌ Using std Types in `no_std`
```rust
// WRONG
fn create_sequence() -> Vec<SequenceStep<Milliseconds>> { }

// RIGHT
fn create_sequence() -> heapless::Vec<SequenceStep<Milliseconds>, 8> { }
```

### ❌ Mismatched Capacity
```rust
// WRONG - Returns SequenceError::CapacityExceeded
let sequence = RgbSequence::<_, 3>::builder()
    .step(color1, dur1, TransitionStyle::Step)?
    .step(color2, dur2, TransitionStyle::Step)?
    .step(color3, dur3, TransitionStyle::Step)?
    .step(color4, dur4, TransitionStyle::Step)?  // 4 steps, capacity 3!
    .build()?;  // Previous step() call will error

// RIGHT
let sequence = RgbSequence::<_, 4>::builder()  // Capacity matches steps
    .step(color1, dur1, TransitionStyle::Step)?
    .step(color2, dur2, TransitionStyle::Step)?
    .step(color3, dur3, TransitionStyle::Step)?
    .step(color4, dur4, TransitionStyle::Step)?
    .build()?;
```

### ❌ Zero-Duration with Interpolating Transitions
```rust
// WRONG - Validation error
.step(color, Duration::zero(), TransitionStyle::Linear)    // Invalid!
.step(color, Duration::zero(), TransitionStyle::EaseIn)    // Invalid!

// RIGHT
.step(color, Duration::zero(), TransitionStyle::Step)  // OK - only Step allows zero duration
```

### ❌ Complex Math on Non-FPU Targets
```rust
// WRONG - Very slow on Cortex-M0
fn expensive_color(base: Srgb, t: Milliseconds) -> Srgb {
    let phase = (t.as_millis() as f32 * 0.001).sin();
    let hue = (phase * 360.0).rem_euclid(360.0);
    // HSV to RGB conversion with more f32 math...
}

// RIGHT - Use step-based sequences on non-FPU targets instead
```

### ❌ Forgetting to Service Sequencer
```rust
// WRONG - LED never updates!
sequencer.start()?;
// ... no service() calls ...

// RIGHT - Regular service() calls in main loop
loop {
    match sequencer.service()? {
        ServiceTiming::Delay(d) => delay(d),
        ServiceTiming::Complete => break,
        _ => {}
    }
}
```

### ❌ Incorrect Color Range
```rust
// WRONG - Srgb expects 0.0-1.0 range
Srgb::new(255.0, 128.0, 64.0)

// RIGHT
Srgb::new(1.0, 0.5, 0.25)

// Or convert from 8-bit
fn from_u8(r: u8, g: u8, b: u8) -> Srgb {
    Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}
```

---

## Testing Approach

### Test Organization

Tests are organized as **integration tests** in the `tests/` directory:
- `tests/sequence_tests.rs`: Tests for sequence validation, evaluation, looping
- `tests/sequencer_tests.rs`: Tests for state machine, timing, operations
- `tests/colors_tests.rs`: Tests for HSV color conversion helpers
- `tests/common/mod.rs`: Shared test infrastructure (mocks, helpers, constants)

**Total: 94 integration tests**

This organization keeps source files clean and provides true black-box testing of the public API.

### Shared Test Infrastructure

The `tests/common/` module provides reusable test utilities:
- **`TestDuration`/`TestInstant`** - Mock time types implementing time traits
- **`MockLed`** - Records all color changes for verification
- **`MockTimeSource`** - Controllable time advancement for deterministic testing
- **Color constants** - `RED`, `GREEN`, `BLUE`, `BLACK`, `YELLOW`
- **`colors_equal()`** - Floating-point color comparison with epsilon tolerance

### Key Testing Patterns

**Table-driven tests** for comprehensive coverage:
```rust
let test_cases = [
    (duration_ms, expected_color, "description"),
    // ...
];

for (duration, expected, desc) in test_cases {
    let result = sequence.evaluate(duration);
    assert_eq!(result, expected, "{}", desc);
}
```

**State transition tests** for state machine validation:
```rust
assert_eq!(sequencer.state(), SequencerState::Loaded);
sequencer.start()?;
assert_eq!(sequencer.state(), SequencerState::Running);

// Test invalid transitions
assert_eq!(sequencer.pause(), Err(SequencerError::InvalidState));
```

**Edge case coverage:**
- Zero-duration sequences
- Timer overflow handling
- Capacity limits
- Pause/resume timing accuracy

### Running Tests

```bash
cargo test                        # Run all tests (integration + unit)
cargo test --test sequence_tests  # Run sequence tests only
cargo test --test sequencer_tests # Run sequencer tests only
cargo test --test colors_tests    # Run color tests only
```

---

## Build Commands

```bash
# Fast check
cargo check

# Run tests
cargo test                         # All tests (integration + unit)
cargo test --test '*'              # Integration tests only
cargo test --lib                   # Unit tests only (currently none)

# Lint
cargo clippy --all-features -- -D warnings

# Format
cargo fmt

# Build examples
cd examples/stm32f0 && cargo build --release
cd examples/stm32f0-embassy && cargo build --release
cd examples/rp-pico && cargo build --release

# Memory analysis tools
cd tools/binary-analyzer && ./analyze.sh           # Binary size analysis (Flash/RAM)
cd tools/sizeof-calculator && cargo run --release  # Sizeof calculator (planning tool)
```

---

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `defmt` | Enable defmt logging support | Adds `defmt` dependency |

```toml
# Minimal (default)
rgb-sequencer = "0.1"

# With defmt
rgb-sequencer = { version = "0.1", features = ["defmt"] }
```

---

## Project Structure

```
src/
├── lib.rs          # Public API, module declarations, documentation
├── types.rs        # TransitionStyle, LoopCount, SequenceStep, errors
├── time.rs         # TimeSource, TimeInstant, TimeDuration traits
├── sequence.rs     # RgbSequence, SequenceBuilder, evaluation logic
├── sequencer.rs    # RgbSequencer, state machine, LED control
└── command.rs      # SequencerAction, SequencerCommand for routing

examples/
├── stm32f0/            # STM32F0 bare-metal examples
├── stm32f0-embassy/    # STM32F0 Embassy async examples
└── rp-pico/            # Raspberry Pi Pico examples

tools/
├── sizeof-calculator/  # Sizeof calculator for planning capacity/types
└── binary-analyzer/    # Binary analyzer for embedded targets
```

---

## Contributing Workflow

**Library Status:** Active development

**For contributions:**
1. Follow existing architectural patterns (see above)
2. Write comprehensive tests for new features
3. Run `cargo test --lib` to verify all tests pass
4. Run `cargo fmt` and `cargo clippy` before committing
5. Update documentation for API changes
6. Consider performance on non-FPU targets

**Documentation updates:**
- Keep explanations concise and technical
- Use code examples to demonstrate patterns
- Document performance implications
- Avoid redundancy across files

---

## Terminology Conventions

**Always use consistent terminology:**

- **Patterns:** "builder pattern", "state machine pattern", "trait-based abstraction"
- **Compound adjectives:** "step-based sequence", "function-based sequence", "zero-allocation design"
- **Code identifiers:** `RgbSequencer`, `RgbSequence`, `RgbLed`, `no_std`, `TransitionStyle::Linear`
- **Project name:** "rgb-sequencer" (kebab-case)
- **Feature names:** `std`, `defmt` (lowercase)
- **Color type:** "Srgb" or "`palette::Srgb`" (not "RGB" or "sRGB" in code context)
- **Timing:** "time system" (not "timer" - encompasses SysTick, HAL timers, Embassy time driver, etc.)

---

**This repository is maintained as a professional library for embedded RGB LED control. Efficiency, clarity, and reliability are priorities.**
