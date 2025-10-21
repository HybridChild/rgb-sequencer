# STM32 NUCLEO-F072RB Examples

Examples for STM32F NUCLEO-F072RB board.

- **rainbow_button** - Simple bare-metal example demonstrating smooth rainbow transitions with button-controlled pause/resume using SysTick timing.

## Hardware Setup

### RGB LED Connections

Connect an RGB LED to the following pins with appropriate current-limiting resistors (220Ω - 330Ω):

- **Red**: PA6 (TIM3_CH1)
- **Green**: PA7 (TIM3_CH2)
- **Blue**: PB0 (TIM3_CH3)
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

A smooth rainbow animation with interactive pause/resume control.

**Features:**
- Cycles through red → green → blue continuously with linear color transitions
- Press the user button to pause/resume the animation
- Uses SysTick timer for precise 1ms timing
- Efficient power management with WFI (Wait For Interrupt)

**What you'll learn:**
- Basic sequencer usage with linear transitions
- Hardware timer integration (SysTick)
- Interactive control with button input
- Pause/resume functionality

**Run:**
```bash
cargo run --release --bin rainbow_button
```
