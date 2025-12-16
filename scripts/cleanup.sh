#!/bin/bash

# cleanup.sh - Run cargo clean for all subprojects

set -e

echo "Cleaning rgb-sequencer workspace..."

# Root project
echo "  [1/9] Root project"
cargo clean

# Examples
echo "  [2/9] examples/stm32f0"
(cd examples/stm32f0 && cargo clean)

echo "  [3/9] examples/stm32f0-embassy"
(cd examples/stm32f0-embassy && cargo clean)

echo "  [4/9] examples/rp-pico"
(cd examples/rp-pico && cargo clean)

# Tools
echo "  [5/9] tools/sizeof-calculator"
(cd tools/sizeof-calculator && cargo clean)

echo "  [6/9] tools/binary-analyzer/minimal"
(cd tools/binary-analyzer/minimal && cargo clean)

echo "  [7/9] tools/benchmark/rp2040"
(cd tools/benchmark/rp2040 && cargo clean)

echo "  [8/9] tools/benchmark/rp2350"
(cd tools/benchmark/rp2350 && cargo clean)

# Remove tmp directory if it exists
if [ -d "tmp" ]; then
    echo "  [9/9] Removing tmp directory"
    rm -rf tmp
else
    echo "  [9/9] tmp directory (not present)"
fi

echo ""
echo "Cleanup complete!"
