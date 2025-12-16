# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- Documentation streamlined and consolidated (removed IMPLEMENTATION.md in favor of inline code comments)

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
- Compiler directives: `#![forbid(unsafe_code)]` and `#![warn(missing_docs)]`
- Comprehensive documentation for all public API items
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

[Unreleased]: https://github.com/HybridChild/rgb-sequencer/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/HybridChild/rgb-sequencer/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/HybridChild/rgb-sequencer/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/HybridChild/rgb-sequencer/releases/tag/v0.1.0
