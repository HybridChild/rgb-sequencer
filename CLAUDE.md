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
- **No hard-wrapping** - Write each markdown paragraph as one long line and let editors soft-wrap it. Never reflow prose to a fixed column. Hard wrapping produces noisy diffs, since changing one word re-wraps every following line of the paragraph.

**When updating docs:**
1. Remove redundant explanations across files
2. Use tables/lists instead of verbose prose
3. Keep README.md focused on quick start and core concepts
4. Include performance implications for embedded users

---

## Usage Patterns

[docs/FEATURES.md](docs/FEATURES.md) is the reference for how the API is used: step-based and function-based sequences, transition styles, capacity, colors, the state machine, servicing, brightness, multi-LED and command-based control. Read it before writing example code, and correct it there rather than restating it here.

Points that are easy to get wrong and that the feature guide does not spell out:

- `TimeSource` is generic over the instant type - `impl TimeSource<MyInstant> for MyTimer`, not an associated type.
- Function-based sequences use `N = 0`; there is no step storage to size.
- Capacity `N` is a maximum, not an exact count. `step()` returns `SequenceError::CapacityExceeded` once `N` steps have been added.
- The library itself is float-free. A user-supplied effect function may use `f32` and `libm` - the constraint applies to `src/`.

---

## Critical Constraints

### `no_std` Environment
- **No heap allocation** - Use `heapless::Vec<T, N>` for sequences
- **Fixed capacity at compile time** - Specify `N` in `RgbSequence<D, N>`
- **Core dependencies only** - Check `default-features = false` for dependencies
- **Tests are also `no_std`** - Maintain consistency across library and tests

### Performance Characteristics

All color math is fixed-point integer arithmetic (Q0.15 for progress and easing, `u16` per channel). No soft-float emulation is linked in on any target.

Flash footprint measured with `tools/binary-analyzer`:
- **Non-FPU (thumbv6m, Cortex-M0/M0+)**: 2104 B
- **FPU (thumbv7em, Cortex-M4F/M7)**: 1828 B

`service()` cycles measured with `tools/benchmark/` on RP2040 (Cortex-M0+) and RP2350 (Cortex-M33F); see the committed result files for the full tables.

**When providing guidance:**
- Don't warn about easing costing more than Linear - measured at ~15 cycles across all four curves, under 1% and inside run-to-run noise
- Don't suggest Step transitions "for performance" on non-FPU targets; that advice belonged to the f32 implementation
- Capacity dominates timing, not transition style - the step search is O(N), and N=32 costs roughly 5x N=4
- Avoid speculative performance claims; the benchmark files hold real numbers, use them

### Static Allocation & Zero-Copy

- **Compile-time capacity** - `RgbSequence<D, N>` where `N` is max steps
- **Step-based**: Stores steps in `heapless::Vec<SequenceStep<D>, N>`
- **Function-based**: Zero storage (`N=0`), generates colors on-demand
- **Stack-based** - Sequences live on stack or as statics

### Color Handling

- **Color type**: Always `Rgb` - three `u16` channels, `0..=65535`
- **Construction**: `Rgb::from_u8(255, 128, 0)` for 8-bit sources, `Rgb::new` for 16-bit. Both are `const`
- **Hardware conversion**: Convert in `RgbLed::set_color()` to native format; `Rgb::to_u8()` for 8-bit drivers
- **Interpolation**: Per-channel linear interpolation (perceptually incorrect but fast). Note it operates on gamma-encoded values, matching the previous behaviour - do not introduce gamma conversion
- **Fixed-point helpers**: `src/fixed.rs` holds `ONE`/`HALF`/`mul_q15`/ `progress_q15`/`lerp_channel`/`scale_channel`/`div_65535`. Keep every intermediate inside a `u32` - a 64-bit divide alone costs ~900 B of `compiler_builtins`
- **Two scales, not one**: channel values and `scale` factors run to 65535; interpolation and easing factors are Q0.15 and run to 32768. `FULL` is the former. Do not "unify" them - `ONE = 32768` is what keeps a squared easing term inside a `u32`

---

## Core Architecture

The public shape of the state machine, the builder and the service timing hints is documented in [docs/FEATURES.md](docs/FEATURES.md). What follows is the reasoning behind it, which that guide does not carry.

### State Machine Pattern

`RgbSequencer` moves between `Idle`, `Loaded`, `Running`, `Paused` and `Complete`, and rejects operations that do not apply to the current state with `SequencerError::InvalidState`. State transition methods (`start()`, `resume()`, `restart()`) change state only - the LED is written by `service()`, so several sequencers can be transitioned together and then serviced in one pass.

### Trait-Based Abstraction

`RgbLed`, `TimeSource`, `TimeInstant` and `TimeDuration` are generic parameters rather than trait objects. There are no vtables and no dynamic dispatch, so a `service()` call inlines down to the hardware write.

### Timing Compensation on Pause/Resume

Color is derived from elapsed time rather than accumulated per-frame steps, so pausing has to move the origin rather than remember a position:

```rust
// On pause: record pause time
pause_time = time_source.now();

// On resume: adjust start time to skip the paused duration
let paused_duration = time_source.now().duration_since(pause_time);
start_time = start_time.checked_add(paused_duration)?;
```

Animations then continue from where they stopped, and no drift accumulates across pauses.

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
let sequence = RgbSequence::<_, 4>::builder()  // Capacity covers the steps added
    .step(color1, dur1, TransitionStyle::Step)?
    .step(color2, dur2, TransitionStyle::Step)?
    .step(color3, dur3, TransitionStyle::Step)?
    .step(color4, dur4, TransitionStyle::Step)?
    .build()?;
```

### ❌ Zero-Duration with Interpolating Transitions
```rust
// WRONG - Validation error (all interpolating transitions require non-zero duration)
.step(color, Duration::zero(), TransitionStyle::Linear)    // Invalid!
.step(color, Duration::zero(), TransitionStyle::EaseIn)    // Invalid!
.step(color, Duration::zero(), TransitionStyle::EaseOut)   // Invalid!
.step(color, Duration::zero(), TransitionStyle::EaseInOut) // Invalid!

// RIGHT
.step(color, Duration::zero(), TransitionStyle::Step)  // OK - only Step allows zero duration
```

### ❌ Reintroducing Floating Point into the Library
```rust
// WRONG - drags __divsf3/__addsf3/__mulsf3 back into every no_std build
fn interpolate(a: Rgb, b: Rgb, t: f32) -> Rgb { /* ... */ }

// RIGHT - Q0.15 integer factor
fn interpolate(a: Rgb, b: Rgb, t: u16) -> Rgb { a.lerp(b, t) }
```

User-authored effect functions may still use `f32` and `libm` - that is their choice. The constraint applies to `src/`.

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
// WRONG - channels are 0..=65535, not 0..=255
Rgb::new(255, 128, 64)   // almost black

// RIGHT
Rgb::from_u8(255, 128, 64)
Rgb::new(65535, 32896, 16448)
```

---

## Testing Approach

### Test Organization

Tests are organized as **integration tests** in the `tests/` directory:
- `tests/sequence_tests.rs`: Tests for sequence validation, evaluation, looping
- `tests/sequencer_tests.rs`: Tests for state machine, timing, operations
- `tests/color_tests.rs`: Tests for the `Rgb` color type
- `tests/colors_tests.rs`: Tests for HSV color conversion helpers
- `tests/easing_tests.rs`: Tests for transition curves and within-step progress
- `tests/common/mod.rs`: Shared test infrastructure (mocks, helpers, constants)

**Total: 117 integration tests**

This organization keeps source files clean and provides true black-box testing of the public API. **`src/` contains no `#[cfg(test)]` blocks, and none should be added.**

Private internals are still covered, by reaching them through the public API rather than by exposing them. `src/fixed.rs` and the easing curves are private, so `tests/easing_tests.rs` drives them through a single BLACK -> WHITE step whose duration matches the Q0.15 scale: the red channel then reads back the eased progress directly, at finer resolution than the value being checked. A mutation of 4 parts in 32768 is detected. Reach for that pattern before concluding something "can only be unit-tested in-file".

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
cargo test                        # Run all tests
cargo test --test sequence_tests  # Run sequence tests only
cargo test --test sequencer_tests # Run sequencer tests only
cargo test --test color_tests     # Run Rgb color type tests only
cargo test --test colors_tests    # Run HSV helper tests only
cargo test --test easing_tests    # Run transition curve tests only
```

---

## Build Commands

```bash
# Fast check
cargo check

# Run tests
cargo test                         # All tests
cargo test --test '*'              # Integration tests only

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
├── color.rs        # Rgb color type (u16 per channel)
├── colors.rs       # Integer HSV helpers
├── fixed.rs        # Q0.15 fixed-point primitives (private)
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
├── binary-analyzer/    # Binary analyzer for embedded targets
└── benchmark/          # service() cycle counts on RP2040 and RP2350
```

---

## Contributing Workflow

**Library Status:** Active development

**For contributions:**
1. Follow existing architectural patterns (see above)
2. Write comprehensive tests for new features
3. Run `cargo test` to verify all tests pass
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
- **Color type:** "`Rgb`" (not "RGB" or "sRGB" in code context)
- **Timing:** "time system" (not "timer" - encompasses SysTick, HAL timers, Embassy time driver, etc.)

---

**This repository is maintained as a professional library for embedded RGB LED control. Efficiency, clarity, and reliability are priorities.**
