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

## Building and Flashing

### Prerequisites

- Rust toolchain with `thumbv6m-none-eabi` target
- probe-rs or picotool for flashing
- (Optional) probe-rs or RTT viewer for viewing logs

### Install Target
```bash
rustup target add thumbv6m-none-eabi
```

### Build
```bash
cargo build --release --bin <example_name>
```

### Flash

#### Method 1: probe-rs (recommended for debugging)
With a debug probe connected:
```bash
cargo run --release --bin <example_name>
```

Or manually:
```bash
probe-rs run --chip RP2040 target/thumbv6m-none-eabi/release/<example_name>
```

#### Method 2: UF2 bootloader (no debug probe needed)
1. Build the UF2 file:
```bash
cargo build --release --bin <example_name>
elf2uf2-rs target/thumbv6m-none-eabi/release/<example_name> <example_name>.uf2
```

2. Enter bootloader mode:
   - Hold BOOTSEL button while connecting USB
   - Or hold BOOTSEL and press RESET if already connected

3. Copy the UF2 file to the RPI-RP2 drive that appears

### Viewing Logs

The examples use RTT (Real-Time Transfer) for logging. To view logs with probe-rs:
```bash
probe-rs attach --chip RP2040
```

Or run directly with logging:
```bash
cargo run --release --bin <example_name>
```

## Common Anode vs Common Cathode

The examples assume a **common anode** RGB LED (common pin connected to 3.3V).

If you have a **common cathode** LED (common pin connected to GND), change the last parameter in `PwmRgbLed::new()` to `false`:
```rust
let led = PwmRgbLed::new(red_channel, green_channel, blue_channel, false);
```

## Examples

### blinky

A simple, clean example demonstrating basic LED sequencing with blocking delays on the RP2040. This is the perfect starting point for learning the library on Raspberry Pi Pico.

**Features:**
- **Single RGB LED**: Controls one RGB LED through a colorful sequence using PWM
- **Infinite loop**: Sequence repeats continuously forever
- **Blocking approach**: Uses Cortex-M `Delay` for simple, easy-to-understand timing
- **Simple time source**: Advances time manually after each delay
- **Zero-duration steps**: Demonstrates instant color changes before fade-outs
- **RTT logging**: Real-time logging for debugging and monitoring
- No interrupt handlers or manual WFI calls needed

**What you'll learn:**
- Basic sequencer usage with minimal setup on RP2040
- PWM-based RGB LED control on Raspberry Pi Pico
- How to create sequences with steps and transitions
- The difference between Step (instant) and Linear (fade) transitions
- Zero-duration steps as color waypoints
- Infinite loop sequencing
- Simple blocking delay pattern

**Technical Highlights:**
This example shows the simplest possible integration pattern for RP2040:
```rust
// Create a simple time source
let time_source = BlinkyTimeSource::new();

// Service loop
loop {
    if let Some(delay_duration) = sequencer.service().unwrap() {
        if delay_duration == TimeDuration::ZERO {
            // Linear transition - maintain frame rate
            delay.delay_ms(FRAME_RATE_MS as u32);
            time_source.advance(BlinkyDuration(FRAME_RATE_MS));
        } else {
            // Step transition - delay for the specified time
            delay.delay_ms(delay_duration.as_millis() as u32);
            time_source.advance(delay_duration);
        }
    } else {
        // Sequence complete
        break;
    }
}
```

The time source is dead simple - just a counter that advances after each delay. This works perfectly for applications where the sequencer is the only thing happening.

**PWM Setup:**
The example configures three PWM channels for RGB control:
- PWM1 handles Red (Channel A) and Green (Channel B) on GPIO2 and GPIO3
- PWM2 handles Blue (Channel A) on GPIO4
- Both slices run at 1 kHz with phase-correct mode for smooth color transitions

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
