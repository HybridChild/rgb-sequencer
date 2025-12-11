# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **BREAKING**: Renamed `RgbSequencer::get_state()` to `state()` to follow Rust API naming conventions
- **BREAKING**: `SequenceBuilder::step()` now returns `Result<Self, SequenceError>` instead of panicking when capacity is exceeded
- License changed from MIT to dual MIT/Apache-2.0
- README updates for clarity and structure
- Removed Memory calculator tool in favor of size-analysis script
- `.gitignore` updated to track `.cargo/config.toml` for examples
- Examples updated to use new convenience methods and type aliases

### Added
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
- Compiler directives: `#![forbid(unsafe_code)]` and `#![warn(missing_docs)]`
- Comprehensive documentation for all public API items
- Common color constants: `COLOR_RED`, `COLOR_GREEN`, `COLOR_BLUE`, `COLOR_WHITE`, `COLOR_YELLOW`, `COLOR_CYAN`, `COLOR_MAGENTA`
- `load_and_start()` convenience method for `RgbSequencer`
- Type aliases for common capacities: `RgbSequencer4/8/16` and `RgbSequence4/8/16`
- `SequenceError::CapacityExceeded` variant for robust error handling
- Easing functions: `TransitionStyle::EaseIn`, `EaseOut`, and `EaseInOut` for smoother, more natural transitions using quadratic interpolation
- Example project demonstrating new transition styles
- `RgbSequencer::current_position()` method for event detection - enables tracking step and loop changes without callbacks
- `RgbSequence::find_step_position()` is now public for advanced use cases

### Fixed
- Removed unintended `std` dependency to maintain full `no_std` compatibility
- Color updates now use epsilon-based f32 comparison to prevent spurious LED updates from floating-point rounding errors

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

[Unreleased]: https://github.com/HybridChild/rgb-sequencer/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/HybridChild/rgb-sequencer/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/HybridChild/rgb-sequencer/releases/tag/v0.1.0
