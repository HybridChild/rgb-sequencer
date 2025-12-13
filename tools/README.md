# Development Tools

## [sizeof-calculator](sizeof-calculator/)

Calculates RAM usage for `RgbSequence` and `RgbSequencer` instances using `sizeof`. Generates `report.md` with component sizes, step costs, and memory tables for capacities 4-64 across different duration types and LED implementations.

```bash
cd tools/sizeof-calculator
cargo run --release
cat report.md
```

**Use for:** Capacity planning, type selection, memory budgeting during design.

## [binary-analyzer](binary-analyzer/)

Analyzes compiled binaries for Flash/RAM overhead on embedded ARM targets. Builds minimal reference binary for Cortex-M0/M4F targets and generates `report.md` with section breakdown and symbol-level analysis.

```bash
cd tools/binary-analyzer
./analyze.sh
cat report.md
```

**Use for:** Binary footprint measurement, FPU impact analysis, release optimization.

## [benchmark](benchmark/)

On-device performance benchmarking tool measuring absolute CPU cycles for `service()` method. Tests different transition styles (Step, Linear, EaseIn/Out/InOut) and capacities (N=4 to N=64) on real ARM Cortex-M hardware using DWT cycle counter.

```bash
cd tools/benchmark
cargo run --release  # Requires connected RP2040 (or configured target)
```

**Use for:** Performance measurement, transition style comparison, FPU vs non-FPU impact analysis.

---

## Tool Comparison

| Tool | Purpose | Speed | Output |
|------|---------|-------|--------|
| **sizeof-calculator** | RAM planning (host, sizeof) | Instant | Component/sequence sizes |
| **binary-analyzer** | Flash/RAM measurement (target, compiled) | Slow (cross-compile) | Binary sections, symbols |
| **benchmark** | Performance measurement (on-device, cycles) | Moderate (requires hardware) | Cycle counts, statistics |
