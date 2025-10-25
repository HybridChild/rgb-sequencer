# STM32 NUCLEO-F072RB Embassy Examples

Embassy async examples for STM32F NUCLEO-F072RB board.

Both examples demonstrate advanced embedded Rust patterns including **enum wrapper collections** for managing heterogeneous LED types (TIM1 and TIM3) in a single collection without heap allocation, enabling efficient multi-LED control with zero-cost abstraction.

- **rainbow_capture** - Embassy async example demonstrating smooth rainbow transitions with interactive color capture using async tasks, channels, and signals. Shows **individual LED control** with the enum wrapper pattern.
- **mode_switcher** - Embassy async example demonstrating coordinated multi-LED control with mode switching using async tasks and channels. Features a **function-based breathing sequence** using sine wave animation and **synchronized LED control** with the enum wrapper pattern.

## Hardware Setup

### RGB LED Connections

These examples use **two RGB LEDs**. Connect them to the following pins with appropriate current-limiting resistors (220Ω - 330Ω):

**LED 1:**
- **Red**: PA6 (TIM3_CH1)
- **Green**: PA7 (TIM3_CH2)
- **Blue**: PB0 (TIM3_CH3)
- **Common**: 3.3V (for common anode) or GND (for common cathode)

**LED 2:**
- **Red**: PA8 (TIM1_CH1)
- **Green**: PA9 (TIM1_CH2)
- **Blue**: PA10 (TIM1_CH3)
- **Common**: 3.3V (for common anode) or GND (for common cathode)

### User Button

The examples use the onboard user button on PC13 (blue button on Nucleo board).

## Building and Flashing

### Prerequisites

- Rust toolchain with `thumbv6m-none-eabi` target
- probe-rs for flashing

### Install Target
```bash
rustup target add thumbv6m-none-eabi
```

### Build
```bash
cargo build --release --bin <example_name>
```

### Flash and Run

With probe-rs:
```bash
cargo run --release --bin <example_name>
```

Or manually:
```bash
probe-rs run --chip STM32F072RBTx target/thumbv6m-none-eabi/release/<example_name>
```

### Viewing logs
Both examples use `defmt` for logging. Logs appear automatically when running with `probe-rs`.

## Common Anode vs Common Cathode

The examples assume a **common anode** RGB LED (common pin connected to 3.3V).

If you have a **common cathode** LED (common pin connected to GND), change the last parameter in `PwmRgbLed::new()` to `false`:
```rust
let led = PwmRgbLed::new(pwm, Channel::Ch1, Channel::Ch2, Channel::Ch3, false);
```

## Examples

### rainbow_capture

A smooth rainbow animation with interactive color capture control using two independent RGB LEDs and async tasks.

**Features:**
- **LED 1**: Continuously cycles through red → green → blue with smooth linear color transitions
- **LED 2**: Starts off, captures and displays the current color from LED 1 when button is pressed
- **Smooth color transitions**: LED 2 smoothly transitions to captured colors over 2 seconds
- **Pause/resume control**: Button toggles LED 1 between running and paused states
- **Bidirectional communication**: Query-response pattern using channels and signals
- **Enum wrapper collection**: Manages TIM1 and TIM3 LEDs in a single collection for individual control
- Uses Embassy's async tasks for coordinated control
- Efficient power management with async timers

**What you'll learn:**
- **Enum wrapper pattern**: How to store heterogeneous LED types (different timers) in a single collection
- **Zero-cost abstraction**: Type-safe multi-LED management without heap allocation or runtime overhead
- Independent sequencer control for multi-LED systems
- Color capture and dynamic sequence creation
- Query-response patterns with Embassy signals
- Pause/resume functionality with timing compensation
- Smooth color transitions using `start_color` feature

**Technical Highlights:**
The enum wrapper pattern (`AnyLed`) allows both TIM1 and TIM3 based LEDs to be stored in the same `Vec`:
```rust
pub enum AnyLed<'d> {
    Tim1(EmbassyPwmRgbLed<'d, TIM1>),
    Tim3(EmbassyPwmRgbLed<'d, TIM3>),
}
```
This enables individual LED control through `get_mut(led_id)` while maintaining type safety and zero-cost abstraction. The collection pattern eliminates code duplication when servicing multiple sequencers.

**Behavior:**
1. On startup, LED 1 begins rainbow animation, LED 2 is off
2. Press button → LED 1 pauses at current color, LED 2 smoothly transitions to that color (over 2 seconds)
3. Press button again → LED 1 resumes animation, LED 2 continues holding the captured color
4. Repeat to capture different colors from the rainbow cycle
5. Onboard LED indicates state: high when LED 1 running, low when paused

**Run:**
```bash
cargo run --release --bin rainbow_capture
```

### mode_switcher

A coordinated multi-LED controller demonstrating Embassy's async task architecture with mode switching. **Features function-based sequences** using sine wave mathematics for the breathing effect.

**Features:**
- **Four display modes**: Rainbow, Breathing (sine wave), Alternating, and Off
- **Function-based breathing sequence**: Uses algorithmic sine wave animation instead of step-based interpolation
- **Task-based architecture**: Three async tasks (button, app_logic, rgb)
- **Inter-task communication**: Channels and signals for coordinated control
- **Enum wrapper collection**: Manages TIM1 and TIM3 LEDs in a single collection for synchronized control
- Uses Embassy's time driver for precise async timing
- Demonstrates both function-based and step-based sequencing approaches

**What you'll learn:**
- **Enum wrapper pattern**: How to operate on all LEDs simultaneously using the same collection
- **Coordinated control**: Loading the same sequence on multiple heterogeneous LEDs with `load_all()`
- **Function-based sequences**: How to create algorithmic animations using custom functions
- **Sine wave mathematics**: Applying trigonometric functions for smooth breathing effects
- Embassy async task patterns and communication
- Multi-LED coordination with heterogeneous collection
- Dynamic sequence loading and mode switching
- Efficient sequencer servicing with optimal timing hints

**Technical Highlights:**
The breathing mode demonstrates the library's function-based sequence feature, where a sine wave function computes LED brightness algorithmically based on elapsed time. This approach:
- Allows the same function to be reused with different colors
- Provides smooth, natural-looking animations through mathematical curves
- Uses `libm` for `no_std` sine calculations
- Integrates seamlessly with Embassy's async runtime for continuous frame-by-frame updates

The enum wrapper collection enables synchronized control:
```rust
// Load same sequence on all LEDs regardless of timer type
collection.load_all(sequence);
```
This demonstrates how the pattern supports both individual control (rainbow_capture) and coordinated control (mode_switcher) with the same underlying abstraction.

**Behavior:**
1. On startup, both LEDs begin rainbow animation (synchronized)
2. Press button → switches to breathing mode (gentle white fade using sine wave)
3. Press again → alternating mode (red/blue swap between LEDs)
4. Press again → off mode (both LEDs turn off)
5. Press again → back to rainbow mode

**Run:**
```bash
cargo run --release --bin mode_switcher
```
