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

- **Architecture comparison**: Cortex-M0/M0+ (ARMv6-M) vs Cortex-M4F/M7 (ARMv7E-M)
- **Symbol analysis**: Identifies library code (`RgbSequencer::service`) vs compiler support routines (`u32_div_rem`, `memcpy`, etc.)
- **Baseline cost**: Minimum library overhead with production optimization

## Understanding Your Total Cost

The analysis measures **rgb-sequencer's baseline overhead** using minimal stub implementations. Your actual binary will include:

**Measured baseline** (from report.md)
- Library code (sequencer logic, builder)
- Integer division support routines
- Minimal test sequence

**Your additions** (not measured):
- `RgbLed` trait implementation: PWM/SPI drivers
- `TimeSource` trait implementation: Timer integration
- Your sequence data: Additional sequencers and sequences

## Floating Point

All color math is fixed-point integer arithmetic, so **no soft-float routines should appear in either binary**. Seeing `__divsf3`, `__addsf3`, `__mulsf3` or `__aeabi_f*` in the symbol lists means floating point has crept back into the library — treat that as a regression.

The two targets are still compared because ARMv6-M lacks instructions ARMv7E-M has (notably hardware divide), so the non-FPU build carries division support routines the other does not.

For reference, dropping `f32` took the non-FPU build from 3784 B to 2104 B (-44%) and the FPU build from 1996 B to 1828 B (-8%).

To minimize overhead:
- Keep sequences small — `N` dominates RAM, and the step search is linear
- Prefer `TransitionStyle::Step` only if you genuinely need no interpolation; it is no longer a meaningful performance lever
