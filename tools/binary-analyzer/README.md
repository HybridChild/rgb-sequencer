# Memory Footprint Analysis

This directory contains tools for analyzing the memory footprint of the **rgb-sequencer** library on embedded ARM Cortex-M targets.

## Quick Start

```bash
cd size-analysis
./analyze.sh
cat report.md
```

This builds a minimal reference binary for multiple ARM Cortex-M targets and generates a comparative size report.

## What Gets Analyzed

The analysis uses `size-analysis/minimal/` - a bare-bones embedded binary with:

- **Single 4-step sequence**: Red → Green → Blue → White with Linear and Step transitions
- **All features exercised**: `start_color`, `landing_color`, finite loop count
- **All API methods exercised**: `load`, `start`, `service`, `pause`, `resume`, `restart`, `state`, `clear`
- **Minimal trait implementations**: Zero-size stubs for `RgbLed` and `TimeSource` to isolate library overhead
- **Multiple targets**: `thumbv6m-none-eabi` (Cortex-M0/M0+) and `thumbv7em-none-eabihf` (Cortex-M4F/M7)
- **Production optimization**: `opt-level = "z"` with LTO enabled

## Reading the Report

The generated `report.md` contains:

1. **Target Comparison Table** - Side-by-side Flash/RAM usage for Cortex-M0 vs Cortex-M4F
2. **Per-Target Analysis**:
   - Binary size breakdown (.text, .rodata, .data, .bss)
   - Top 20 largest symbols showing what contributes to Flash usage
3. **Interpretation Guide**:
   - Test scenario description
   - Binary section meanings
   - Symbol analysis guide
   - How to estimate your application's total size

### Key Insights from the Report

- **FPU Impact**: Compare Cortex-M0 (software f32) vs Cortex-M4F (hardware FPU) overhead
- **Symbol analysis**: Identifies library code (`RgbSequencer::service`) vs compiler overhead (`__divsf3`, etc.)
- **Baseline cost**: Minimum library overhead with production optimization

## Understanding Your Total Cost

The analysis measures **rgb-sequencer's baseline overhead** using minimal stub implementations. Your actual binary will include:

**Measured baseline** (from report.md)
- Library code (sequencer logic, builder)
- Software f32 emulation (non-FPU targets only)
- Minimal test sequence

**Your additions** (not measured):
- `RgbLed` trait implementation: PWM/SPI drivers
- `TimeSource` trait implementation: Timer integration
- Your sequence data: Additional sequencers and sequences

## FPU Considerations

This library uses `f32` for color math and interpolation:

- **Cortex-M0/M0+/M3** (no FPU): Software f32 emulation adds overhead (visible as `compiler_builtins` symbols)
- **Cortex-M4F/M7/M33** (with FPU): Hardware f32 operations, no emulation overhead

The report compares both to show the FPU impact clearly.

To minimize overhead on non-FPU targets:
- Use `TransitionStyle::Step` instead of `Linear` (avoids interpolation)
- Keep sequences simple
- Avoid function-based sequences with f32 operations
