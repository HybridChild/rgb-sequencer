# Development Tools

## [memory-calculator](memory-calculator/)

Calculates RAM usage for `RgbSequence` and `RgbSequencer` instances with different configurations. Generates `report.md` with component sizes, step costs, and memory tables for capacities 4-64 across different duration types and LED implementations.

```bash
cd tools/memory-calculator
cargo run --release
cat report.md
```

**Use for:** Capacity planning, type selection, memory budgeting during design.

---

## Memory Analysis Tools

| Tool | Purpose | When to Use |
|------|---------|-------------|
| **memory-calculator** | RAM planning (host) | Design phase, quick estimates |
| **size-analysis** | Flash/RAM measurement (target) | Binary optimization, release builds |
