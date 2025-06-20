#!/usr/bin/env bash
set -e

echo "🚀 Building p2pgo GUI..."
cargo build -p p2pgo-ui-egui --release

BIN="./target/release/p2pgo-ui-egui"

# Check if debug mode is enabled
DEBUG_FLAG=""
if [ "$DEBUG_GUI" = "1" ] || [ "$DEBUG_GUI" = "true" ]; then
    DEBUG_FLAG="--debug"
    echo "🐛 Debug mode enabled (set DEBUG_GUI=0 to disable)"
fi

echo "🎮 Launching two GUI windows for p2pgo..."

if command -v osascript &>/dev/null; then
    # macOS Terminal automation
    osascript <<OSA
tell application "Terminal"
    do script "cd $(pwd) && ${BIN} --player-name Alice ${DEBUG_FLAG}"
    delay 2
    do script "cd $(pwd) && ${BIN} --player-name Bob ${DEBUG_FLAG}" in (make new tab with properties {current tab:tab 1 of window 1})
end tell
OSA
    echo "✅ Two GUI windows opened via macOS Terminal"
elif command -v gnome-terminal &>/dev/null; then
    # Linux GNOME Terminal
    gnome-terminal --title="p2pgo Alice" -- bash -c "${BIN} --player-name Alice ${DEBUG_FLAG} ; read -p 'Press Enter to close...'"
    sleep 1
    gnome-terminal --title="p2pgo Bob" -- bash -c "${BIN} --player-name Bob ${DEBUG_FLAG} ; read -p 'Press Enter to close...'"
    echo "✅ Two GUI windows opened via gnome-terminal"
elif command -v xterm &>/dev/null; then
    # Fallback X11 terminal
    xterm -title "p2pgo Alice" -e bash -c "${BIN} --player-name Alice ${DEBUG_FLAG} ; read -p 'Press Enter to close...'" &
    sleep 1
    xterm -title "p2pgo Bob" -e bash -c "${BIN} --player-name Bob ${DEBUG_FLAG} ; read -p 'Press Enter to close...'" &
    echo "✅ Two GUI windows opened via xterm"
else
    echo "❌ No suitable terminal found. Please install Terminal.app (macOS), gnome-terminal, or xterm"
    echo "Manual run: ${BIN} --player-name Alice ${DEBUG_FLAG}"
    echo "Then in another terminal: ${BIN} --player-name Bob ${DEBUG_FLAG}"
    exit 1
fi

echo ""
echo "🎯 Instructions:"
echo "  1. In Alice's window: Create a new game"
echo "  2. In Bob's window: Join the game using the Game ID"
echo "  3. Play Go together!"
if [ -n "$DEBUG_FLAG" ]; then
    echo "  4. Press backtick (`) to toggle debug overlay"
    echo "  5. Check terminal output for verbose logging"
fi
echo ""
