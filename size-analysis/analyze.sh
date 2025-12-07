#!/bin/bash
set -e

# Memory footprint analysis script for rgb-sequencer
# Builds minimal reference binary and generates a detailed size report

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINIMAL_DIR="$SCRIPT_DIR/minimal"
REPORT="$SCRIPT_DIR/report.md"
TARGET="thumbv6m-none-eabi"
BINARY_NAME="minimal"

# Color output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

printf "${BLUE}=== rgb-sequencer Memory Footprint Analysis ===${NC}\n"
printf "\n"

# Check for required tools
echo "Checking for required tools..."
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found"
    exit 1
fi

if ! command -v cargo-bloat &> /dev/null; then
    echo "Installing cargo-bloat..."
    cargo install cargo-bloat
fi

# Check if target is installed
if ! rustup target list | grep -q "$TARGET (installed)"; then
    echo "Installing target $TARGET..."
    rustup target add $TARGET
fi

# Navigate to minimal directory
cd "$MINIMAL_DIR"

# Remove old report to start fresh
rm -f "$REPORT"

printf "${GREEN}Building and analyzing...${NC}\n"

# Verify required files exist
if [ ! -f "memory.x" ]; then
    echo "  Warning: memory.x not found - linking may fail"
fi

if [ ! -f ".cargo/config.toml" ]; then
    echo "  Warning: .cargo/config.toml not found - rustflags will not be applied"
fi

# Build with no default features (baseline)
echo "  Building..."
if ! cargo build --release --target $TARGET --no-default-features; then
    echo "  Error: Build failed"
    exit 1
fi

# Get binary path
BINARY_PATH="target/$TARGET/release/$BINARY_NAME"

if [ ! -f "$BINARY_PATH" ]; then
    echo "  Error: Binary not found at $BINARY_PATH"
    exit 1
fi

# Extract size information using detailed section breakdown
echo "  Running size analysis..."

# Get detailed section breakdown using -A format
SIZE_DETAIL=$(cargo size --release --target $TARGET --no-default-features -- -A 2>/dev/null) || true

# Extract individual sections
TEXT=$(echo "$SIZE_DETAIL" | grep "^\.text" | awk '{print $2}' || echo "0")
RODATA=$(echo "$SIZE_DETAIL" | grep "^\.rodata" | awk '{print $2}' || echo "0")
DATA=$(echo "$SIZE_DETAIL" | grep "^\.data" | awk '{print $2}' || echo "0")
BSS=$(echo "$SIZE_DETAIL" | grep "^\.bss" | awk '{print $2}' || echo "0")

# Validate we got numeric values
if ! [[ "$TEXT" =~ ^[0-9]+$ ]]; then
    TEXT=0
fi
if ! [[ "$RODATA" =~ ^[0-9]+$ ]]; then
    RODATA=0
fi
if ! [[ "$DATA" =~ ^[0-9]+$ ]]; then
    DATA=0
fi
if ! [[ "$BSS" =~ ^[0-9]+$ ]]; then
    BSS=0
fi

# Warn if binary has no loadable sections
if [ "$TEXT" = "0" ] && [ "$RODATA" = "0" ] && [ "$DATA" = "0" ] && [ "$BSS" = "0" ]; then
    echo "  Warning: Binary has no loadable sections (check .cargo/config.toml and memory.x)"
fi

# Calculate total flash: .text + .rodata + .data
TOTAL_FLASH=$((TEXT + RODATA + DATA))

# Generate report
cat > "$REPORT" <<EOF
# rgb-sequencer Memory Footprint Analysis

**Generated:** $(date)  
**Target:** $TARGET (ARMv6-M, Cortex-M0/M0+)  
**Optimization:** \`opt-level = "z"\`, LTO enabled

This analysis uses a minimal reference binary to measure the baseline overhead of rgb-sequencer.

## Binary Size

| .text (Flash) | .rodata (Flash) | .data (RAM) | .bss (RAM) | Total Flash |
|---------------|-----------------|-------------|------------|-------------|
| ${TEXT}B | ${RODATA}B | ${DATA}B | ${BSS}B | ${TOTAL_FLASH}B |

---

### Binary Size Breakdown

\`\`\`
EOF

# Use cargo size with berkeley format for memory sections
cargo size --release --target $TARGET --no-default-features >> "$REPORT" 2>&1 || true

cat >> "$REPORT" <<EOF


Detailed sections:
EOF

# Also show detailed section breakdown
cargo size --release --target $TARGET --no-default-features -- -A >> "$REPORT" 2>&1 || true

cat >> "$REPORT" <<EOF
\`\`\`

### Top 10 Largest Symbols (Flash Usage)

\`\`\`
EOF

# Run cargo-bloat for symbol-level analysis
# Note: May fail if symbols are stripped
BLOAT_OUTPUT=$(cargo bloat --release --target $TARGET --no-default-features -n 10 2>&1 || echo "Symbol analysis not available (binary may be stripped)")
echo "$BLOAT_OUTPUT" >> "$REPORT"

cat >> "$REPORT" <<EOF
\`\`\`
EOF

# Add interpretation section
cat >> "$REPORT" <<EOF

---

## Interpretation Guide

**Build configuration:** Release build optimized for size (\`opt-level = "z"\`, LTO enabled) targeting $TARGET (Cortex-M0/M0+).

### Section Meanings

- **.text**: Executable code (stored in Flash)
- **.rodata**: Read-only data like constants and string literals (stored in Flash)
- **.data**: Initialized variables (stored in Flash, copied to RAM at startup)
- **.bss**: Uninitialized/zero-initialized variables (RAM only, no Flash cost)

### Total Memory Cost

- **Flash usage** = .text + .rodata + .data
- **RAM usage** = .data + .bss + stack

### Understanding the Measurements

This analysis uses **minimal stub implementations** to measure only rgb-sequencer's code overhead:

- **MinimalLed**: Zero-size LED implementation (no-op)
- **Duration32/Instant32**: Simple u32-based time types
- **MinimalTimeSource**: Zero-size time source

**Your actual binary will be larger** due to:

1. **Your trait implementations**:
   - \`RgbLed\` - PWM setup, SPI drivers, gamma correction (~100-500 bytes)
   - \`TimeSource\` - SysTick, HAL timer, Embassy integration (~50-200 bytes)

2. **Your sequence data**:
   - Each \`SequenceStep\` adds ~20 bytes (12B Srgb + 4B duration + 4B transition/padding)
   - Capacity N sequences: ~(20 × N) bytes + overhead
   - Static sequences stored in Flash (.rodata)

3. **Compiler optimizations**:
   - Generic monomorphization creates specialized code per type
   - Inlining may increase code size but improve performance
   - Different time types (u32 vs u64) affect size

### Test Sequence Capacities

The minimal binary exercises:
- **4-step sequence** - Basic color transitions
- **8-step sequence** - Medium complexity with pause/resume
- **16-step sequence** - Larger sequence with start/landing colors
- **Function-based sequence** (0 capacity) - Algorithmic animation

This ensures the optimizer doesn't remove unused code paths.

### Optional Features

The size analysis focuses on the baseline library overhead. Optional features have the following typical impact:

| Feature | Flash Impact | RAM Impact | Notes |
|---------|--------------|------------|-------|
| defmt | ~500B-1KB | 0 | Debug logging - actual cost depends on usage and transport (defmt-rtt, etc.) |

**Note:** The \`std\` feature is for host builds/tests only and cannot be used with embedded targets.

### Optimization Strategies

If you need to reduce size:

1. **Use Step transitions** instead of Linear (avoids f32 interpolation math)
2. **Minimize sequence capacity** - Use smallest N that fits your needs
3. **Function-based sequences** - Zero storage (N=0) for algorithmic patterns
4. **Simple time types** - u32 milliseconds smaller than u64 or complex types
5. **Disable unused features** - Use \`default-features = false\` in Cargo.toml

### Platform Considerations

**FPU Impact:** This library uses f32 for color math. On Cortex-M0/M0+ (no FPU),
software float emulation adds ~1-2KB. Cortex-M4F/M7 with FPU have no such overhead.

**Target-specific:** Cortex-M0 (thumbv6m) has the most constrained instruction set.
Actual size on Cortex-M4 (thumbv7em) may differ slightly due to different instructions.

EOF

printf "\n"
printf "${GREEN}✓ Analysis complete!${NC}\n"
printf "Report generated at: ${BLUE}%s${NC}\n" "$REPORT"
printf "\n"
echo "To view the report:"
echo "  cat $REPORT"
echo "  or open $REPORT in your editor"

# Clean up build artifacts
printf "\n"
printf "${GREEN}Cleaning up build artifacts...${NC}\n"
cargo clean --quiet
cd ..
