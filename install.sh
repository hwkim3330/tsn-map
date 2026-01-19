#!/bin/bash
# Install TSN-Map desktop shortcut

cd "$(dirname "$0")"

echo "Building TSN-Map..."
cargo build --release

echo "Building Tauri app..."
cd src-tauri
cargo build --release
cd ..

echo "Installing desktop shortcut..."
cp tsn-map.desktop ~/.local/share/applications/

echo ""
echo "============================================"
echo "  TSN-Map installed successfully!"
echo "============================================"
echo ""
echo "Run options:"
echo "  1. From terminal:  ./run.sh"
echo "  2. With sudo:      sudo ./run.sh"
echo "  3. From app menu:  Search 'TSN-Map'"
echo ""
echo "Note: sudo is required for packet capture"
echo ""
