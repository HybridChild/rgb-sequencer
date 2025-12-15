# rgb-sequencer

A `no_std`-compatible Rust library for controlling RGB LEDs in embedded systems through timed color sequences.

[![Platform](https://img.shields.io/badge/platform-no__std-blue)](https://github.com/HybridChild/rgb-sequencer)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green)](https://github.com/HybridChild/rgb-sequencer)

---

## Overview

**rgb-sequencer** provides a lightweight framework for creating and executing RGB LED animations on resource-constrained embedded devices. Define high-level sequences and let the library handle timing, interpolation, and LED updates.

**Key features:**
- **Platform independent** - Hardware is abstracted through traits for LEDs and time systems
- **Smooth color transitions** - Linear interpolation and quadratic easing
- **Brightness control** - Global brightness adjustment without recreating sequences
- **Pause/resume** - With timing compensation for perfect continuity
- **Multi-LED support** - Independent sequencers, command-based control
- **Drift-free timing** - Time-based color calculation prevents drift and enables true synchronization
- **Efficient timing** - Service hints enable power-efficient operation

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
    LoopCount, WHITE, BLACK
};
use palette::Srgb;

// 1. Implement the RgbLed trait for your hardware
struct MyLed {
    // Your GPIO pins, PWM channels, etc.
}

impl RgbLed for MyLed {
    fn set_color(&mut self, color: Srgb) {
        // Convert Srgb color to your hardware format
        // e.g., PWM duty cycles, 8-bit RGB values
    }
}

// 2. Implement the TimeSource trait for your timing system
struct MyTimer;
impl TimeSource<MyInstant> for MyTimer {
    fn now(&self) -> MyInstant {
        // Return current time
    }
}

// 3. Create a blinking sequence
let sequence = RgbSequence8::builder()
    .step(WHITE, Duration::from_millis(500), TransitionStyle::Step).unwrap()  // White
    .step(BLACK, Duration::from_millis(500), TransitionStyle::Step).unwrap()  // Off
    .loop_count(LoopCount::Infinite)                                          // Loop indefinitely
    .build()
    .unwrap();

// 4. Create sequencer and start
let led = MyLed::new();
let timer = MyTimer::new();
let mut sequencer = RgbSequencer8::new(led, &timer);

sequencer.load_and_start(sequence).unwrap();

// 5. Service in your main loop and use timing hint for optimal sleep duration
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

## Documentation

- **[FEATURES.md](docs/FEATURES.md)** - Complete feature guide with examples

## Memory Impact

**Planning tool**: Use the [sizeof-calculator](tools/sizeof-calculator/README.md) to estimate RAM costs for different sequence capacities and duration types. Runs instantly on your host machine.

**Binary analysis**: Use the [binary-analyzer](tools/binary-analyzer/README.md) to measure Flash/RAM overhead on embedded ARM targets with symbol-level breakdowns.

## Performance Considerations

### Benchmark Results

Performance measured on embedded targets:

**RP2040 (Cortex-M0+, 125 MHz, no FPU)**
- Step transitions: ~50 µs per `service()` call
- Linear/Easing: ~75 µs per `service()` call

**RP2350 (Cortex-M33F, 150 MHz, with FPU)**
- Step transitions: ~19 µs per `service()` call
- Linear/Easing: ~22 µs per `service()` call

See [benchmark results](tools/benchmark/) for detailed cycle counts and test configurations.

### Floating Point Math Requirements

This library uses `f32` for color math and interpolation, so performance will vary by target as the benchmarks demonstrate:

#### Hardware FPU (Fast) ✅
Cortex-M4F, M7, M33F - Hardware-accelerated `f32` operations. Minimal overhead for easing functions.

#### No Hardware FPU (Slower) ⚠️
Cortex-M0/M0+, M3 - Software-emulated `f32` operations. Linear/easing adds ~50% overhead vs Step transitions.

**Recommendation:** For low-power scenarios on non-FPU targets, prefer Step transitions exclusively.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
