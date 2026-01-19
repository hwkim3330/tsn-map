#!/bin/bash
# TSN-Map Web Server Launcher
# Runs backend only, access via browser at http://localhost:8080

cd "$(dirname "$0")"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Kill any existing instance
pkill -f "tsn-map.*-p 8080" 2>/dev/null
sleep 1

# Auto-detect interface
INTERFACE="${1:-enp5s0}"
for iface in enp5s0 enp11s0 eth0 eno1; do
    if [ -d "/sys/class/net/$iface" ]; then
        INTERFACE="$iface"
        break
    fi
done

echo -e "${GREEN}TSN-Map Web Server${NC}"
echo -e "${GREEN}Interface: ${INTERFACE}${NC}"
echo -e "${GREEN}URL: http://localhost:8080${NC}"
echo ""

# Build if needed
if [ ! -f target/release/tsn-map ]; then
    echo -e "${YELLOW}Building...${NC}"
    cargo build --release 2>&1 | tail -3
fi

# Set capabilities if not set
BINARY="$(pwd)/target/release/tsn-map"
if ! getcap "$BINARY" 2>/dev/null | grep -q cap_net_raw; then
    echo -e "${YELLOW}Setting capabilities (one-time sudo)...${NC}"
    if command -v pkexec &> /dev/null; then
        pkexec setcap 'cap_net_raw,cap_net_admin+eip' "$BINARY"
    else
        sudo setcap 'cap_net_raw,cap_net_admin+eip' "$BINARY"
    fi
fi

# Open browser
(sleep 2 && xdg-open http://localhost:8080 2>/dev/null) &

# Run server (foreground)
exec "$BINARY" -i "$INTERFACE" -p 8080
