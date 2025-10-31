# STM32 NUCLEO-F072RB Examples

Examples for STM32F NUCLEO-F072RB board.

- **blinky** - Simple bare-metal example demonstrating basic RGB LED sequencing with a clean, blocking delay approach. Perfect starting point for learning the library.
- **mode_switcher** - Bare-metal example demonstrating RGB LED control with mode switching using SysTick timing. Features a **function-based breathing sequence** using sine wave animation.
- **rainbow_capture** - Bare-metal example demonstrating smooth rainbow transitions with interactive color capture using SysTick timing and two RGB LEDs.

## Hardware Setup

### RGB LED Connections

These examples use **one or two external RGB LEDs** depending on the example. Connect them to the following pins with appropriate current-limiting resistors:

**LED 1 (used by all examples):**
- **Red**: PA6 (TIM3_CH1)
- **Green**: PA7 (TIM3_CH2)
- **Blue**: PB0 (TIM3_CH3)
- **Common**: 3.3V (for common anode) or GND (for common cathode)

**LED 2 (only used by rainbow_capture):**
- **Red**: PA8 (TIM1_CH1)
- **Green**: PA9 (TIM1_CH2)
- **Blue**: PA10 (TIM1_CH3)
- **Common**: 3.3V (for common anode) or GND (for common cathode)

### User Button

The rainbow_capture and mode_switcher examples use the onboard user button on PC13 (blue button on Nucleo board).

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

### blinky

A simple, clean example demonstrating basic LED sequencing with blocking delays. This is the perfect starting point for learning the library.

**Features:**
- **Single RGB LED**: Controls one RGB LED through a colorful sequence
- **Blocking approach**: Uses HAL's `Delay` for simple, easy-to-understand timing
- **Simple time source**: Advances time manually after each delay
- **Finite sequence**: Runs 3 loops then displays a landing color
- **Zero-duration steps**: Demonstrates instant color changes before fade-outs
- No interrupt handlers or manual WFI calls needed

**What you'll learn:**
- Basic sequencer usage with minimal setup
- How to create sequences with steps and transitions
- The difference between Step (instant) and Linear (fade) transitions
- Zero-duration steps as color waypoints
- Finite loop counts with landing colors
- Simple blocking delay pattern

**Behavior:**
1. LED fades from Yellow to off over 1 second
2. LED fades from Cyan to off over 1 second
3. LED fades from Purple to off over 1 second
4. Sequence repeats 3 times total
5. After completion, LED shows white (landing color)

**Run:**
```bash
cargo run --release --bin blinky
```

### mode_switcher

An RGB LED controller demonstrating mode switching with different animations. **Features function-based sequences** using sine wave mathematics for the breathing effect.

**Features:**
- **Three display modes**: Breathing (sine wave), Rainbow, and Police
- **Function-based breathing sequence**: Uses algorithmic sine wave animation instead of step-based interpolation
- **Mode indicator**: Onboard LED (PA5) indicates current mode
- Uses SysTick timer for precise 1ms timing
- Efficient power management with WFI (Wait For Interrupt)
- Demonstrates both function-based and step-based sequencing approaches

**What you'll learn:**
- **Function-based sequences**: How to create algorithmic animations using custom functions
- **Sine wave mathematics**: Applying trigonometric functions for smooth breathing effects
- Dynamic sequence loading and mode switching
- Mode state management
- Efficient sequencer servicing with optimal timing hints

**Technical Highlights:**
The breathing mode demonstrates the library's function-based sequence feature, where a sine wave function computes LED brightness algorithmically based on elapsed time. This approach:
- Allows the same function to be reused with different colors
- Provides smooth, natural-looking animations through mathematical curves
- Uses `libm` for `no_std` sine calculations
- Returns to the sequencer continuously for frame-by-frame updates

**Behavior:**
1. On startup, the RGB LED begins rainbow animation
3. Press button → switches to police mode (red/blue alternating)
2. Press again → switches to breathing mode (gentle white fade using sine wave)
4. Press again → back to rainbow animation (cycle repeats)
5. Onboard LED indicates mode: off = breathing, on = rainbow/police

**Run:**
```bash
cargo run --release --bin mode_switcher
```

### rainbow_capture

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
cargo run --release --bin rainbow_capture
```
