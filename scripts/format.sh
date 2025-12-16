#!/bin/bash

# format.sh - Run cargo fmt for all subprojects

set -e

echo "Formatting rgb-sequencer workspace..."

# Root project
echo "  [1/8] Root project"
cargo fmt

# Examples
echo "  [2/8] examples/stm32f0"
(cd examples/stm32f0 && cargo fmt)

echo "  [3/8] examples/stm32f0-embassy"
(cd examples/stm32f0-embassy && cargo fmt)

echo "  [4/8] examples/rp-pico"
(cd examples/rp-pico && cargo fmt)

# Tools
echo "  [5/8] tools/sizeof-calculator"
(cd tools/sizeof-calculator && cargo fmt)

echo "  [6/8] tools/binary-analyzer/minimal"
(cd tools/binary-analyzer/minimal && cargo fmt)

echo "  [7/8] tools/benchmark/rp2040"
(cd tools/benchmark/rp2040 && cargo fmt)

echo "  [8/8] tools/benchmark/rp2350"
(cd tools/benchmark/rp2350 && cargo fmt)

echo ""
echo "Format complete!"
