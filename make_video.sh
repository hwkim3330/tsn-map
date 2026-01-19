#!/bin/bash
# TSN-Map Professional Demo Video Creator
# Uses Edge TTS for high-quality English narration

set -e
echo "=========================================="
echo "TSN-Map Demo Video Creator"
echo "=========================================="

# Install edge-tts if not installed
if ! command -v edge-tts &> /dev/null; then
    echo "[1/5] Installing edge-tts..."
    pip3 install --break-system-packages edge-tts
else
    echo "[1/5] edge-tts already installed"
fi

# Setup
cd /home/kim/tsn-map
mkdir -p pic/video_work
cd pic/video_work
ICON="/home/kim/tsn-map/src-tauri/icons/icon.png"

echo "[2/5] Copying screenshots..."
cp "../Screenshot from 2026-01-19 15-31-13.png" s01_interface.png
cp "../Screenshot from 2026-01-19 15-16-01.png" s02_topology.png
cp "../Screenshot from 2026-01-19 15-32-57.png" s03_filter.png
cp "../Screenshot from 2026-01-19 15-33-05.png" s04_stats.png
cp "../Screenshot from 2026-01-19 15-33-10.png" s05_hosts.png
cp "../Screenshot from 2026-01-19 15-32-37.png" s06_detail.png
cp "../Screenshot from 2026-01-19 15-17-14.png" s07_detail2.png
cp "../Screenshot from 2026-01-19 15-18-14.png" s08_tester.png
cp "../Screenshot from 2026-01-19 15-18-44.png" s09_large.png
echo "  Screenshots copied"

echo "[3/5] Generating TTS narration with Edge TTS..."
VOICE="en-US-AriaNeural"

edge-tts --voice $VOICE --text "Welcome to TSN-Map. A real-time network topology visualization tool. Built with Rust and D3.js. Developed by KETI, Korea Electronics Technology Institute." --write-media a00_title.mp3
echo "  Title narration done"

edge-tts --voice $VOICE --text "Let's look at the system architecture. The backend is written in Rust using the Axum web framework. Packet capture is handled by libpcap for high-performance network monitoring. The frontend uses D3.js for interactive topology visualization and Chart.js for statistics. Real-time data flows through Server-Sent Events." --write-media a01_arch.mp3
echo "  Architecture narration done"

edge-tts --voice $VOICE --text "When you start TSN-Map, select your network interface. You can choose from physical network cards, the loopback interface, or virtual interfaces like Docker." --write-media a02_interface.mp3

edge-tts --voice $VOICE --text "The topology view automatically discovers network nodes from captured traffic. Nodes are arranged using a force-directed graph layout. You can drag nodes, zoom in and out, and click on any node to see detailed information." --write-media a03_topology.mp3

edge-tts --voice $VOICE --text "Use the filter bar to focus on specific traffic. Filter by IP address, protocol type, or any custom criteria. The topology and packet list update in real-time to show only matching connections." --write-media a04_filter.mp3

edge-tts --voice $VOICE --text "The statistics dashboard provides comprehensive traffic analysis. View protocol distribution as a pie chart, real-time traffic rate over time, top ten conversations by traffic volume, and packet size distribution histogram." --write-media a05_stats.mp3

edge-tts --voice $VOICE --text "The hosts view lists all discovered network devices. Each entry shows the MAC address, IP address, vendor name from OUI database lookup, total packet count, protocols in use, and active port numbers." --write-media a06_hosts.mp3

edge-tts --voice $VOICE --text "Click any packet for deep inspection. The detail panel shows a complete layer-by-layer breakdown. Frame information, Ethernet header with MAC addresses, IP header with addresses and TTL, transport layer with ports and flags, and the raw hexadecimal data dump." --write-media a07_detail.mp3

edge-tts --voice $VOICE --text "TSN-Map includes network testing tools. The packet generator can send UDP traffic at configurable packet rates. Monitor real-time throughput on the chart. This is useful for network performance testing and troubleshooting." --write-media a08_tester.mp3

edge-tts --voice $VOICE --text "TSN-Map efficiently handles large-scale networks. This example shows over one hundred nodes with more than two hundred links. The D3.js rendering is optimized for smooth interaction even with complex network topologies." --write-media a09_large.mp3

edge-tts --voice $VOICE --text "Thank you for watching this demonstration of TSN-Map. The project is open source and available on GitHub. Built with Rust, Axum, libpcap, D3.js, and Chart.js. Developed by KETI, Korea Electronics Technology Institute." --write-media a10_closing.mp3
echo "  All narration generated"

echo "[4/5] Creating video frames..."

# Title frame with logo
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" -i "$ICON" \
  -filter_complex "[1:v]scale=160:160[logo];[0:v][logo]overlay=(W-w)/2:(H-h)/2-160[bg]; \
    [bg]drawtext=text='TSN-Map':fontsize=96:fontcolor=white:x=(w-text_w)/2:y=(h/2)+40, \
    drawtext=text='Real-time Network Topology Visualization':fontsize=36:fontcolor=#8b949e:x=(w-text_w)/2:y=(h/2)+140, \
    drawtext=text='Rust + D3.js':fontsize=28:fontcolor=#58a6ff:x=(w-text_w)/2:y=(h/2)+200, \
    drawtext=text='KETI':fontsize=24:fontcolor=#6e7681:x=(w-text_w)/2:y=h-60" \
  -frames:v 1 f00_title.png 2>/dev/null

# Architecture frame
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" \
  -vf "drawtext=text='System Architecture':fontsize=56:fontcolor=white:x=(w-text_w)/2:y=50, \
    drawbox=x=100:y=140:w=800:h=320:color=#161b22:t=fill, \
    drawtext=text='Backend (Rust)':fontsize=32:fontcolor=#58a6ff:x=140:y=160, \
    drawtext=text='• Axum Web Framework':fontsize=22:fontcolor=#c9d1d9:x=160:y=210, \
    drawtext=text='• libpcap Packet Capture':fontsize=22:fontcolor=#c9d1d9:x=160:y=250, \
    drawtext=text='• SSE Real-time Streaming':fontsize=22:fontcolor=#c9d1d9:x=160:y=290, \
    drawtext=text='• Protocol Parsers':fontsize=22:fontcolor=#c9d1d9:x=160:y=330, \
    drawtext=text='• Topology Graph Engine':fontsize=22:fontcolor=#c9d1d9:x=160:y=370, \
    drawbox=x=1020:y=140:w=800:h=320:color=#161b22:t=fill, \
    drawtext=text='Frontend (Web)':fontsize=32:fontcolor=#f0883e:x=1060:y=160, \
    drawtext=text='• D3.js Force Graph':fontsize=22:fontcolor=#c9d1d9:x=1080:y=210, \
    drawtext=text='• Chart.js Statistics':fontsize=22:fontcolor=#c9d1d9:x=1080:y=250, \
    drawtext=text='• EventSource API (SSE)':fontsize=22:fontcolor=#c9d1d9:x=1080:y=290, \
    drawtext=text='• Responsive Dark UI':fontsize=22:fontcolor=#c9d1d9:x=1080:y=330, \
    drawtext=text='• Interactive Controls':fontsize=22:fontcolor=#c9d1d9:x=1080:y=370, \
    drawtext=text='───────────────▶':fontsize=40:fontcolor=#7ee787:x=920:y=280, \
    drawbox=x=100:y=500:w=1720:h=140:color=#161b22:t=fill, \
    drawtext=text='Supported Protocols':fontsize=28:fontcolor=#7ee787:x=140:y=520, \
    drawtext=text='Ethernet • IPv4/IPv6 • TCP/UDP • ARP • ICMP • LLDP • VLAN (802.1Q) • PTP (1588) • STP • LACP':fontsize=20:fontcolor=#c9d1d9:x=140:y=570, \
    drawbox=x=100:y=680:w=1720:h=100:color=#161b22:t=fill, \
    drawtext=text='Data Flow\: Network → Capture → Parse → Build Topology → SSE Stream → Web Visualization':fontsize=22:fontcolor=#58a6ff:x=140:y=720" \
  -frames:v 1 f01_arch.png 2>/dev/null
echo "  Title and architecture frames done"

# Screenshot frames with headers
create_frame() {
    local img=$1
    local output=$2
    local title=$3
    local desc=$4

    ffmpeg -y -i "$img" \
      -vf "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=#0d1117, \
        drawbox=y=0:w=iw:h=80:color=#0d1117@0.95:t=fill, \
        drawtext=text='$title':fontsize=36:fontcolor=white:x=40:y=22, \
        drawbox=y=ih-70:w=iw:h=70:color=#0d1117@0.95:t=fill, \
        drawtext=text='$desc':fontsize=22:fontcolor=#8b949e:x=40:y=h-48" \
      -frames:v 1 "$output" 2>/dev/null
}

create_frame s01_interface.png f02_interface.png "Interface Selection" "Choose from physical NICs, loopback, or virtual interfaces"
create_frame s02_topology.png f03_topology.png "Network Topology" "Auto-discovery with force-directed graph layout"
create_frame s03_filter.png f04_filter.png "Packet Filtering" "Filter by IP, protocol, or custom criteria"
create_frame s04_stats.png f05_stats.png "Statistics Dashboard" "Protocol distribution, traffic rate, conversations, packet sizes"
create_frame s05_hosts.png f06_hosts.png "Host Discovery" "MAC, IP, vendor lookup, packet counts, protocols, ports"
create_frame s06_detail.png f07_detail.png "Deep Packet Inspection" "Layer-by-layer analysis with hex dump"
create_frame s08_tester.png f08_tester.png "Packet Generator" "UDP traffic generation for performance testing"
create_frame s09_large.png f09_large.png "Large Scale Support" "Efficiently handles 100+ nodes"
echo "  Screenshot frames done"

# Closing frame
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" -i "$ICON" \
  -filter_complex "[1:v]scale=120:120[logo];[0:v][logo]overlay=(W-w)/2:(H-h)/2-160[bg]; \
    [bg]drawtext=text='TSN-Map':fontsize=72:fontcolor=white:x=(w-text_w)/2:y=(h/2), \
    drawtext=text='Open Source Network Visualization':fontsize=32:fontcolor=#8b949e:x=(w-text_w)/2:y=(h/2)+80, \
    drawtext=text='github.com/keti/tsn-map':fontsize=28:fontcolor=#58a6ff:x=(w-text_w)/2:y=(h/2)+140, \
    drawtext=text='Rust • Axum • libpcap • D3.js • Chart.js':fontsize=22:fontcolor=#7ee787:x=(w-text_w)/2:y=(h/2)+200, \
    drawtext=text='KETI - Korea Electronics Technology Institute':fontsize=20:fontcolor=#6e7681:x=(w-text_w)/2:y=h-60" \
  -frames:v 1 f10_closing.png 2>/dev/null
echo "  Closing frame done"

echo "[5/5] Creating video segments and combining..."

# Create video segments (image + audio)
create_segment() {
    local frame=$1
    local audio=$2
    local output=$3

    ffmpeg -y -loop 1 -i "$frame" -i "$audio" \
      -c:v libx264 -tune stillimage -preset fast -crf 22 \
      -c:a aac -b:a 192k \
      -shortest -pix_fmt yuv420p -r 30 \
      "$output" 2>/dev/null
}

create_segment f00_title.png a00_title.mp3 v00.mp4
echo "  Segment 0/10 done"
create_segment f01_arch.png a01_arch.mp3 v01.mp4
echo "  Segment 1/10 done"
create_segment f02_interface.png a02_interface.mp3 v02.mp4
echo "  Segment 2/10 done"
create_segment f03_topology.png a03_topology.mp3 v03.mp4
echo "  Segment 3/10 done"
create_segment f04_filter.png a04_filter.mp3 v04.mp4
echo "  Segment 4/10 done"
create_segment f05_stats.png a05_stats.mp3 v05.mp4
echo "  Segment 5/10 done"
create_segment f06_hosts.png a06_hosts.mp3 v06.mp4
echo "  Segment 6/10 done"
create_segment f07_detail.png a07_detail.mp3 v07.mp4
echo "  Segment 7/10 done"
create_segment f08_tester.png a08_tester.mp3 v08.mp4
echo "  Segment 8/10 done"
create_segment f09_large.png a09_large.mp3 v09.mp4
echo "  Segment 9/10 done"
create_segment f10_closing.png a10_closing.mp3 v10.mp4
echo "  Segment 10/10 done"

# Create concat list
cat > concat.txt << 'EOF'
file 'v00.mp4'
file 'v01.mp4'
file 'v02.mp4'
file 'v03.mp4'
file 'v04.mp4'
file 'v05.mp4'
file 'v06.mp4'
file 'v07.mp4'
file 'v08.mp4'
file 'v09.mp4'
file 'v10.mp4'
EOF

# Combine all segments
ffmpeg -y -f concat -safe 0 -i concat.txt \
  -c:v libx264 -preset medium -crf 20 \
  -c:a aac -b:a 192k \
  /home/kim/tsn-map/tsn-map-demo-full.mp4 2>/dev/null

echo ""
echo "=========================================="
echo "Video created successfully!"
echo "=========================================="
ls -lh /home/kim/tsn-map/tsn-map-demo-full.mp4
echo ""
echo "Duration:"
ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 /home/kim/tsn-map/tsn-map-demo-full.mp4 | xargs printf "%.1f seconds\n"

# Cleanup
echo ""
echo "Cleaning up temporary files..."
cd /home/kim/tsn-map
rm -rf pic/video_work

echo "Done!"
