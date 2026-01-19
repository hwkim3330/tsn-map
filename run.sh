#!/bin/bash
# TSN-Map Launcher Script
# This script builds and runs the TSN-Map desktop application

cd "$(dirname "$0")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}       TSN-Map Desktop Application${NC}"
echo -e "${GREEN}========================================${NC}"

# Set up capabilities so we don't need root
setup_capabilities() {
    local binary="$1"
    if [ -f "$binary" ]; then
        if ! getcap "$binary" 2>/dev/null | grep -q cap_net_raw; then
            echo -e "${YELLOW}Setting network capabilities (one-time sudo required)...${NC}"
            if command -v pkexec &> /dev/null; then
                pkexec setcap 'cap_net_raw,cap_net_admin+eip' "$binary"
            else
                sudo setcap 'cap_net_raw,cap_net_admin+eip' "$binary"
            fi
        fi
    fi
}

# Kill any existing instance
pkill -f "tsn-map.*-p 8080" 2>/dev/null

# Auto-detect network interface
detect_interface() {
    # Try to find a real network interface (not lo, docker, veth, etc.)
    for iface in $(ls /sys/class/net/ 2>/dev/null); do
        case "$iface" in
            lo|docker*|veth*|br-*|virbr*)
                continue
                ;;
            *)
                if [ -d "/sys/class/net/$iface" ]; then
                    echo "$iface"
                    return
                fi
                ;;
        esac
    done
    echo "any"  # Fallback to capture all
}

INTERFACE="${1:-$(detect_interface)}"
echo -e "${GREEN}Using interface: ${INTERFACE}${NC}"

# Build backend if needed
if [ ! -f target/release/tsn-map ] || [ Cargo.toml -nt target/release/tsn-map ]; then
    echo -e "${YELLOW}Building backend...${NC}"
    cargo build --release 2>&1 | tail -5
fi

# Set up capabilities for packet capture without root
setup_capabilities "$(pwd)/target/release/tsn-map"

# Check if Tauri app exists
if [ ! -f src-tauri/target/release/tsn-map-app ]; then
    echo -e "${YELLOW}Building Tauri app...${NC}"
    cd src-tauri
    cargo build --release 2>&1 | tail -5
    cd ..
fi

# Start backend server in background
echo -e "${GREEN}Starting backend server on port 8080...${NC}"
./target/release/tsn-map -i "$INTERFACE" -p 8080 &
BACKEND_PID=$!

# Wait for server to start
sleep 1

# Check if server started
if ! kill -0 $BACKEND_PID 2>/dev/null; then
    echo -e "${RED}Failed to start backend server${NC}"
    exit 1
fi

echo -e "${GREEN}Backend server started (PID: $BACKEND_PID)${NC}"

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}Stopping backend server...${NC}"
    kill $BACKEND_PID 2>/dev/null
    echo -e "${GREEN}Done.${NC}"
}
trap cleanup EXIT

# Run Tauri app
echo -e "${GREEN}Starting TSN-Map application...${NC}"
./src-tauri/target/release/tsn-map-app

echo -e "${GREEN}Application closed.${NC}"
