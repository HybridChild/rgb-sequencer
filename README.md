# rgb-sequencer

A `no_std`-compatible Rust library for controlling RGB LEDs through timed color sequences on embedded systems.

## Overview

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

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.