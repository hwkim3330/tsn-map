#!/bin/bash
# TSN-Map launcher script
# Run with: ./tsn-map.sh [interface]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$SCRIPT_DIR/target/release/tsn-map"
INTERFACE="${1:-enp5s0}"

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "Binary not found. Building..."
    cd "$SCRIPT_DIR"
    cargo build --release
fi

# Check if capabilities are set
if ! getcap "$BINARY" | grep -q cap_net_raw; then
    echo "Setting network capabilities (requires sudo once)..."
    sudo setcap 'cap_net_raw,cap_net_admin+eip' "$BINARY"
fi

# Run without sudo
exec "$BINARY" -i "$INTERFACE" "$@"
