# Raspberry Pi Pico Examples

Examples for Raspberry Pi Pico board using PWM-based RGB LED control.

- **[blinky](#blinky)** - Simple bare-metal example demonstrating basic RGB LED sequencing with PWM control and a clean, blocking delay approach. Perfect starting point for learning the library on RP2040.
- **[breathing](#breathing)** - White breathing effect using function-based animation with sine wave modulation. Demonstrates algorithmic color generation and smooth brightness transitions.
- **[fade_check](#fade_check)** - Diagnostic rather than demo. Runs PWM at the full 16-bit range so slow fades can be inspected for banding, and reports per-frame step sizes over RTT.

## Hardware Setup

### RGB LED Connections

These examples use **one external RGB LED** controlled via PWM. Connect it to the following GPIO pins with appropriate current-limiting resistors:

**LED 1:**
- **Red**: GPIO2 (PWM1 Channel A)
- **Green**: GPIO3 (PWM1 Channel B)
- **Blue**: GPIO4 (PWM2 Channel A)
- **Common**: 3.3V (for common anode) or GND (for common cathode)

## Common Anode vs Common Cathode

The examples assume a **common anode** RGB LED (common pin connected to 3.3V).

If you have a **common cathode** LED (common pin connected to GND), change the last parameter in `PwmRgbLed::new()` to `false`:

```rust
let led = PwmRgbLed::new(red_channel, green_channel, blue_channel, false);
```

## Examples

### blinky

Simple LED sequencing with blocking delays. Perfect starting point for learning the library.

**Features:**
- Single RGB LED with colorful blink sequence
- Infinite loop with Step and Linear transitions
- Blocking approach using Cortex-M `Delay`
- Hardware timer (RP2040 Timer peripheral at 1 MHz)
- Zero-duration steps for instant color changes
- PWM configuration: 1 kHz with phase-correct mode

**Behavior:**
1. LED instantly changes to Yellow, then fades to off over 1 second
2. LED instantly changes to Cyan, then fades to off over 1 second
3. LED instantly changes to Purple, then fades to off over 1 second
4. Sequence repeats infinitely

**Run:**
```bash
cargo run --release --bin blinky
```

Or build UF2 and flash via bootloader:
```bash
cargo build --release --bin blinky
elf2uf2-rs target/thumbv6m-none-eabi/release/blinky blinky.uf2
# Copy blinky.uf2 to RPI-RP2 drive
```

### breathing

White breathing effect using function-based animation. Demonstrates algorithmic sequence generation.

**Features:**
- Function-based sequence using sine wave modulation
- Smooth brightness oscillation (10% to 100%)
- 4-second breathing cycle (2s fade up, 2s fade down)
- Hardware timer (RP2040 Timer peripheral at 1 MHz)
- Continuous animation with 16ms frame rate
- PWM configuration: 1 kHz with phase-correct mode

**Behavior:**
- White LED smoothly breathes in and out with a sine wave pattern
- Brightness oscillates between dim (10%) and full (100%)
- Creates a calming, natural breathing effect
- Runs infinitely

**Run:**
```bash
cargo run --release --bin breathing
```

Or build UF2 and flash via bootloader:
```bash
cargo build --release --bin breathing
elf2uf2-rs target/thumbv6m-none-eabi/release/breathing breathing.uf2
# Copy breathing.uf2 to RPI-RP2 drive
```

### fade_check

Checks slow fades for banding. This is a diagnostic, not a demonstration of the API.

The other two examples run PWM with `top = 1000`, which maps roughly 65 channel counts onto every duty step — 32x coarser than the Q0.15 progress behind a fade. Banding introduced by the library would disappear into the PWM's own quantization, so this binary uses `top = 65535` with no clock divider. That makes `max_duty_cycle` 65535, the driver's scaling the identity, and the carrier 1.9 kHz.

**Features:**
- Ten-second Linear fades, black to red and black to amber
- Full 16-bit PWM, so a channel value reaches the LED unchanged
- Per-ramp step-size statistics over RTT, computed on the device
- Compile-time constants for the fade duration, the color epsilon and per-frame CSV output

**Expected output:**
```
ramp 0: 625 frames, 624 updates, step 103..106
```

Roughly 625 frames at the 16 ms frame rate, each moving about 105 counts. Frame overhead makes the real count slightly lower and the steps slightly larger. **A minimum step of 0 means a frame produced no change, which is what banding is.**

**What to look at:** PWM duty is linear in light while channel values are gamma-encoded, so a linear ramp appears to move fastest at the start. Watch the first second out of black — that is where the eye is most able to resolve individual steps.

**Two things that produce stepping unrelated to the color math:**

- Raising `FADE_MS` past about 16 seconds. A full-range fade then moves less than `DEFAULT_COLOR_EPSILON` (64) per frame and the sequencer starts suppressing LED writes. At 30 s only half the frames reach the LED. Set `EPSILON` to 0 to take that out of the chain and confirm which one you are looking at.
- Reverting `top` to 1000, which reintroduces the 65-count duty granularity this binary exists to avoid.

**Run:**
```bash
cargo run --release --bin fade_check
```

Or build UF2 and flash via bootloader:
```bash
cargo build --release --bin fade_check
elf2uf2-rs target/thumbv6m-none-eabi/release/fade_check fade_check.uf2
# Copy fade_check.uf2 to RPI-RP2 drive
```
