# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-07-31

Replaces all floating-point color math with fixed-point integer arithmetic and drops the `palette` dependency. On Cortex-M0/M0+ this removes the software float emulation routines (`__divsf3`, `__addsf3`, `__mulsf3`) entirely.

Measured with `tools/binary-analyzer` against a minimal reference binary:

| Target | 0.2.1 | 0.3.0 | Change |
|--------|-------|-------|--------|
| `thumbv6m-none-eabi` (Cortex-M0/M0+, no FPU) | 3784 B | 2104 B | **-44.4%** |
| `thumbv7em-none-eabihf` (Cortex-M4F/M7, FPU) | 1996 B | 1828 B | **-8.4%** |

RAM per sequence drops too — `SequenceStep<u32>` goes from 20 B to 12 B, and `RgbSequence<u32, 32>` from 712 B to 440 B.

### Breaking

- **Color type** — `palette::Srgb` is replaced by `rgb_sequencer::Rgb`, three `u16` channels running `0..=65535`. `palette` is no longer a dependency, so it can be removed from your `Cargo.toml` unless you use it directly.

  ```rust
  // Before
  Srgb::new(1.0, 0.5, 0.0)
  // After
  Rgb::from_u8(255, 128, 0)   // or Rgb::new(65535, 32768, 0)
  ```

- **`RgbLed::set_color`** now receives an `Rgb`. Convert to your hardware format with integer math:

  ```rust
  // Before
  let duty = (color.red.clamp(0.0, 1.0) * max_duty as f32) as u16;
  // After
  let duty = (color.r as u32 * max_duty as u32 / 65535) as u16;
  // ...or, for 8-bit drivers
  let (r, g, b) = color.to_u8();
  ```

- **Channel fields** are `r`, `g`, `b` rather than `red`, `green`, `blue`.

- **Brightness** is a `u16` where 65535 is full brightness: `set_brightness`, `brightness()` and `SequencerAction::SetBrightness` all changed, as did `Rgb::scale`. The type bounds the range, so out-of-range values are no longer clamped — they cannot be expressed. `0.5` becomes `32768`, `1.0` becomes `65535`. Sixteen bits keep dimming smooth at the bottom of the range, where adjacent factors differ by a single channel count.

- **`colors::hsv` and `colors::hue`** take `u16` arguments. Hue spans the full color wheel across the whole `u16` range, so rotation wraps by plain addition. Use the new `colors::degrees()` helper to convert: `hue(60.0)` becomes `hue(degrees(60))`.

- **`TransitionStyle::EaseOutIn`** is removed. The fast-slow-fast curve saw no use, and its match arm cost every build 56 B on Cortex-M0/M0+ and 32 B on Cortex-M4F/M7 whether or not the variant was named. `EaseInOut` is the nearest replacement; a function-based sequence can reproduce the original curve exactly.

- **`DEFAULT_COLOR_EPSILON`**, `with_epsilon`, `color_epsilon()` and `set_color_epsilon()` use `u16` per-channel thresholds instead of `f32`. The default is 64 out of 65535 (~0.1%), matching the previous `0.001`.

### Added

- `Rgb` with `new`, `from_u8`, `to_u8`, `lerp`, `scale` and `approx_eq` — all `const`, so palettes can be built at compile time.
- `colors::degrees()` for converting degrees to the `u16` hue wheel.
- `FULL` (`u16::MAX`) constant, serving as both the full-scale channel value and the unity brightness factor.
- Unit tests for the fixed-point primitives, including an exhaustive sweep of every easing curve across all 32769 progress values against its floating-point reference.

### Changed

- Progress, easing and interpolation use Q0.15 fixed point. Every intermediate fits in 32 bits, so no 64-bit multiply or divide helper is linked in.
- HSV conversion is integer sector math and needs no `libm`.
- CI now runs the integration test suite, which `cargo test --lib` had been skipping.

### Removed

- The `palette` dependency, and with it `libm` and `fast-srgb8`.

### Notes

Interpolation behaviour is unchanged. `palette::Srgb` was gamma-encoded and its `Mix` was a naive per-channel lerp, so the integer lerp is equivalent — no gamma conversion was introduced.

Measured on hardware against 0.2.1, `service()` is 25-43% faster on RP2040 (Cortex-M0+) and 5-8% faster on RP2350 (Cortex-M33F) at `N=4`. The margin narrows as capacity grows, to 5-13% and around 1% at `N=32`, because the O(N) step search was already integer work. The gap between `Linear` and the most expensive easing curve on Cortex-M0+ fell from 582 cycles to about 15.

## [0.2.1] - 2026-03-11

### Changed
- Updated dependencies in `stm32f0` and `rp-pico` examples
- Updated `stm32f0-embassy` examples for `embassy-stm32` 0.5.0 breaking API changes

### Added
- Dependabot configuration for automated dependency updates
- `documentation` and `readme` fields in `Cargo.toml` for crates.io metadata

## [0.2.0] - 2025-12-16

### Changed
- **BREAKING**: Color constants renamed from `COLOR_*` prefix to simple names (`RED`, `GREEN`, `BLUE`, `WHITE`, `YELLOW`, `CYAN`, `MAGENTA`). `COLOR_OFF` renamed to `BLACK`
- **BREAKING**: `RgbSequencer::current_position()` now returns `Option<Position>` instead of `Option<(usize, u32)>`
- `RgbSequencer::current_position()` now returns the frozen position when paused (previously returned `None`)
- **BREAKING**: Renamed `RgbSequencer::get_state()` to `state()` to follow Rust API naming conventions
- **BREAKING**: `SequenceBuilder::step()` now returns `Result<Self, SequenceError>` instead of panicking when capacity is exceeded
- **BREAKING**: `RgbSequence::solid()` signature changed to remove duration parameter (holds indefinitely)
- **BREAKING**: State transition methods (`start()`, `resume()`, `restart()`) no longer call `service()` internally - applications must explicitly call `service()` to update LED after state changes
- **BREAKING**: `SequenceError::ZeroDurationWithLinear` renamed to `ZeroDurationWithInterpolation` to reflect all interpolating transition styles
- License changed from MIT to dual MIT/Apache-2.0
- README updates for clarity and structure
- Test suite reorganized into dedicated `tests/` directory with integration tests
- Memory analysis tools consolidated and moved to `tools/` directory
- `.gitignore` updated to track `.cargo/config.toml` for examples and ignore `tmp/` directory
- Examples updated to use new convenience methods and type aliases

### Added
- Global brightness control via `RgbSequencer::brightness()`, `set_brightness()` and `SequencerAction::SetBrightness`
- Configurable color epsilon via `RgbSequencer::with_epsilon()`, `color_epsilon()`, and `set_color_epsilon()` for customizable color change detection sensitivity
- `DEFAULT_COLOR_EPSILON` constant for the default threshold value
- `Position` struct for representing playback position with named fields (`step_index`, `loop_number`)
- `colors` module with HSV color space helpers (`hsv()` and `hue()`) for more intuitive color creation
- `RgbSequencer::into_led()` and `into_parts()` methods for extracting LED from sequencer
- `RgbSequence::solid()` convenience method for creating single-color sequences
- `RgbSequencer::peek_next_timing()` method for checking timing hints without state mutation
- `#[inline]` attributes on all simple getter methods for better optimization
- Introduce CHANGELOG.md (this file)
- `CLAUDE.md` file for AI assistant guidance
- Comprehensive CI workflow for automated testing
- ci-local script for verifying CI workflow locally
- Size analysis script for tracking binary size impact
- Benchmark tool (`tools/benchmark/`) for profiling performance on embedded targets (RP2040, RP2350)
- Common color constants: `RED`, `GREEN`, `BLUE`, `WHITE`, `YELLOW`, `CYAN`, `MAGENTA`, `BLACK`
- `load_and_start()` convenience method for `RgbSequencer`
- Type aliases for common capacities: `RgbSequencer4/8/16`, `RgbSequence4/8/16`, `SequencerCommand4/8/16` and `SequencerAction4/8/16`
- `SequenceError::CapacityExceeded` variant for robust error handling
- Easing functions: `TransitionStyle::EaseIn`, `EaseOut`, `EaseInOut`, and `EaseOutIn` for smoother, more natural transitions using quadratic interpolation
- Example projects demonstrating transition styles and breathing effects
- `RgbSequencer::current_position()` method for event detection - enables tracking step and loop changes without callbacks
- `RgbSequence::find_step_position()` is now public for advanced use cases
- Builder validation: `start_color` is rejected with `TransitionStyle::Step`, `landing_color` is rejected with infinite loops
- Development helper scripts: `format.sh` and `cleanup.sh`

### Fixed
- Removed unintended `std` dependency to maintain full `no_std` compatibility
- Color updates now use epsilon-based f32 comparison to prevent spurious LED updates from floating-point rounding errors
- Flame flicker sequences in mode_switcher examples corrected

## [0.1.1] - 2025-11-20

### Changed
- Updated `heapless` dependency version

## [0.1.0] - 2025-11-08

Initial release of rgb-sequencer, a `no_std` embedded RGB LED animation library.

### Added
- Step-based and function-based RGB sequences with linear interpolation
- State machine control (load, start, pause, resume, restart, clear)
- Builder pattern API with loop support and timing compensation
- Trait-based hardware abstraction (`RgbLed`, `TimeSource`)
- Optional `defmt` logging support
- Example projects for STM32F0 and RP Pico

[Unreleased]: https://github.com/HybridChild/rgb-sequencer/compare/v0.2.1...HEAD [0.2.1]: https://github.com/HybridChild/rgb-sequencer/compare/v0.2.0...v0.2.1 [0.2.0]: https://github.com/HybridChild/rgb-sequencer/compare/v0.1.1...v0.2.0 [0.1.1]: https://github.com/HybridChild/rgb-sequencer/compare/v0.1.0...v0.1.1 [0.1.0]: https://github.com/HybridChild/rgb-sequencer/releases/tag/v0.1.0
