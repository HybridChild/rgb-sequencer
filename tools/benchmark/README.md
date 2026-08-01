# rgb-sequencer Benchmark tool

On-device performance benchmarking tool for measuring `service()` method execution time across different transition styles and sequence capacities.

## Purpose

Measures **absolute CPU cycles** on real ARM Cortex-M hardware to:
- Compare transition style performance
- Show O(N) capacity scaling behavior
- Identify worst-case performance
- Compare the same integer workload across ARM cores

## Target Hardware

### RP2040 (Raspberry Pi Pico)
- **CPU**: Cortex-M0+ at 125 MHz
- **Target**: `thumbv6m-none-eabi`
- **Shows**: The floor - no hardware multiply-accumulate, no barrel shifter on every operand

### RP2350 (Raspberry Pi Pico 2)
- **CPU**: Cortex-M33F at **150 MHz** (default, 20% faster than RP2040)
- **Target**: `thumbv8m.main-none-eabihf`
- **Shows**: A richer instruction set on the same integer code

Since 0.3.0 the library contains no floating point, so neither target exercises an FPU. The pair is a core-and-clock comparison, not an FPU comparison.

## Running Benchmarks

### Manual

```bash
# RP2040 (Pico)
cd tools/benchmark/rp2040
cargo run --release
```

```bash
# RP2350 (Pico 2)
cd tools/benchmark/rp2350
cargo run --release
```

### Report generator scripts

Use the provided scripts to run benchmarks and save results to markdown files:

```bash
# RP2040 (Pico)
cd tools/benchmark/rp2040
./run_benchmark.sh
# Saves results to `tools/benchmark/rp2040_benchmarks.md`
```

```bash
# RP2350 (Pico 2)
cd tools/benchmark/rp2350
./run_benchmark.sh
# Saves results to `tools/benchmark/rp2350_benchmarks.md`
```

Generated markdown reports include timestamp and toolchain metadata. Results can be committed to track performance over time.

## Interpreting Results

### What is Measured

The benchmark measures **worst-case performance** for all transition styles by:
1. Creating a sequence with N steps
2. Advancing time to **50% through the LAST step**
3. Measuring `service()` execution time in CPU cycles

This reveals the O(N) search cost in `find_step_at_time()`.

### What the Current Results Show

Capacity dominates. On Cortex-M0+ the whole spread between `Linear` and the most expensive easing curve is about 20 cycles, while raising `N` from 4 to 32 adds about 15,000 — nearly three orders of magnitude apart. Pick a transition style for its appearance and size `N` to the sequence you actually need.

Run-to-run variation is a few cycles on Cortex-M0+ and up to ~30 on Cortex-M33F, which is wider than the gap between transition styles. Treat differences of that size as noise; only capacity moves the number meaningfully.

## License

Same as parent project (MIT/Apache-2.0).
