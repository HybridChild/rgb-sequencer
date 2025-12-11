# STM32 NUCLEO-F072RB Embassy Examples

Embassy async examples for STM32F NUCLEO-F072RB board.

- **[mode_switcher](#mode_switcher)** - Embassy async example demonstrating single-LED control with mode switching using async tasks and channels. Features a **function-based breathing sequence** using sine wave animation.
- **[rainbow_capture](#rainbow_capture)** - Embassy async example demonstrating smooth rainbow transitions with interactive color capture using async tasks, channels, and signals. Shows **individual LED control** with the enum wrapper pattern for managing heterogeneous LED types.
- **[transition_styles](#transition_styles)** - Embassy async example demonstrating all 5 TransitionStyle variants (Step, Linear, EaseIn, EaseOut, EaseInOut) with visual mode indication using the onboard LED.
- **[brightness_control](#brightness_control)** - Embassy async example demonstrating **global brightness control** with runtime adjustment. Button press cycles through brightness levels while maintaining the same rainbow sequence.

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
All examples use `defmt` for logging. Logs appear automatically when running with `probe-rs`.

## Common Anode vs Common Cathode

The examples assume a **common anode** RGB LED (common pin connected to 3.3V).

If you have a **common cathode** LED (common pin connected to GND), change the last parameter in `EmbassyPwmRgbLed::new()` to `false`:
```rust
let led = EmbassyPwmRgbLed::new(pwm, max_duty, false);
```

## Examples

### mode_switcher

A single-LED controller demonstrating Embassy's async task architecture with mode switching. **Features function-based sequences** using sine wave mathematics for the breathing and flame effects.

**Features:**
- **Single RGB LED**: Controls one RGB LED with four different animation modes: Rainbow, Police, Flame, and Breathing
- **Function-based sequences**: Uses algorithmic animation for breathing and flame effects instead of step-based interpolation
- **Task-based architecture**: Three async tasks (button, app_logic, rgb)
- **Inter-task communication**: Channels and signals for coordinated control
- **Mode indicator**: Onboard LED shows current mode state
- Uses Embassy's time driver for precise async timing
- Demonstrates both function-based and step-based sequencing approaches

**What you'll learn:**
- **Embassy async patterns**: How to structure multi-task applications with channels and signals
- **Function-based sequences**: How to create algorithmic animations using custom functions
- **Sine wave mathematics**: Applying trigonometric functions for smooth breathing and flickering flame effects
- **Multi-frequency animation**: Combining multiple sine waves to create complex, pseudo-random effects
- Dynamic sequence loading and mode switching
- Efficient sequencer servicing with optimal timing hints
- Simple single-LED control with Embassy tasks

**Technical Highlights:**
The breathing and flame modes demonstrate the library's function-based sequence feature, where mathematical functions compute LED color and brightness algorithmically based on elapsed time. This approach:
- **Breathing**: Uses a single sine wave for smooth, periodic brightness oscillation
- **Flame**: Combines multiple sine waves at different frequencies (fast, medium, slow) to create realistic flickering with color temperature variation
- Allows the same functions to be reused with different base colors
- Provides smooth, natural-looking animations through mathematical curves
- Uses `libm` for `no_std` sine calculations
- Integrates seamlessly with Embassy's async runtime for continuous frame-by-frame updates

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

A smooth rainbow animation with interactive color capture control using two independent RGB LEDs and async tasks. This example demonstrates advanced embedded Rust patterns including **enum wrapper collections** for managing varying LED types (TIM1 and TIM3) in a single heterogeneous collection without heap allocation.

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

### transition_styles

Demonstrates all five TransitionStyle variants using a single RGB LED with visual mode indication via the onboard LED. Cycles through Step (instant), Linear (constant-speed), EaseIn (slow start), EaseOut (slow end), and EaseInOut (slow both ends) transitions.

**Features:**
- **Single RGB LED**: Cycles through color sequence with different transition styles
- **All 5 TransitionStyle variants**: Step, Linear, EaseIn, EaseOut, EaseInOut
- **Visual mode indication**: Onboard LED blinks to show current transition mode
- **Button-controlled mode switching**: Press button to cycle through transition styles
- **Task-based architecture**: Separate tasks for button, blink pattern, app logic, and RGB control
- Uses Embassy's time driver for precise async timing
- Demonstrates easing functions for smooth, natural-looking animations

**What you'll learn:**
- **Easing functions**: How quadratic easing creates natural acceleration/deceleration
- **Transition comparison**: Visual comparison of all transition styles side-by-side
- Dynamic sequence reloading with different transition parameters
- Multi-task coordination using channels and signals
- Visual feedback patterns for mode indication

**Technical Highlights:**
The example cycles through the same color sequence (red, green, blue, white, yellow, cyan, magenta with black transitions) using all five transition styles. The onboard LED provides visual feedback:
- **Solid ON**: Step mode (instant color changes)
- **1 blink**: Linear mode (constant-speed interpolation)
- **2 blinks**: EaseIn mode (slow start, accelerating)
- **3 blinks**: EaseOut mode (fast start, decelerating)
- **4 blinks**: EaseInOut mode (slow start and end)

This allows direct comparison of how each transition style affects the animation feel and demonstrates the practical differences between linear and eased interpolation.

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

Demonstrates global brightness control using a single RGB LED with a continuously running rainbow sequence. The brightness level is adjusted dynamically in response to button presses, showing how the same sequence appears at different brightness levels.

**Features:**
- **Single RGB LED**: Runs a continuous rainbow color cycle
- **Global brightness control**: Adjusts overall LED brightness without modifying the sequence
- **5 brightness levels**: Full (100%), High (75%), Medium (50%), Low (25%), Dim (10%)
- **Runtime adjustment**: Brightness changes immediately during playback
- **Task-based architecture**: Separate tasks for button handling, app logic, and RGB control
- **Mode indicator**: Onboard LED pattern indicates current brightness level
- Uses Embassy's time driver for precise async timing

**What you'll learn:**
- **Global brightness control**: How to use `set_brightness()` to dim LEDs without rebuilding sequences
- **Runtime brightness adjustment**: Changing brightness during sequence playback
- **Power management**: Using brightness control for battery saving and night mode
- **Brightness vs sequence separation**: Understanding that brightness affects output but not sequence timing
- Channel-based command routing for brightness commands

**Technical Highlights:**
The example demonstrates the library's global brightness feature, which multiplies all color values by a brightness factor (0.0-1.0) before sending them to the LED. This allows:
- **Same sequence, different appearance**: The rainbow sequence runs unchanged while appearance varies
- **Efficient dimming**: No need to recreate sequences or modify color values
- **Instant updates**: Brightness changes apply on the next `service()` call
- **Practical applications**: Night mode (low brightness), battery saving (reduced power), ambient light adaptation

The brightness control is independent of sequence logic - timing, transitions, and color progression remain unchanged while the overall output intensity varies.

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
