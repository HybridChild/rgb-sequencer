# Memory Footprint Analysis

This directory contains tools for analyzing the memory footprint of the **rgb-sequencer** library across different feature combinations.

## Quick Start

```bash
cd size-analysis
./analyze.sh
cat report.md
```

This will build a minimal reference binary with all feature combinations and generate a detailed size report.

## What Gets Analyzed

The analysis uses `size-analysis/minimal/` - a bare-bones embedded binary with:

- **Minimal trait implementations**: Zero-size stubs for `RgbLed` and `TimeSource` to isolate library overhead
- **Multiple sequence capacities**: Tests 4, 8, and 16-step sequences plus function-based (0 capacity)
- **Real embedded target**: `thumbv6m-none-eabi` (Cortex-M0/M0+, most constrained target)
- **Size-optimized build**: `opt-level = "z"` with LTO enabled
- **All transitions exercised**: Step and Linear transitions, pause/resume, start/landing colors

### Feature Combinations Tested

1. **none** - Minimal build (no features)
2. **defmt** - Debug formatting support

**Note:** The `std` feature is for host builds/tests only and cannot be used with embedded targets (`thumbv6m-none-eabi`).

## Reading the Report

The generated `report.md` contains:

1. **Summary Table** - Quick overview of Flash/RAM usage for each feature combination
2. **Detailed Analysis** - Per-feature breakdown with:
   - Binary size by section (.text, .rodata, .data, .bss)
   - Top 10 largest symbols contributing to Flash usage
3. **Interpretation Guide** - Detailed explanations including:
   - Section meanings (.text, .rodata, .data, .bss)
   - How your trait implementations add to baseline measurements
   - Sequence capacity impact on memory
   - Optimization strategies for reducing footprint

### Understanding the Output

**Quick reference for memory calculations:**

- **Flash usage** = `.text` + `.rodata` + `.data` (code + constants + initialized data)
- **RAM usage** = `.data` + `.bss` + stack (initialized data + zero-initialized + runtime stack)

The report's **Interpretation Guide** provides complete details on:
- What each section means and where it's stored
- How your trait implementations add to these baseline numbers
- Sequence capacity scaling (each step adds ~20 bytes)
- Feature-by-feature impact analysis

### Key Insights from the Report

- **Baseline overhead**: Check the "none" configuration for minimum rgb-sequencer footprint
- **Feature costs**: Compare each feature against baseline to see incremental cost
- **Symbol analysis**: Identify which functions consume the most Flash
- **Capacity scaling**: Understand how sequence capacity affects memory

## Using the Results

### Optimization Strategies

If you need to reduce size:

1. **Minimize sequence capacity**: Use smallest `N` that fits your animation
2. **Use Step transitions**: Avoid Linear transitions (saves f32 interpolation code)
3. **Function-based sequences**: Use `N=0` for algorithmic patterns (zero storage)
4. **Simple time types**: u32 milliseconds smaller than u64 or complex Embassy types
5. **Disable unused features**: Use `default-features = false` in your `Cargo.toml`
6. **Target with FPU**: Cortex-M4F/M7 have hardware f32, saving ~1-2KB vs M0

### Understanding Your Total Cost

The analysis measures **rgb-sequencer's overhead** using minimal stub implementations. Your actual binary will be larger due to:

- Your `RgbLed` implementation (PWM setup, SPI drivers, gamma tables: ~100-500 bytes)
- Your `TimeSource` implementation (SysTick, HAL timer, Embassy: ~50-200 bytes)
- Your sequence data (each step: ~20 bytes; capacity N: ~20N bytes)
- Static sequences stored in Flash (.rodata section)

See the report's "Interpretation Guide" for detailed explanations of how to calculate your total memory footprint.

## Sequence Capacity Reference

Each `SequenceStep` is approximately **20 bytes**:
- Srgb (f32 × 3): 12 bytes
- Duration: 4 bytes (u32) or 8 bytes (u64)
- TransitionStyle: 1 byte + padding

**Example sequence sizes:**
- 4 steps: ~80 bytes + overhead
- 8 steps: ~160 bytes + overhead
- 16 steps: ~320 bytes + overhead
- Function-based (0 capacity): ~40 bytes (function pointers only)

## CI Integration

The `.github/workflows/size-analysis.yml` workflow:

- Runs on PRs and main branch pushes
- Generates size reports as artifacts
- Comments on PRs with summary table (informational only)
- Does NOT fail builds on size increases

Size increases may be justified for feature additions. The workflow provides data for informed decisions.

## FPU Considerations

**Important:** This library uses `f32` extensively for color math and interpolation.

- **Cortex-M0/M0+/M3** (no FPU): Software float emulation adds ~1-2KB
- **Cortex-M4F/M7/M33** (with FPU): Hardware acceleration, no additional overhead

The analysis targets Cortex-M0 (worst case). If your target has FPU, expect lower overhead.

To minimize f32 cost on non-FPU targets:
- Use `TransitionStyle::Step` (no interpolation)
- Avoid complex function-based sequences with math
- Consider pre-computed lookup tables for smooth animations
