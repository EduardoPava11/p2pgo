#!/usr/bin/env bash
set -e

# Build the CLI binary
cargo build -p p2pgo-cli --release
BIN=./target/release/p2pgo-cli

# Check if tmux is available
if ! command -v tmux &>/dev/null; then
    echo "Error: tmux not found. Please install tmux to use this script."
    exit 1
fi

# Create new tmux session with two panes
tmux new-session -d -s p2pgo-demo -x 120 -y 40

# Split horizontally
tmux split-window -h

# Left pane: Alice (host)
tmux send-keys -t 0 "${BIN} --role host" C-m

# Right pane: Bob (joins after 2 seconds)  
tmux send-keys -t 1 "sleep 2 && ${BIN} --role join" C-m

# Attach to session
tmux attach-session -t p2pgo-demo
