# RGB Sequencer Tools

Utility programs for working with the rgb-sequencer library.

## memory_calculator

A utility that calculates and displays the exact memory footprint of `RgbSequence` and `RgbSequencer` instances with different configurations.

### Usage

```bash
cd tools
cargo run --bin memory_calculator
```

### What It Shows

The memory calculator displays:

1. **Component Sizes**: Individual sizes of `Srgb`, `TransitionStyle`, `LoopCount`, etc.
2. **Duration Type Sizes**: Sizes for common duration types (u32, u64, Embassy)
3. **Instant Type Sizes**: Sizes for common instant types (u32, u64, Embassy)
4. **LED Implementation Sizes**: Sizes for different LED driver implementations (Small/Medium/Large)
5. **Step Sizes**: Memory required per step for different duration types
6. **Sequence Tables**: Complete memory breakdown for sequences with capacities from 4 to 64
7. **Sequencer Tables**: Complete memory breakdown for full RgbSequencer instances with different LED implementations

### Architecture Notes
This tool runs on your host machine (requires `std`) and shows sizes for your host architecture (x86-64, ARM64, etc.). If you're developing on a 64-bit machine for a 32-bit embedded target (like ARM Cortex-M), the actual embedded sizes will be slightly smaller due to pointer size differences (4 bytes vs 8 bytes). The step storage costs remain the same, so these numbers provide a conservative estimate for memory planning.
