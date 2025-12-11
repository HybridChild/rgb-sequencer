# STM32 NUCLEO-F072RB Embassy Examples

Embassy async examples for STM32F NUCLEO-F072RB board.

- **[mode_switcher](#mode_switcher)** - Example demonstrating single-LED control with mode switching using async tasks and channels. Features a **function-based breathing sequence** using sine wave animation.
- **[rainbow_capture](#rainbow_capture)** - Example demonstrating smooth rainbow transitions with interactive color capture using async tasks, channels, and signals. Shows **individual LED control** with the enum wrapper pattern for managing heterogeneous LED types.
- **[transition_styles](#transition_styles)** - Example demonstrating all 5 TransitionStyle variants (Step, Linear, EaseIn, EaseOut, EaseInOut) with visual mode indication using the onboard LED.
- **[brightness_control](#brightness_control)** - Example demonstrating **global brightness control** with runtime adjustment. Button press cycles through brightness levels while maintaining the same rainbow sequence.

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

All examples use the onboard user button on PC13 (blue button on Nucleo board).

### Viewing logs
All examples use `defmt` for logging. Logs appear automatically when running with `probe-rs`.

## Common Anode vs Common Cathode

The examples assume a **common anode** RGB LED (common pin connected to 3.3V).

If you have a **common cathode** LED (common pin connected to GND), change the last parameter in `EmbassyPwmRgbLed::new()` to `false`:
```rust
let led = EmbassyPwmRgbLed::new(pwm, max_duty, false);
```

## Examples

### mode_switcher

Single-LED controller demonstrating Embassy async architecture with mode switching and function-based sequences.

**Features:**
- Four modes: Rainbow, Police, Flame and Breathing
- Function-based sequences for breathing and flame (algorithmic animation)
- Three async tasks: button, app_logic, rgb
- Inter-task communication via channels and signals
- Mode indicator via onboard LED
- Embassy time driver for precise async timing

**Behavior:**
1. On startup, LED begins rainbow animation (synchronized)
2. Press button → switches to police mode (red/blue alternating)
3. Press again → switches to flame mode (flickering orange/yellow fire effect)
4. Press again → switches to breathing mode (gentle white fade using sine wave)
5. Press again → back to rainbow mode (cycle repeats)
6. Onboard LED indicates mode: low when breathing, high when rainbow/police/flame

**Run:**
```bash
cargo run --release --bin mode_switcher
```

### rainbow_capture

Smooth rainbow animation with interactive color capture using two independent RGB LEDs, async tasks, and enum wrapper pattern for heterogeneous LED management.

**Features:**
- LED 1: Continuous red → green → blue rainbow cycle
- LED 2: Captures and displays current color from LED 1 on button press with smooth 2s transition
- Enum wrapper collection (`AnyLed`) for managing TIM1 and TIM3 LEDs together
- Pause/resume control with timing compensation
- Query-response pattern using channels and signals
- Embassy async tasks for coordinated control

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

### transition_styles

Demonstrates all five TransitionStyle variants using a single RGB LED with visual mode indication.

**Features:**
- All 5 TransitionStyle variants: Step, Linear, EaseIn, EaseOut, EaseInOut
- Same color sequence (red, green, blue, white, yellow, cyan, magenta) with different transitions
- Onboard LED blink pattern indicates current mode (solid/1/2/3/4 blinks)
- Button-controlled mode cycling
- Four async tasks: button, blink pattern, app logic, RGB control
- Embassy time driver for precise async timing

**Behavior:**
1. On startup, LED begins with Step transitions (instant color changes), onboard LED solid ON
2. Press button → switches to Linear transitions (constant-speed), onboard LED blinks once per cycle
3. Press again → switches to EaseIn transitions (slow start), onboard LED blinks twice per cycle
4. Press again → switches to EaseOut transitions (slow end), onboard LED blinks three times per cycle
5. Press again → switches to EaseInOut transitions (slow both ends), onboard LED blinks four times per cycle
6. Press again → back to Step mode (cycle repeats)

**Run:**
```bash
cargo run --release --bin transition_styles
```

### brightness_control

Demonstrates global brightness control with a continuously running rainbow sequence.

**Features:**
- Continuous rainbow color cycle with runtime brightness adjustment
- 5 brightness levels: Full (100%), High (75%), Medium (50%), Low (25%), Dim (10%)
- Button cycles through brightness levels
- Brightness changes immediately without rebuilding sequence
- Three async tasks: button, app logic, RGB control
- Onboard LED pattern indicates current brightness level
- Embassy time driver for precise async timing

**Behavior:**
1. On startup, LED begins rainbow animation at full brightness (100%)
2. Press button → brightness reduces to 75% (High), onboard LED turns off
3. Press again → brightness reduces to 50% (Medium), onboard LED turns on
4. Press again → brightness reduces to 25% (Low), onboard LED turns off
5. Press again → brightness reduces to 10% (Dim), onboard LED turns on
6. Press again → brightness returns to 100% (Full), onboard LED turns on (cycle repeats)
7. The rainbow sequence continues running throughout - only brightness changes

**Run:**
```bash
cargo run --release --bin brightness_control
```
