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

## Quick Start

### Add Dependency
```toml
[dependencies]
rgb-sequencer = "0.1"
palette = { version = "0.7.6", default-features = false, features = ["libm"] }
```

### Minimal Example
```rust
use rgb_sequencer::{RgbSequencer, RgbSequence, RgbLed, TimeSource, TransitionStyle, LoopCount};
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

// 3. Create a blinking sequence
let sequence = RgbSequence::new()
    .step(Srgb::new(1.0, 0.0, 0.0), Duration::from_millis(500), TransitionStyle::Step)  // Red
    .step(Srgb::new(0.0, 0.0, 0.0), Duration::from_millis(500), TransitionStyle::Step)  // Off
    .loop_count(LoopCount::Finite(3))                                                   // Loop 3 times
    .landing_color(Srgb::new(1.0, 1.0, 1.0))                                            // Land on white
    .build()?;

// 4. Create sequencer and start
let led = MyLed::new();
let timer = MyTimer::new();
let mut sequencer = RgbSequencer::<_, _, _, 8>::new(led, &timer);

sequencer.load(sequence);
sequencer.start().unwrap();

// 5. Service in your main loop
loop {
    if let Some(delay) = sequencer.service().unwrap() {
        if delay == TimeDuration::ZERO {
            // Linear transition - sleep for desired frame rate
            sleep_ms(FRAME_RATE_MS);
        } else {
            // Step transition - delay for the specified time
            sleep_ms(delay.as_millis());
        }
    } else {
        // Sequence complete
        break;
    }
}
```

### Next Steps

- **See complete examples**: Check the [`examples/`](examples/) directory for working STM32F0 implementations (bare-metal and Embassy)
- **Learn advanced features**: Function-based sequences, multi-LED control, pause/resume
- **Read the docs**: `cargo doc --open` for detailed API documentation

## License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/HybridChild/rgb-sequencer/blob/main/LICENSE) file for details.
