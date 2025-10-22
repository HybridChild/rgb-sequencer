# STM32 NUCLEO-F072RB Examples

Examples for STM32F NUCLEO-F072RB board.

- **rainbow_button** - Bare-metal example demonstrating smooth rainbow transitions with interactive color capture using SysTick timing and two RGB LEDs.

## Hardware Setup

### RGB LED Connections

These example uses **two RGB LEDs**. Connect them to the following pins with appropriate current-limiting resistors (220Ω - 330Ω):

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
- probe-rs or OpenOCD for flashing

### Install Target
```bash
rustup target add thumbv6m-none-eabi
```

### Build
```bash
cargo build --release --bin <example_name>
```

### Flash

With probe-rs (recommended):
```bash
cargo run --release --bin <example_name>
```

Or manually:
```bash
probe-rs run --chip STM32F072RBTx target/thumbv6m-none-eabi/release/<example_name>
```

## Common Anode vs Common Cathode

The examples assume a **common anode** RGB LED (common pin connected to 3.3V).

If you have a **common cathode** LED (common pin connected to GND), change the last parameter in `PwmRgbLed::new()` to `false`:
```rust
let led = PwmRgbLed::new(red_pwm, green_pwm, blue_pwm, false);
```

## Examples

### rainbow_button

A smooth rainbow animation with interactive color capture control using two independent RGB LEDs.

**Features:**
- **LED 1**: Continuously cycles through red → green → blue with smooth linear color transitions
- **LED 2**: Starts off, captures and displays the current color from LED 1 when button is pressed
- Uses SysTick timer for precise 1ms timing
- Efficient power management with WFI (Wait For Interrupt)
- Demonstrates independent sequencer control for multi-LED systems

**What you'll learn:**
- Multi-LED sequencer usage with different animations
- Color capture and dynamic sequence creation
- Hardware timer integration (SysTick)
- Pause/resume functionality

**Behavior:**
1. On startup, LED 1 begins its rainbow animation, LED 2 is off
2. Press button → LED 1 pauses at current color, LED 2 lights up with that same color
3. Press button again → LED 1 resumes animation, LED 2 continues holding the captured color
4. Repeat to capture different colors from the rainbow cycle

**Run:**
```bash
cargo run --release --bin rainbow_button
```
