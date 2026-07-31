# rgb-sequencer

A `no_std`-compatible Rust library for controlling RGB LEDs in embedded systems through timed color sequences.

[![Platform](https://img.shields.io/badge/platform-no__std-blue)](https://github.com/HybridChild/rgb-sequencer)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green)](https://github.com/HybridChild/rgb-sequencer)

---

## Overview

**rgb-sequencer** provides a lightweight framework for creating and executing RGB LED animations on resource-constrained embedded devices. Define high-level sequences and let the library handle timing, interpolation, and LED updates.

**Key features:**
- **No floating point** - All color math is fixed-point integer arithmetic, so nothing pulls in soft-float emulation on Cortex-M0/M0+
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
rgb-sequencer = "0.3"
```

### Minimal Example
```rust
use rgb_sequencer::{
    Rgb, RgbSequencer8, RgbSequence8, RgbLed, TimeSource, TransitionStyle,
    LoopCount, WHITE, BLACK
};

// 1. Implement the RgbLed trait for your hardware
struct MyLed {
    // Your GPIO pins, PWM channels, etc.
}

impl RgbLed for MyLed {
    fn set_color(&mut self, color: Rgb) {
        // Channels are 0..=65535. Scale to your hardware's format:
        //   PWM:    (color.r as u32 * max_duty as u32 / 65535) as u16
        //   8-bit:  let (r, g, b) = color.to_u8();
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

## Colors

Colors are `Rgb`, three `u16` channels running `0..=65535`:

```rust
use rgb_sequencer::{Rgb, RED};

let orange = Rgb::from_u8(255, 128, 0);  // const-friendly, 255 maps to 65535 exactly
let half    = RED.scale(128);            // 8-bit brightness factor, 255 = unchanged
let blend   = RED.lerp(orange, 16384);   // Q0.15 factor, 32768 = fully `orange`
let (r, g, b) = orange.to_u8();          // for WS2812 and other 8-bit drivers
```

Sixteen bits per channel is both a normalized value and a natural 16-bit PWM duty, so most hardware needs only a multiply and a shift. Interpolation happens at full 16-bit precision, so slow fades show no banding on 8-bit hardware.

## Performance

### No Floating Point

All color math — progress, easing, interpolation, and brightness — is fixed-point integer arithmetic. Nothing in the library links soft-float emulation, so Cortex-M0/M0+ pays no penalty for `Linear` or easing transitions, and every transition style is practical on every target.

### Flash Footprint

Measured with [binary-analyzer](tools/binary-analyzer/README.md) against a minimal reference binary (`opt-level = "z"`, LTO):

| Target | Flash |
|--------|-------|
| `thumbv6m-none-eabi` (Cortex-M0/M0+, no FPU) | 2156 B |
| `thumbv7em-none-eabihf` (Cortex-M4F/M7, FPU) | 1848 B |

Dropping `f32` cut the non-FPU build by 43% — the `__divsf3`/`__addsf3`/`__mulsf3` emulation routines it used to carry are gone entirely.

### Timing

Per-`service()` timings depend on your target and sequence capacity. Use the [benchmark tool](tools/benchmark/) to measure on your own hardware; it reports every transition style across capacities of 4 to 32 steps.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
