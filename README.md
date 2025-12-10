# rgb-sequencer

A `no_std`-compatible Rust library for controlling RGB LEDs through timed color sequences on embedded systems.

[![Platform](https://img.shields.io/badge/platform-no__std-blue)](https://github.com/HybridChild/rgb-sequencer)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green)](https://github.com/HybridChild/rgb-sequencer)

---

## Overview

**rgb-sequencer** provides a lightweight framework for creating and executing RGB LED animations on resource-constrained embedded devices. Define high-level sequences and let the library handle timing, interpolation, and LED updates.

**Key features:**
- **Zero heap allocation** - Fixed-capacity collections, compile-time sizing
- **Platform independent** - Trait abstractions for LEDs and timing
- **Two animation approaches** - Step-based (waypoints) and function-based (algorithmic)
- **Efficient timing** - Service hints enable power-efficient operation
- **State machine** - Explicit states prevent errors (Idle, Loaded, Running, Paused, Complete)
- **Pause/resume** - With timing compensation for perfect continuity
- **Multi-LED support** - Independent sequencers, command-based control

The library supports two animation approaches:

1. **Step-based sequences**: Define explicit color waypoints with durations and transition styles (instant, linear, or eased interpolation). Supports quadratic easing functions (ease-in, ease-out, ease-in-out) for more natural-looking transitions. Perfect for discrete animations like police lights, status indicators, or scripted color shows. Support finite or infinite looping with configurable landing colors, and smooth entry animations via start colors.

2. **Function-based sequences**: Use custom functions to compute colors algorithmically based on elapsed time. Ideal for mathematical animations like sine wave breathing effects, HSV color wheels, or any procedurally generated pattern.

Each `RgbSequencer` instance controls one LED independently through trait abstractions, allowing you to:
- Run different animations on multiple LEDs simultaneously
- Pause and resume individual sequences
- Query the current color of individual LEDs

The library is built for embedded systems with:
- **Zero heap allocation**: All storage uses fixed-capacity collections with compile-time sizing
- **Platform independence**: Abstracts LED control and timing system through traits
- **Efficient timing**: Service timing hints enable power-efficient operation without unnecessary polling
- **Type-safe colors**: Uses `palette::Srgb<f32>` for accurate color math and smooth transitions

Whether you're building a status LED that breathes gently, a multi-LED notification system with synchronized animations, or an interactive light show that responds to user input, rgb-sequencer provides the building blocks and lets you focus on your application logic.

## Documentation

- **[FEATURES.md](docs/FEATURES.md)** - Complete feature guide with examples
- **[IMPLEMENTATION.md](docs/IMPLEMENTATION.md)** - Technical implementation details for contributors

## Quick Start

### Add Dependency
```toml
[dependencies]
rgb-sequencer = "0.1"
palette = { version = "0.7.6", default-features = false, features = ["libm"] }
```

### Minimal Example
```rust
use rgb_sequencer::{
    RgbSequencer8, RgbSequence8, RgbLed, TimeSource, TransitionStyle,
    LoopCount, COLOR_WHITE, COLOR_OFF
};
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

// 3. Create a blinking sequence (capacity of 8 steps)
let sequence = RgbSequence8::<MyDuration>::builder()
    .step(COLOR_WHITE, Duration::from_millis(500), TransitionStyle::Step).unwrap()  // White
    .step(COLOR_OFF, Duration::from_millis(500), TransitionStyle::Step).unwrap()    // Off
    .loop_count(LoopCount::Infinite)                                                // Loop indefinitely
    .build()
    .unwrap();

// 4. Create sequencer and start (load_and_start convenience method)
let led = MyLed::new();
let timer = MyTimer::new();
let mut sequencer = RgbSequencer8::new(led, &timer);

sequencer.load_and_start(sequence).unwrap();

// 5. Service in your main loop
loop {
    match sequencer.service().unwrap() {
        ServiceTiming::Continuous => {
            // Linear transition - sleep for desired frame rate
            sleep_ms(16);  // ~60 FPS
        }
        ServiceTiming::Delay(duration) => {
            // Step transition - sleep for exact duration
            sleep_ms(duration.as_millis());
        }
        ServiceTiming::Complete => {
            // Sequence finished
            break;
        }
    }
}
```

## Memory Impact

The library has minimal Flash overhead. Use the [size-analysis tool](size-analysis/README.md) to generate detailed reports showing baseline library cost across different ARM Cortex-M targets, with symbol-level analysis of what contributes to binary size.

## Performance Considerations

### Floating Point Math Requirements

**IMPORTANT**: This library uses `f32` extensively for color math and interpolation. Performance varies by target:

#### Hardware FPU (Fast) ✅
Cortex-M4F, M7, M33 (e.g., STM32F4, STM32H7, nRF52) - Hardware-accelerated f32 operations, excellent performance.

#### No Hardware FPU (Slow) ⚠️
Cortex-M0/M0+, M3 (e.g., STM32F0, STM32F1, RP2040) - Software-emulated f32 is **10-100x slower**.

**Recommendations for non-FPU targets:**
- Prefer Step transitions (no interpolation math)
- Linear is acceptable for simple transitions
- Avoid easing functions (EaseIn/EaseOut/EaseInOut - additional f32 operations)
- Avoid math-heavy function-based sequences

The library works on all targets but is optimized for microcontrollers with FPU.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
