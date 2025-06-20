# P2P Go Makefile
# MVP networking and test automation

.PHONY: help build test test-network test-ui check clean clippy fmt
.DEFAULT_GOAL := help

help: ## Show this help message
	@echo "P2P Go - Go Game with P2P Networking"
	@echo "Usage:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build all packages
	cargo build --all

check: ## Check all packages for compilation errors
	cargo check --all

test: ## Run all tests (without networking features)
	cargo test --all

test-network: ## Run network tests with iroh features enabled (headless, single-threaded)
	cargo test --workspace --features "iroh,headless" -- --test-threads=1

test-ui: ## Run UI tests
	cargo test -p p2pgo-ui-egui

test-core: ## Run core Go game logic tests
	cargo test -p p2pgo-core

test-first-stone: ## Run the key MVP test: first stone sync
	cargo test --features iroh --test first_stone_sync first_stone_appears_on_both_boards

test-gossip: ## Run gossip networking tests
	cargo test --features iroh --test iroh_gossip_game_advertisement

clippy: ## Run Clippy linter
	cargo clippy --all -- -D warnings

clippy-network: ## Run Clippy on network package with iroh features
	cargo clippy --features iroh -p p2pgo-network -- -D warnings

fmt: ## Format code
	cargo fmt --all

clean: ## Clean build artifacts
	cargo clean

# Development helpers
dev-check: check clippy fmt ## Run quick development checks

# CI targets
ci-test: test test-network ## Run all tests for CI

# Quick MVP test
mvp-test: test-first-stone ## Test the core MVP functionality
