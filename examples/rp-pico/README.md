# Raspberry Pi Pico Examples

Examples for Raspberry Pi Pico board using PWM-based RGB LED control.

- **[blinky](#blinky)** - Simple bare-metal example demonstrating basic RGB LED sequencing with PWM control and a clean, blocking delay approach. Perfect starting point for learning the library on RP2040.

## Hardware Setup

### RGB LED Connections

These examples use **one external RGB LED** controlled via PWM. Connect it to the following GPIO pins with appropriate current-limiting resistors:

**LED 1:**
- **Red**: GPIO2 (PWM1 Channel A)
- **Green**: GPIO3 (PWM1 Channel B)
- **Blue**: GPIO4 (PWM2 Channel A)
- **Common**: 3.3V (for common anode) or GND (for common cathode)

### PWM Configuration

The examples configure PWM with:
- **PWM Frequency**: 1 kHz (125 MHz system clock / 125 divider / 1000 top value)
- **Resolution**: 10-bit (0-1000 duty cycle range)
- **Mode**: Phase-correct PWM for smoother output

### Viewing Logs

The examples use RTT (Real-Time Transfer) for logging.

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
- Single RGB LED with colorful sequence (yellow, cyan, purple) using PWM
- Infinite loop with Step and Linear transitions
- Blocking approach using Cortex-M `Delay`
- Manual time source (counter-based, no interrupts)
- Zero-duration steps for instant color changes
- RTT logging for debugging
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
