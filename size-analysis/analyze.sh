#!/bin/bash
set -e

# Memory footprint analysis script for rgb-sequencer
# Builds minimal reference binary for multiple targets and generates a comparative size report

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINIMAL_DIR="$SCRIPT_DIR/minimal"
REPORT="$SCRIPT_DIR/report.md"
BINARY_NAME="minimal"

# Define targets to analyze (space-separated pairs of "target|description")
TARGETS=(
    "thumbv6m-none-eabi|ARMv6-M, Cortex-M0/M0+ (no FPU)"
    "thumbv7em-none-eabihf|ARMv7E-M, Cortex-M4F/M7 (with FPU)"
)

# Color output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
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

# Check if all targets are installed
for TARGET_PAIR in "${TARGETS[@]}"; do
    TARGET="${TARGET_PAIR%%|*}"
    if ! rustup target list | grep -q "$TARGET (installed)"; then
        echo "Installing target $TARGET..."
        rustup target add $TARGET
    fi
done

# Navigate to minimal directory
cd "$MINIMAL_DIR"

# Remove old report to start fresh
rm -f "$REPORT"

# Verify required files exist
if [ ! -f "memory.x" ]; then
    echo "  Warning: memory.x not found - linking may fail"
fi

if [ ! -f ".cargo/config.toml" ]; then
    echo "  Warning: .cargo/config.toml not found - rustflags will not be applied"
fi

# Temporary directory for storing results
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

# Function to analyze a single target
analyze_target() {
    local TARGET=$1
    local DESCRIPTION=$2
    local RESULT_FILE="$TMP_DIR/${TARGET}.result"

    printf "${GREEN}Building and analyzing $TARGET...${NC}\n"

    # Build with no default features (baseline)
    echo "  Building..."
    if ! cargo build --release --target $TARGET --no-default-features > /dev/null 2>&1; then
        echo "  Error: Build failed for $TARGET"
        return 1
    fi

    # Get binary path
    BINARY_PATH="target/$TARGET/release/$BINARY_NAME"

    if [ ! -f "$BINARY_PATH" ]; then
        echo "  Error: Binary not found at $BINARY_PATH"
        return 1
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

    # Store results in file
    cat > "$RESULT_FILE" <<EOF
TEXT=$TEXT
RODATA=$RODATA
DATA=$DATA
BSS=$BSS
TOTAL_FLASH=$TOTAL_FLASH
EOF

    # Get Berkeley format size output
    cargo size --release --target $TARGET --no-default-features > "$TMP_DIR/${TARGET}.size.berkeley" 2>&1 || true

    # Get detailed size output
    cargo size --release --target $TARGET --no-default-features -- -A > "$TMP_DIR/${TARGET}.size.detail" 2>&1 || true

    # Get bloat output
    cargo bloat --release --target $TARGET --no-default-features -n 20 -w > "$TMP_DIR/${TARGET}.bloat" 2>&1 || echo "Symbol analysis not available (binary may be stripped)" > "$TMP_DIR/${TARGET}.bloat"

    echo "  ✓ Complete"
}

# Analyze all targets
for TARGET_PAIR in "${TARGETS[@]}"; do
    TARGET="${TARGET_PAIR%%|*}"
    DESCRIPTION="${TARGET_PAIR#*|}"
    analyze_target "$TARGET" "$DESCRIPTION"
    printf "\n"
done

# Generate comparative report
printf "${GREEN}Generating comparative report...${NC}\n"

cat > "$REPORT" <<EOF
# rgb-sequencer Memory Footprint Analysis

**Generated:** $(date)  
**Optimization:** \`opt-level = "z"\`, LTO enabled

This analysis uses a minimal reference binary to measure the baseline overhead of rgb-sequencer across different ARM Cortex-M targets.

## Target Comparison

EOF

# Add comparison table
echo "| Target | Architecture | .text | .rodata | .data | .bss | Total Flash |" >> "$REPORT"
echo "|--------|--------------|-------|---------|-------|------|-------------|" >> "$REPORT"

for TARGET_PAIR in "${TARGETS[@]}"; do
    TARGET="${TARGET_PAIR%%|*}"
    DESCRIPTION="${TARGET_PAIR#*|}"

    # Load results
    source "$TMP_DIR/${TARGET}.result"

    echo "| \`$TARGET\` | $DESCRIPTION | ${TEXT}B | ${RODATA}B | ${DATA}B | ${BSS}B | **${TOTAL_FLASH}B** |" >> "$REPORT"
done

cat >> "$REPORT" <<EOF

---

EOF

# Add detailed sections for each target
for TARGET_PAIR in "${TARGETS[@]}"; do
    TARGET="${TARGET_PAIR%%|*}"
    DESCRIPTION="${TARGET_PAIR#*|}"

    cat >> "$REPORT" <<EOF
## $TARGET ($DESCRIPTION)

### Binary Size Breakdown

\`\`\`
EOF

    cat "$TMP_DIR/${TARGET}.size.berkeley" >> "$REPORT"

    cat >> "$REPORT" <<EOF


Detailed sections:
EOF

    cat "$TMP_DIR/${TARGET}.size.detail" >> "$REPORT"

    cat >> "$REPORT" <<EOF
\`\`\`

### Top 20 Largest Symbols

\`\`\`
EOF

    cat "$TMP_DIR/${TARGET}.bloat" >> "$REPORT"

    cat >> "$REPORT" <<EOF
\`\`\`

---

EOF
done

# Add interpretation section
cat >> "$REPORT" <<EOF

## Interpretation Guide

**Build configuration:** Release build with \`opt-level = "z"\`, LTO enabled (production-grade size optimization).

### Test Scenario

Single 4-step sequence (Red → Green → Blue → White) with:
- Linear and Step transitions
- \`start_color\` and \`landing_color\`
- Finite loop count (3 iterations)
- All API methods exercised: load, start, service, pause, resume, restart, state, clear

Trait implementations are minimal stubs (zero-size) to isolate library overhead.

### Binary Sections

- **.text**: Executable code (Flash)
- **.rodata**: Constants and string literals (Flash)
- **.data**: Initialized variables (Flash → RAM at startup)
- **.bss**: Zero-initialized variables (RAM only, no Flash cost)

**Total Flash** = .text + .rodata + .data
**Total RAM** = .data + .bss + stack

### Understanding Symbol Analysis

On non-FPU targets (Cortex-M0/M0+), **compiler_builtins symbols** like \`__divsf3\`, \`__addsf3\`, and \`__mulsf3\` indicate software f32 emulation overhead. These are absent on Cortex-M4F/M7 with hardware FPU.

**rgb_sequencer crate symbols** - Library code:
- \`RgbSequencer::service\` - Main sequencer logic
- \`SequenceBuilder::step\` - Sequence construction

**Your binary size** = Measured baseline + your implementations + your sequence data:
- \`RgbLed\` trait: PWM/SPI drivers
- \`TimeSource\` trait: Timer integration
- Additional sequencers and sequences

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
