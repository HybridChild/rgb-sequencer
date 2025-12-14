#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "Running RP2350 benchmark..."
echo "Make sure RP2350 hardware is connected and probe-rs is configured."
echo ""

OUTPUT_FILE="../rp2350_benchmarks.md"
TEMP_FILE=$(mktemp)
trap "rm -f $TEMP_FILE" EXIT

# Run cargo in background, capture to temp file
cargo run --release > "$TEMP_FILE" 2>&1 &
CARGO_PID=$!

# Wait for benchmark to complete
sleep 8

# Kill the process
kill $CARGO_PID 2>/dev/null || true
wait $CARGO_PID 2>/dev/null || true

# Generate output file, filtering out termination messages
{
    echo "# RP2350 Benchmark Results"
    echo ""
    echo "**Last Updated:** $(date '+%Y-%m-%d %H:%M:%S')  "
    echo "**Toolchain:** $(rustc --version)  "
    echo "**Target:** thumbv8m.main-none-eabihf (Cortex-M33, with FPU)  "
    echo "**Optimization:** --release"
    echo ""
    echo "## Results"
    echo ""
    echo '```'

    grep -v "Received SIGTERM" "$TEMP_FILE" | grep -v "Exited by user request" | grep -v "Benchmark complete"

    echo '```'
} > "$OUTPUT_FILE"

echo ""
echo "Results saved to $OUTPUT_FILE"
