# RGB Sequencer Tools

Utility programs for working with the rgb-sequencer library.

## sequence_memory_calculator

A utility that calculates and displays the exact memory footprint of RGB sequences with different configurations.

### Usage

```bash
cd tools
cargo run --bin sequence_memory_calculator
```

### What It Shows

The memory calculator displays:

1. **Component Sizes**: Individual sizes of `Srgb`, `TransitionStyle`, `LoopCount`, etc.
2. **Duration Type Sizes**: Sizes for common duration types (u32, u64, Embassy)
3. **Step Sizes**: Memory required per step for different duration types
4. **Sequence Tables**: Complete memory breakdown for sequences with capacities from 4 to 64

**Note on Architecture**: This tool runs on your host machine (requires `std`) and shows sizes for your host architecture (x86-64, ARM64, etc.). If you're developing on a 64-bit machine for a 32-bit embedded target (like ARM Cortex-M), the actual embedded sizes will be slightly smaller due to pointer size differences (4 bytes vs 8 bytes). The step storage costs remain the same, so these numbers provide a conservative estimate for memory planning.

### Example Output

```
╔════════════════════════════════════════════════════════════════╗
║        RGB Sequencer Memory Footprint Calculator              ║
╚════════════════════════════════════════════════════════════════╝

Component Sizes:
├─ Srgb (f32 RGB):              12 bytes
├─ TransitionStyle (enum):      1 bytes
├─ LoopCount (enum):             8 bytes
├─ Option<Srgb>:                 16 bytes
├─ Color function pointer:       8 bytes
└─ Timing function pointer:      8 bytes

Duration Type Sizes:
├─ u32 (milliseconds):           4 bytes
├─ u64 (milliseconds):           8 bytes
└─ Embassy Duration (ticks):     8 bytes

Step Sizes (by duration type):
├─ SequenceStep<u32>:            20 bytes
├─ SequenceStep<u64>:            24 bytes
└─ SequenceStep<EmbassyDuration>: 24 bytes

RgbSequence<u64, N> Memory Usage:
┌──────────┬──────────────┬─────────────────┬────────────────┐
│ Capacity │ Sequence     │ Storage Cost    │ Overhead       │
│ (N)      │ Total Size   │ (Step size * N) │ (Fixed)        │
├──────────┼──────────────┼─────────────────┼────────────────┤
│    4     │        168 B │            96 B │           72 B │
│    8     │        264 B │           192 B │           72 B │
│    16    │        456 B │           384 B │           72 B │
│    32    │        840 B │           768 B │           72 B │
│    64    │       1608 B │          1536 B │           72 B │
└──────────┴──────────────┴─────────────────┴────────────────┘
```

**Understanding the Table Columns:**

- **Capacity (N)**: The maximum number of steps the sequence can hold (const generic parameter)
- **Sequence Total Size**: The complete size of `RgbSequence<D, N>` in memory
- **Storage Cost (Step size * N)**: Memory used to store the steps array (`N * sizeof(SequenceStep<D>)`)
  - This scales linearly with capacity
  - Example: If each step is 24 bytes, then 8 steps = 192 bytes, 16 steps = 384 bytes
- **Overhead (Fixed)**: Memory used by sequence metadata that doesn't depend on capacity
  - Includes: loop count, start/landing colors, loop duration, function pointers, vector length
  - This stays constant regardless of capacity
  - Example: ~104 bytes for loop control, color options, and bookkeeping

**Key Insight**: 
- The overhead is constant - paid once per sequence regardless of capacity
- Storage is allocated for the full capacity, even if you use fewer steps

### Why Use It?

- **Compare duration types**: See how different duration implementations affect memory
- **Optimize capacity choices**: Make informed decisions about the `N` parameter
- **Budget memory**: Know exactly how much RAM your sequences will consume
