#!/bin/bash

# Integration Test Runner for P2P Go
# This script runs all integration tests with proper environment setup

set -e

echo "=== P2P Go Integration Test Suite ==="
echo

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Set environment variables
export RUST_LOG=info,libp2p=warn
export RUST_BACKTRACE=1

# Create test directories
echo "Setting up test environment..."
mkdir -p logs
mkdir -p test_data/sgf
mkdir -p test_data/models

# Check if SGF file exists
SGF_FILE="/Users/daniel/Downloads/76794817-078-worki-ve..sgf"
if [ ! -f "$SGF_FILE" ]; then
    echo -e "${RED}Error: SGF file not found at $SGF_FILE${NC}"
    echo "Please ensure the SGF file is available for testing."
    exit 1
fi

# Function to run a test module
run_test_module() {
    local module=$1
    local description=$2
    
    echo -e "${YELLOW}Testing: $description${NC}"
    
    if cargo test -p p2pgo-integration-tests --test $module -- --nocapture --test-threads=1; then
        echo -e "${GREEN}✓ $description passed${NC}"
        echo
    else
        echo -e "${RED}✗ $description failed${NC}"
        echo
        exit 1
    fi
}

# Run tests in order of dependency
echo "Running integration tests..."
echo

# Test 1: Network Discovery
run_test_module "discovery" "Network Discovery (mDNS, Direct, Relay)"

# Test 2: RNA Propagation
run_test_module "rna_propagation" "RNA Message Propagation"

# Test 3: SGF Training Data
run_test_module "sgf_training" "SGF Upload and Training Data Generation"

# Test 4: Neural Network Training
run_test_module "neural_training" "Neural Network Training Visualization"

# Test 5: Network Visualization
run_test_module "network_visualization" "Network Visualization Components"

# Summary
echo
echo -e "${GREEN}=== All Integration Tests Passed! ===${NC}"
echo
echo "Test logs available in: ./logs/"
echo "Test artifacts in: ./test_data/"

# Optional: Run specific test with verbose output
if [ "$1" = "--verbose" ] && [ -n "$2" ]; then
    echo
    echo "Running $2 with verbose output..."
    RUST_LOG=debug cargo test -p p2pgo-integration-tests $2 -- --nocapture
fi