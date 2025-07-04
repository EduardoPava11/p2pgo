#!/bin/bash

# Script to analyze Ko situations in SGF files

set -e

echo "=== Ko Analyzer for P2P Go ==="
echo

# Check if SGF file exists
SGF_FILE="/Users/daniel/Downloads/76794817-078-worki-ve..sgf"
if [ ! -f "$SGF_FILE" ]; then
    echo "Error: SGF file not found at $SGF_FILE"
    exit 1
fi

# Create output directory
OUTPUT_DIR="./ko_mrna_output"
mkdir -p "$OUTPUT_DIR"

echo "Building Ko analyzer tool..."
cargo build --release --bin ko-analyzer

echo
echo "Analyzing SGF file for Ko situations..."
echo "SGF: $SGF_FILE"
echo "Output: $OUTPUT_DIR"
echo

# Run the analyzer
RUST_LOG=info cargo run --release --bin ko-analyzer -- \
    --sgf "$SGF_FILE" \
    --output "$OUTPUT_DIR" \
    --context-before 10 \
    --context-after 10 \
    --verbose

echo
echo "Analysis complete. Check $OUTPUT_DIR for generated mRNA CBOR files."

# List generated files
echo
echo "Generated files:"
ls -la "$OUTPUT_DIR"/*.cbor 2>/dev/null || echo "No CBOR files generated (no Ko situations found)"

# If CBOR files were generated, show their sizes
if ls "$OUTPUT_DIR"/*.cbor 1> /dev/null 2>&1; then
    echo
    echo "CBOR file details:"
    for cbor in "$OUTPUT_DIR"/*.cbor; do
        size=$(stat -f%z "$cbor" 2>/dev/null || stat -c%s "$cbor" 2>/dev/null)
        echo "  $(basename "$cbor"): $size bytes"
    done
fi