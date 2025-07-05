#!/bin/bash
# Network diagnostics for P2P Go
# Helps debug connectivity issues

echo "🔍 P2P Go Network Diagnostics"
echo "============================"

# Check if p2pgo is running
check_process() {
    if pgrep -x "p2pgo" > /dev/null; then
        echo "✅ p2pgo is running"
        return 0
    else
        echo "❌ p2pgo is not running"
        return 1
    fi
}

# Check port availability
check_port() {
    local port=$1
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null ; then
        echo "✅ Port $port is listening"
    else
        echo "❌ Port $port is not listening"
    fi
}

# Check firewall (macOS)
check_firewall_macos() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        if sudo pfctl -s info 2>/dev/null | grep -q "Status: Enabled"; then
            echo "⚠️  macOS firewall is enabled"
            echo "   Make sure p2pgo is allowed in System Preferences > Security & Privacy"
        else
            echo "✅ macOS firewall is disabled"
        fi
    fi
}

# Check connectivity to bootstrap nodes
check_bootstrap() {
    echo "🌐 Checking bootstrap connectivity..."
    # Add actual bootstrap addresses when available
    local bootstraps=(
        "relay.example.com:4001"
        "bootstrap1.p2pgo.net:4001"
    )
    
    for bootstrap in "${bootstraps[@]}"; do
        if timeout 2 bash -c "</dev/tcp/${bootstrap%:*}/${bootstrap#*:}" 2>/dev/null; then
            echo "✅ Can reach $bootstrap"
        else
            echo "❌ Cannot reach $bootstrap"
        fi
    done
}

# NAT type detection
detect_nat() {
    echo "🔒 Detecting NAT type..."
    # Simple NAT detection based on local vs public IP
    LOCAL_IP=$(ifconfig | grep -Eo 'inet (addr:)?([0-9]*\.){3}[0-9]*' | grep -Eo '([0-9]*\.){3}[0-9]*' | grep -v '127.0.0.1' | head -1)
    PUBLIC_IP=$(curl -s ifconfig.me 2>/dev/null || echo "unknown")
    
    if [[ "$LOCAL_IP" == "$PUBLIC_IP" ]]; then
        echo "✅ No NAT detected (public IP)"
    else
        echo "⚠️  Behind NAT"
        echo "   Local IP:  $LOCAL_IP"
        echo "   Public IP: $PUBLIC_IP"
        echo "   Relay mode recommended"
    fi
}

# Check mDNS (for local discovery)
check_mdns() {
    if command -v avahi-browse &> /dev/null; then
        echo "🔍 Checking mDNS/Bonjour..."
        timeout 2 avahi-browse -t _p2pgo._tcp 2>/dev/null || echo "No local p2pgo nodes found via mDNS"
    fi
}

# Performance check
check_performance() {
    echo "⚡ Quick performance check..."
    
    # Check CPU usage
    if command -v top &> /dev/null; then
        CPU=$(top -l 1 | grep "CPU usage" | awk '{print $3}' | sed 's/%//')
        if (( $(echo "$CPU < 80" | bc -l) )); then
            echo "✅ CPU usage OK: ${CPU}%"
        else
            echo "⚠️  High CPU usage: ${CPU}%"
        fi
    fi
    
    # Check memory
    if [[ "$OSTYPE" == "darwin"* ]]; then
        MEM=$(top -l 1 | grep PhysMem | awk '{print $2}' | sed 's/M//')
        echo "   Memory used: ${MEM}MB"
    fi
}

# Main diagnostics
echo ""
echo "1️⃣  Process Status"
echo "-------------------"
check_process

echo ""
echo "2️⃣  Network Ports"
echo "-----------------"
for port in 4001 4002 4003; do
    check_port $port
done

echo ""
echo "3️⃣  Firewall Status"
echo "-------------------"
check_firewall_macos

echo ""
echo "4️⃣  NAT Detection"
echo "-----------------"
detect_nat

echo ""
echo "5️⃣  Bootstrap Nodes"
echo "-------------------"
check_bootstrap

echo ""
echo "6️⃣  Local Discovery"
echo "-------------------"
check_mdns

echo ""
echo "7️⃣  Performance"
echo "---------------"
check_performance

echo ""
echo "📊 Diagnostics complete!"
echo ""
echo "💡 Troubleshooting tips:"
echo "- If behind NAT, use Minimal or Normal relay mode"
echo "- Ensure ports 4001-4003 are not blocked"
echo "- Check firewall allows p2pgo"
echo "- For local testing, ensure nodes are on same network"