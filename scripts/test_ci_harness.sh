#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0

# CI test harness script for running relay integration tests
# This script is designed to work both locally and in CI environments

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;36m'
RESET='\033[0m'

echo -e "${BLUE}===== P2P Go CI Test Harness =====${RESET}"

# Set up log directory
LOG_DIR="target/relay_logs"
mkdir -p "$LOG_DIR"

# Export CI=true to enable skipping of real network tests
export CI=true

# Check if we can reach the default relay
echo -e "${YELLOW}Checking relay connectivity...${RESET}"
RELAY_REACHABLE=0

if ping -c 1 -W 2 relay.iroh.network &> /dev/null; then
    echo -e "${GREEN}✓ relay.iroh.network is reachable${RESET}"
    RELAY_REACHABLE=1
else
    echo -e "${YELLOW}⚠ relay.iroh.network is not reachable, will skip network tests${RESET}"
fi

# Run core tests first (these don't depend on network)
echo -e "${BLUE}Running core tests...${RESET}"
cargo test --package p2pgo_core --all-features || { 
    echo -e "${RED}❌ Core tests failed${RESET}"; 
    exit 1; 
}
echo -e "${GREEN}✓ Core tests passed${RESET}"

# Run trainer tests
echo -e "${BLUE}Running trainer tests...${RESET}"
cargo test --package p2pgo_trainer --all-features || { 
    echo -e "${RED}❌ Trainer tests failed${RESET}"; 
    exit 1; 
}
echo -e "${GREEN}✓ Trainer tests passed${RESET}"

# Run CLI tests
echo -e "${BLUE}Running CLI tests...${RESET}"
cargo test --package p2pgo_cli --all-features || { 
    echo -e "${RED}❌ CLI tests failed${RESET}"; 
    exit 1; 
}
echo -e "${GREEN}✓ CLI tests passed${RESET}"

# Run UI tests
echo -e "${BLUE}Running UI tests...${RESET}"
cargo test --package p2pgo_ui_egui --all-features || { 
    echo -e "${RED}❌ UI tests failed${RESET}"; 
    exit 1; 
}
echo -e "${GREEN}✓ UI tests passed${RESET}"

# Run network tests with or without external connectivity
echo -e "${BLUE}Running network tests...${RESET}"

# If relay is reachable, run all tests
if [ $RELAY_REACHABLE -eq 1 ]; then
    echo -e "${BLUE}Running all network tests including external relay tests...${RESET}"
    # Unset CI so external relay tests run
    unset CI
    cargo test --package p2pgo_network --all-features -- --nocapture 2>&1 | tee "$LOG_DIR/network_tests.log" || { 
        echo -e "${RED}❌ Network tests failed${RESET}"; 
        exit 1; 
    }
else
    echo -e "${YELLOW}Running network tests with external relay tests skipped...${RESET}"
    # CI remains true to skip external relay tests
    cargo test --package p2pgo_network --all-features -- --nocapture 2>&1 | tee "$LOG_DIR/network_tests.log" || { 
        echo -e "${RED}❌ Network tests failed${RESET}"; 
        exit 1; 
    }
fi
echo -e "${GREEN}✓ Network tests passed${RESET}"

# Run embedded relay tests separately (should always work, even in CI)
echo -e "${BLUE}Running embedded relay tests...${RESET}"
cargo test --package p2pgo_network --test spawn_cancelable --test embedded_relay --all-features -- --nocapture 2>&1 | tee "$LOG_DIR/embedded_relay_tests.log" || {
    echo -e "${RED}❌ Embedded relay tests failed${RESET}";
    exit 1;
}
echo -e "${GREEN}✓ Embedded relay tests passed${RESET}"

# Run clippy (with warnings as errors)
echo -e "${BLUE}Running clippy checks...${RESET}"
cargo clippy --all-features -- -D warnings || {
    echo -e "${RED}❌ Clippy checks failed${RESET}";
    exit 1;
}
echo -e "${GREEN}✓ Clippy checks passed${RESET}"

# Try build to ensure everything compiles
echo -e "${BLUE}Building release version...${RESET}"
cargo build --release --all-features || {
    echo -e "${RED}❌ Build failed${RESET}";
    exit 1;
}
echo -e "${GREEN}✓ Release build successful${RESET}"

# All tests passed
echo -e "${GREEN}=======================================${RESET}"
echo -e "${GREEN}✓✓✓ All tests passed successfully ✓✓✓${RESET}"
echo -e "${GREEN}=======================================${RESET}"

exit 0
