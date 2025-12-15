# Raspberry Pi Pico Examples

Examples for Raspberry Pi Pico board using PWM-based RGB LED control.

- **[blinky](#blinky)** - Simple bare-metal example demonstrating basic RGB LED sequencing with PWM control and a clean, blocking delay approach. Perfect starting point for learning the library on RP2040.
- **[breathing](#breathing)** - White breathing effect using function-based animation with sine wave modulation. Demonstrates algorithmic color generation and smooth brightness transitions.

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
