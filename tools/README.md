# Development Tools

## [memory-calculator](memory-calculator/)

Calculates RAM usage for `RgbSequence` and `RgbSequencer` instances. Generates `report.md` with component sizes, step costs, and memory tables for capacities 4-64 across different duration types and LED implementations.

```bash
cd tools/memory-calculator
cargo run --release
cat report.md
```

**Use for:** Capacity planning, type selection, memory budgeting during design.

## [size-analysis](size-analysis/)

Measures compiled Flash/RAM overhead on embedded ARM targets. Builds minimal reference binary for Cortex-M0/M4F targets and generates `report.md` with section breakdown and symbol-level analysis.

```bash
cd tools/size-analysis
./analyze.sh
cat report.md
```

**Use for:** Binary footprint measurement, FPU impact analysis, release optimization.

---

## Tool Comparison

| Tool | Purpose | Speed | Output |
|------|---------|-------|--------|
| **memory-calculator** | RAM planning (host) | Instant | Component/sequence sizes |
| **size-analysis** | Flash/RAM measurement (target) | Slow (cross-compile) | Binary sections, symbols |
