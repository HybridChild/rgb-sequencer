# Sizeof Calculator

Calculates RAM usage for `RgbSequence<D, N>` and `RgbSequencer` instances using `sizeof`.

```bash
cd tools/sizeof-calculator
cargo run --release
cat report.md
```

## Output

- **Component Sizes:** Individual type sizes (`Srgb`, `TransitionStyle`, `LoopCount`)
- **Duration/Instant Types:** Sizes for u32, u64, Embassy
- **Step Sizes:** Memory per step for each duration type
- **Sequence Tables:** RAM cost by capacity (N=4,8,16,32,64) for each duration type
- **Sequencer Tables:** Total RAM including LED implementation overhead

## Duration Type Selection

| Type | Size | Range | Use When |
|------|------|-------|----------|
| u32 | 4B | ~49 days | Recommended for most embedded |
| u64 | 8B | Unlimited | Long-running sequences only |
| Embassy | 8B | Unlimited | Using Embassy framework |

## Key Insights

- Sequence overhead is constant; only step storage scales with N
- LED implementation size directly affects sequencer total
- Host sizes shown (64-bit); embedded 32-bit slightly smaller (pointers only)
- Step storage costs identical across architectures

## Complementary Tool

Use [binary-analyzer](../binary-analyzer/) for Flash/RAM binary measurements on embedded targets.
