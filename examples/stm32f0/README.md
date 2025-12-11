# STM32 NUCLEO-F072RB Examples

Examples for STM32F NUCLEO-F072RB board.

- **[blinky](#blinky)** - Simple bare-metal example demonstrating basic RGB LED sequencing with a clean, blocking delay approach. Perfect starting point for learning the library.
- **[mode_switcher](#mode_switcher)** - Bare-metal example demonstrating RGB LED control with mode switching using SysTick timing. Features a **function-based breathing sequence** using sine wave animation.
- **[rainbow_capture](#rainbow_capture)** - Bare-metal example demonstrating smooth rainbow transitions with interactive color capture using SysTick timing and two RGB LEDs.

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

## Common Anode vs Common Cathode

The examples assume a **common anode** RGB LED (common pin connected to 3.3V).

If you have a **common cathode** LED (common pin connected to GND), change the last parameter in `PwmRgbLed::new()` to `false`:
```rust
let led = PwmRgbLed::new(red_pwm, green_pwm, blue_pwm, false);
```

## Examples

### blinky

Simple LED sequencing with blocking delays. Perfect starting point for learning the library.

**Features:**
- Single RGB LED with colorful sequence (yellow, cyan, purple)
- Infinite loop with Step and Linear transitions
- Blocking approach using HAL's `Delay`
- Manual time source (counter-based, no interrupts)
- Zero-duration steps for instant color changes

**Behavior:**
1. LED fades from Yellow to off over 1 second
2. LED fades from Cyan to off over 1 second
3. LED fades from Purple to off over 1 second
4. Sequence repeats infinitely

**Run:**
```bash
cargo run --release --bin blinky
```

### mode_switcher

RGB LED controller with mode switching and function-based sequences.

**Features:**
- Four modes: Rainbow, Police, Flame, Breathing
- Function-based sequences for breathing and flame (algorithmic animation)
- Step-based sequences for rainbow and police
- Mode indicator via onboard LED
- SysTick timer with efficient WFI power management

**Behavior:**
1. On startup, the RGB LED begins rainbow animation
2. Press button → switches to police mode (red/blue alternating)
3. Press again → switches to flame mode (flickering orange/yellow fire effect)
4. Press again → switches to breathing mode (gentle white fade using sine wave)
5. Press again → back to rainbow animation (cycle repeats)
6. Onboard LED indicates mode: off = breathing, on = rainbow/police/flame

**Run:**
```bash
cargo run --release --bin mode_switcher
```

### rainbow_capture

Smooth rainbow animation with interactive color capture using two independent RGB LEDs.

**Features:**
- LED 1: Continuous red → green → blue rainbow cycle
- LED 2: Captures and displays current color from LED 1 on button press
- Independent sequencer control for multi-LED systems
- Pause/resume with timing compensation
- SysTick timer with efficient WFI power management

**Behavior:**
1. On startup, LED 1 begins its rainbow animation, LED 2 is off
2. Press button → LED 1 pauses at current color, LED 2 lights up with that same color
3. Press button again → LED 1 resumes animation, LED 2 continues holding the captured color
4. Repeat to capture different colors from the rainbow cycle

**Run:**
```bash
cargo run --release --bin rainbow_capture
```
