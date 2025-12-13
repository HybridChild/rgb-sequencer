# RGB Sequencer Benchmark

On-device performance benchmarking tool for measuring `service()` method execution time across different transition styles and sequence capacities.

## Purpose

Measures **absolute CPU cycles** on real ARM Cortex-M hardware to:
- Compare transition style performance (Step, Linear, EaseIn, EaseOut, EaseInOut)
- Show capacity scaling behavior (N=4 vs N=8 vs N=16 vs N=64)
- Identify worst-case scenarios
- Demonstrate FPU vs non-FPU performance impact

## Target Hardware

**Default**: Raspberry Pi Pico (RP2040)
- Cortex-M0+ at 125 MHz
- **No FPU** - shows software f32 emulation overhead
- Hardware TIMER (1 MHz) for microsecond-precision measurements
- Converts to cycle counts at 125 MHz (125 cycles/microsecond)

**Adapting to other targets**: Modify `Cargo.toml` and `.cargo/config.toml` to target STM32, ESP32, etc.

## Prerequisites

```bash
# Install probe-rs (for flashing and RTT output)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh

# Install ARM Cortex-M target
rustup target add thumbv6m-none-eabi
```

## Running the Benchmark

1. **Connect your Pico** via USB (while holding BOOTSEL for first flash, or just connect if already flashed)

2. **Run the benchmark**:
   ```bash
   cd tools/benchmark
   cargo run --release
   ```

3. **View results** - Output appears via RTT (Real-Time Transfer):
   ```
   === RGB Sequencer Benchmark ===
   System clock: 125000000 Hz
   Warmup iterations: 100
   Benchmark iterations: 1000

   ╔═══════════════════════════════════════════════════════════════════════════════════╗
   ║ Benchmark 1: Transition Styles (N=4, 50% progress)                               ║
   ╚═══════════════════════════════════════════════════════════════════════════════════╝
     Step                    250 cycles     2.00 µs  │  min    250  max    250  median    250
     Linear                 2500 cycles    20.00 µs  │  min   2500  max   2625  median   2500
     EaseIn                 3125 cycles    25.00 µs  │  min   3125  max   3250  median   3125
     EaseOut                3375 cycles    27.00 µs  │  min   3375  max   3500  median   3375
     EaseInOut              3625 cycles    29.00 µs  │  min   3625  max   3750  median   3625

   ╔═══════════════════════════════════════════════════════════════════════════════════╗
   ║ Benchmark 2: Capacity Scaling (Linear transition)                                ║
   ╚═══════════════════════════════════════════════════════════════════════════════════╝
     N=4                    2500 cycles    20.00 µs  │  min   2500  max   2625  median   2500
     N=8                    2500 cycles    20.00 µs  │  min   2500  max   2625  median   2500
     N=16                   2625 cycles    21.00 µs  │  min   2625  max   2750  median   2625
     N=64                   2625 cycles    21.00 µs  │  min   2625  max   2750  median   2625

   ╔═══════════════════════════════════════════════════════════════════════════════════╗
   ║ Benchmark 3: Worst Case Scenarios                                                ║
   ╚═══════════════════════════════════════════════════════════════════════════════════╝
     Worst: EaseInOut N=64  3750 cycles    30.00 µs  │  min   3750  max   3875  median   3750
     Best: Step N=4          250 cycles     2.00 µs  │  min    250  max    250  median    250

   ╔═══════════════════════════════════════════════════════════════════════════════════╗
   ║ Benchmark Complete - All measurements at 125 MHz                                 ║
   ╚═══════════════════════════════════════════════════════════════════════════════════╝

   ┌─────────────────────────────────────────────────────────────────────────────────┐
   │ Performance Summary                                                             │
   ├─────────────────────────────────────────────────────────────────────────────────┤
   │                                                                                 │
   │ At 60 FPS (16.67ms frame time):                                                │
   │   • Step transitions:      ~0.001%  CPU usage                                  │
   │   • Linear transitions:    ~0.020%  CPU usage                                  │
   │   • Easing transitions:    ~0.030%  CPU usage                                  │
   │                                                                                 │
   │ Maximum service() calls per second:                                            │
   │   • Step N=4:              ~500,000/sec  (2 µs each)                           │
   │   • Linear N=4:             ~50,000/sec  (20 µs each)                          │
   │   • EaseInOut N=64:         ~20,000/sec  (50 µs each)                          │
   │                                                                                 │
   └─────────────────────────────────────────────────────────────────────────────────┘
   ```

## Interpreting Results

### Understanding the Numbers

Each benchmark shows:
- **Mean cycles and microseconds**: Average execution time
- **Min/Max/Median**: Statistical distribution
- **Microsecond precision**: ~1 µs granularity (125 cycle increments)

At 125 MHz (RP2040):
- **125 cycles** = 1 microsecond
- **1000 cycles** = 8 microseconds
- **125,000 cycles** = 1 millisecond

### What the Benchmarks Measure

**Benchmark 1**: Transition style overhead
- Compares Step (no interpolation) vs Linear vs easing functions
- All measured at 50% progress through a 1-second transition
- Shows relative cost of f32 math on non-FPU target

**Benchmark 2**: Capacity scaling
- Tests if performance degrades with more steps
- Should show minimal impact (O(1) evaluation)

**Benchmark 3**: Worst-case scenarios
- `EaseInOut N=64`: Maximum complexity (most math, largest sequence)
- `Step N=4`: Minimum complexity (no math, small sequence)
- Shows performance range

### Statistical Metrics

- **min/max**: Shows variance (cache misses, interrupts)
- **median**: Most representative (robust to outliers)
- **mean**: Average performance

## Key Findings

Expected results on RP2040 (Cortex-M0+, no FPU):

1. **Step transitions are ~5-8x faster** than interpolating transitions
2. **Easing functions add ~30-50% overhead** vs Linear
3. **Capacity has minimal impact** (<10% difference N=4 to N=64)
4. **Worst case is still fast** (~2000 cycles = 16 microseconds at 125 MHz)

## Comparing FPU vs Non-FPU Targets

To see FPU impact, run on both:
- **Cortex-M0/M0+** (RP2040, STM32F0) - Software f32 emulation
- **Cortex-M4F** (STM32F4, nRF52) - Hardware FPU

Expected difference:
- **Step transitions**: Similar (minimal f32 math)
- **Linear transitions**: ~3-5x faster on M4F
- **Easing transitions**: ~5-10x faster on M4F

## Modifying the Benchmark

Edit `src/main.rs` to:
- Change iteration counts (`WARMUP_ITERATIONS`, `BENCH_ITERATIONS`)
- Add new transition patterns
- Test function-based sequences
- Measure at different time points (0%, 25%, 75%, 100%)

## Output Format

Results are logged via **RTT** (Real-Time Transfer):
- No serial port needed
- Low overhead (doesn't affect measurements)
- Integrates with probe-rs

## Troubleshooting

**"probe-rs: No probe found"**
- Check USB connection
- Try `probe-rs list` to verify detection
- Hold BOOTSEL while connecting (RP2040)

**Unexpected cycle counts**
- Verify optimization level (`opt-level = 3` in release profile)
- Check for interrupts (disable if needed)
- Note: Measurements have ~125 cycle granularity (1 microsecond at 125 MHz)

**Compilation errors**
- Verify target installed: `rustup target list --installed`
- Check `Cargo.toml` dependencies match your HAL version

## License

Same as parent project (MIT/Apache-2.0).
