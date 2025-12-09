# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **BREAKING**: `SequenceBuilder::step()` now returns `Result<Self, SequenceError>` instead of panicking when capacity is exceeded
- License changed from MIT to dual MIT/Apache-2.0
- README updates for clarity and structure
- Removed Memory calculator tool in favor of size-analysis script
- `.gitignore` updated to track `.cargo/config.toml` for examples
- Examples updated to use new convenience method and type aliases
- CI workflow and ci-local script extended to check formatting for examples and size-analysis/minimal

### Added
- Introduce CHANGELOG.md (this file)
- `CLAUDE.md` file for AI assistant guidance
- Comprehensive CI workflow for automated testing
- Size analysis script for tracking binary size impact
- Compiler directives: `#![forbid(unsafe_code)]` and `#![warn(missing_docs)]`
- Comprehensive documentation for all public API items
- Common color constants: `COLOR_RED`, `COLOR_GREEN`, `COLOR_BLUE`, `COLOR_WHITE`, `COLOR_YELLOW`, `COLOR_CYAN`, `COLOR_MAGENTA`, `COLOR_ORANGE`, `COLOR_PURPLE`
- `load_and_start()` convenience method for `RgbSequencer`
- Type aliases for common capacities: `RgbSequencer4/8/16` and `RgbSequence4/8/16`
- `SequenceError::CapacityExceeded` variant for robust error handling

### Fixed
- Removed unintended `std` dependency to maintain full `no_std` compatibility

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
