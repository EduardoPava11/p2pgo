#!/bin/bash
# Test P2P gameplay with activity logging

set -e

echo "P2P Go - Gameplay Testing Script"
echo "================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

# Check if running on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    print_error "This script is designed for macOS"
    exit 1
fi

# Function to build and create DMG
build_dmg() {
    print_status "Building P2P Go..."
    
    # Clean previous builds
    rm -rf target/release/bundle
    
    # Build release with optimizations
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    
    # Bundle the app
    print_status "Creating app bundle..."
    cargo bundle --release
    
    # Create DMG
    print_status "Creating DMG..."
    ./scripts/build_universal_dmg.sh
    
    if [ -f "P2P Go.dmg" ]; then
        print_status "DMG created successfully: P2P Go.dmg"
        return 0
    else
        print_error "Failed to create DMG"
        return 1
    fi
}

# Function to install DMG on test machine
install_dmg() {
    local dmg_path="$1"
    local target_machine="$2"
    
    if [ -z "$target_machine" ] || [ "$target_machine" == "local" ]; then
        print_status "Installing locally..."
        
        # Mount DMG
        hdiutil attach "$dmg_path" -nobrowse -quiet
        
        # Copy to Applications
        cp -R "/Volumes/P2P Go/P2P Go.app" /Applications/
        
        # Unmount
        hdiutil detach "/Volumes/P2P Go" -quiet
        
        print_status "Installed to /Applications/P2P Go.app"
    else
        print_status "Copying DMG to $target_machine..."
        scp "$dmg_path" "$target_machine:~/Downloads/"
        
        print_status "Installing on $target_machine..."
        ssh "$target_machine" << 'EOF'
            hdiutil attach ~/Downloads/P2P\ Go.dmg -nobrowse -quiet
            cp -R "/Volumes/P2P Go/P2P Go.app" /Applications/
            hdiutil detach "/Volumes/P2P Go" -quiet
            echo "Installation complete"
EOF
    fi
}

# Function to start the app with logging
start_with_logging() {
    local log_dir="$HOME/Library/Application Support/p2pgo/game_logs"
    mkdir -p "$log_dir"
    
    print_status "Starting P2P Go with activity logging..."
    print_status "Logs will be saved to: $log_dir"
    
    # Start the app with console logging enabled
    RUST_LOG=debug,p2pgo=trace /Applications/P2P\ Go.app/Contents/MacOS/p2pgo-ui-egui \
        --console-log \
        --activity-log \
        2>&1 | tee "$log_dir/console_$(date +%Y%m%d_%H%M%S).log" &
    
    local app_pid=$!
    print_status "P2P Go started with PID: $app_pid"
    
    # Start log monitor in another terminal
    if command -v osascript &> /dev/null; then
        osascript -e "tell app \"Terminal\" to do script \"tail -f '$log_dir'/game_activity_*.jsonl | jq -r '\\\"[\\\" + .timestamp + \\\"] \\\" + .entry_type.type + \\\": \\\" + (.data | tostring)'\""
    fi
    
    return $app_pid
}

# Function to monitor game activity
monitor_activity() {
    local log_dir="$HOME/Library/Application Support/p2pgo/game_logs"
    local latest_log=$(ls -t "$log_dir"/game_activity_*.jsonl 2>/dev/null | head -1)
    
    if [ -z "$latest_log" ]; then
        print_warning "No activity log found"
        return
    fi
    
    print_status "Monitoring game activity from: $latest_log"
    echo ""
    echo "Game Activity Summary:"
    echo "====================="
    
    # Parse and display activity
    tail -f "$latest_log" | while read line; do
        if echo "$line" | jq -e '.entry_type.type == "MoveMade"' > /dev/null 2>&1; then
            local move_info=$(echo "$line" | jq -r '.entry_type.move_data | "Move \(.move_number): \(.move_type) at \(.coord)"')
            echo "  🎯 $move_info"
        elif echo "$line" | jq -e '.entry_type.type == "NetworkOp"' > /dev/null 2>&1; then
            local net_info=$(echo "$line" | jq -r '.entry_type.operation.op_type')
            echo "  🌐 Network: $net_info"
        elif echo "$line" | jq -e '.entry_type.type == "Error"' > /dev/null 2>&1; then
            local error_info=$(echo "$line" | jq -r '.entry_type.error')
            echo "  ❌ Error: $error_info"
        fi
    done
}

# Function to test relay connectivity
test_relay() {
    print_status "Testing relay connectivity..."
    
    # Check if relay is reachable
    local relay_status=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/health || echo "000")
    
    if [ "$relay_status" == "200" ]; then
        print_status "Local relay is healthy"
    else
        print_warning "Local relay not responding, starting..."
        ./scripts/start_relay.sh &
        sleep 5
    fi
}

# Function to create test game
create_test_game() {
    print_status "Creating test game..."
    
    # Use curl to create game via API (if implemented)
    # For now, we'll need to use the UI
    
    print_warning "Please create a game using the UI"
    print_warning "Game code will appear in the activity log"
}

# Main menu
show_menu() {
    echo ""
    echo "P2P Go Testing Options:"
    echo "1) Build and create DMG"
    echo "2) Install DMG locally"
    echo "3) Install DMG on remote Mac"
    echo "4) Start with activity logging"
    echo "5) Monitor game activity"
    echo "6) Test relay connectivity"
    echo "7) Run full test suite"
    echo "8) View logs"
    echo "9) Exit"
    echo ""
    read -p "Select option: " choice
    
    case $choice in
        1)
            build_dmg
            ;;
        2)
            if [ -f "P2P Go.dmg" ]; then
                install_dmg "P2P Go.dmg" "local"
            else
                print_error "DMG not found. Build it first."
            fi
            ;;
        3)
            read -p "Enter target machine (user@host): " target
            if [ -f "P2P Go.dmg" ]; then
                install_dmg "P2P Go.dmg" "$target"
            else
                print_error "DMG not found. Build it first."
            fi
            ;;
        4)
            start_with_logging
            ;;
        5)
            monitor_activity
            ;;
        6)
            test_relay
            ;;
        7)
            print_status "Running full test suite..."
            build_dmg
            install_dmg "P2P Go.dmg" "local"
            test_relay
            start_with_logging
            create_test_game
            ;;
        8)
            log_dir="$HOME/Library/Application Support/p2pgo/game_logs"
            echo "Available logs:"
            ls -la "$log_dir"/*.jsonl 2>/dev/null || echo "No logs found"
            ;;
        9)
            exit 0
            ;;
        *)
            print_error "Invalid option"
            ;;
    esac
}

# Check dependencies
check_dependencies() {
    local missing=()
    
    command -v jq >/dev/null 2>&1 || missing+=("jq")
    command -v cargo >/dev/null 2>&1 || missing+=("rust/cargo")
    
    if [ ${#missing[@]} -ne 0 ]; then
        print_error "Missing dependencies: ${missing[*]}"
        print_warning "Install with: brew install ${missing[*]}"
        exit 1
    fi
}

# Main
main() {
    check_dependencies
    
    # Show banner
    cat << "EOF"
    ____  ___   ____    ______      
   / __ \|__ \ / __ \  / ____/___   
  / /_/ /__/ // /_/ / / / __/ __ \  
 / ____// __// ____/ / /_/ / /_/ /  
/_/    /____/_/      \____/\____/   
                                    
EOF
    
    while true; do
        show_menu
    done
}

# Run main
main "$@"