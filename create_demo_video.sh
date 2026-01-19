#!/bin/bash
set -e

cd /home/kim/tsn-map
mkdir -p pic/video_work
cd pic/video_work

# Copy screenshots
cp "../Screenshot from 2026-01-19 15-16-01.png" slide1_topology.png
cp "../Screenshot from 2026-01-19 15-16-46.png" slide2_stats.png
cp "../Screenshot from 2026-01-19 15-16-58.png" slide3_hosts.png
cp "../Screenshot from 2026-01-19 15-17-14.png" slide4_detail1.png
cp "../Screenshot from 2026-01-19 15-17-18.png" slide5_detail2.png
cp "../Screenshot from 2026-01-19 15-18-14.png" slide6_tester.png
cp "../Screenshot from 2026-01-19 15-18-44.png" slide7_large.png

# Copy logos
cp /home/kim/tsn-map/src-tauri/icons/icon.png app_icon.png
cp /home/kim/tsn-map/web/img/keti-logo.png keti_logo.png 2>/dev/null || true

WIDTH=1920
HEIGHT=1080
FONTSIZE=36
TITLESIZE=72
SUBTITLESIZE=48

# ========================================
# SLIDE 0: Title Card with Logo
# ========================================
echo "Creating title slide..."
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=${WIDTH}x${HEIGHT}:d=1" \
  -i app_icon.png \
  -filter_complex " \
    [1:v]scale=200:200[logo]; \
    [0:v][logo]overlay=(W-w)/2:(H-h)/2-150[bg]; \
    [bg]drawtext=text='TSN-Map':fontsize=${TITLESIZE}:fontcolor=white:x=(w-text_w)/2:y=(h/2)+80, \
    drawtext=text='Real-time Network Topology Visualization':fontsize=${SUBTITLESIZE}:fontcolor=#8b949e:x=(w-text_w)/2:y=(h/2)+160, \
    drawtext=text='Built with Rust + D3.js':fontsize=${FONTSIZE}:fontcolor=#58a6ff:x=(w-text_w)/2:y=(h/2)+230, \
    drawtext=text='KETI':fontsize=28:fontcolor=#6e7681:x=(w-text_w)/2:y=h-80" \
  -frames:v 1 frame_00_title.png

# ========================================
# SLIDE 1: Architecture Overview
# ========================================
echo "Creating architecture slide..."
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=${WIDTH}x${HEIGHT}:d=1" \
  -vf " \
    drawtext=text='Architecture':fontsize=${TITLESIZE}:fontcolor=white:x=(w-text_w)/2:y=60, \
    drawtext=text='┌─────────────────────────────────────────────────────────────┐':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=(w-text_w)/2:y=180, \
    drawtext=text='│                         TSN-Map                             │':fontsize=24:fontcolor=#58a6ff:fontfamily=monospace:x=(w-text_w)/2:y=210, \
    drawtext=text='├─────────────────────────────────────────────────────────────┤':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=(w-text_w)/2:y=240, \
    drawtext=text='│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=(w-text_w)/2:y=290, \
    drawtext=text='│  │   Packet    │    │   Topology  │    │     Web     │     │':fontsize=24:fontcolor=#f0883e:fontfamily=monospace:x=(w-text_w)/2:y=320, \
    drawtext=text='│  │   Capture   │───▶│   Builder   │───▶│   Server    │     │':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=(w-text_w)/2:y=350, \
    drawtext=text='│  │  (libpcap)  │    │   (Graph)   │    │   (Axum)    │     │':fontsize=24:fontcolor=#7ee787:fontfamily=monospace:x=(w-text_w)/2:y=380, \
    drawtext=text='│  └─────────────┘    └─────────────┘    └─────────────┘     │':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=(w-text_w)/2:y=410, \
    drawtext=text='└─────────────────────────────────────────────────────────────┘':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=(w-text_w)/2:y=460, \
    drawtext=text='                            ▼':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=(w-text_w)/2:y=500, \
    drawtext=text='┌─────────────────────────────────────────────────────────────┐':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=(w-text_w)/2:y=540, \
    drawtext=text='│                     Web Frontend                            │':fontsize=24:fontcolor=#58a6ff:fontfamily=monospace:x=(w-text_w)/2:y=570, \
    drawtext=text='├─────────────────────────────────────────────────────────────┤':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=(w-text_w)/2:y=600, \
    drawtext=text='│     D3.js          Chart.js        Server-Sent Events       │':fontsize=24:fontcolor=#f0883e:fontfamily=monospace:x=(w-text_w)/2:y=640, \
    drawtext=text='│   (Topology)      (Statistics)      (Real-time Data)        │':fontsize=24:fontcolor=#7ee787:fontfamily=monospace:x=(w-text_w)/2:y=670, \
    drawtext=text='└─────────────────────────────────────────────────────────────┘':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=(w-text_w)/2:y=710, \
    drawtext=text='Tech Stack: Rust • Axum • libpcap • D3.js • Chart.js • SSE':fontsize=32:fontcolor=#58a6ff:x=(w-text_w)/2:y=800" \
  -frames:v 1 frame_01_arch.png

# ========================================
# SLIDE 2: Code - Packet Capture
# ========================================
echo "Creating code slide 1..."
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=${WIDTH}x${HEIGHT}:d=1" \
  -vf " \
    drawtext=text='Packet Capture Implementation':fontsize=${TITLESIZE}:fontcolor=white:x=(w-text_w)/2:y=50, \
    drawtext=text='src/capture/mod.rs':fontsize=28:fontcolor=#8b949e:x=100:y=140, \
    drawbox=x=80:y=180:w=1760:h=500:color=#161b22:t=fill, \
    drawtext=text='pub fn start_capture(interface\: \&str) -> Result<()> \{':fontsize=26:fontcolor=#ff7b72:fontfamily=monospace:x=100:y=200, \
    drawtext=text='    let mut cap = Capture\:\:from_device(interface)?':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=240, \
    drawtext=text='        .promisc(true)':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=280, \
    drawtext=text='        .snaplen(65535)':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=320, \
    drawtext=text='        .timeout(1000)':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=360, \
    drawtext=text='        .open()?;':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=400, \
    drawtext=text='    ':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=440, \
    drawtext=text='    while let Ok(packet) = cap.next_packet() \{':fontsize=26:fontcolor=#ff7b72:fontfamily=monospace:x=100:y=480, \
    drawtext=text='        let parsed = parse_packet(packet.data);':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=520, \
    drawtext=text='        topology.update(parsed);':fontsize=26:fontcolor=#79c0ff:fontfamily=monospace:x=100:y=560, \
    drawtext=text='    \}':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=600, \
    drawtext=text='\}':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=640, \
    drawtext=text='• Raw packet capture using libpcap':fontsize=32:fontcolor=#7ee787:x=100:y=720, \
    drawtext=text='• Promiscuous mode for all traffic':fontsize=32:fontcolor=#7ee787:x=100:y=770, \
    drawtext=text='• Real-time topology updates':fontsize=32:fontcolor=#7ee787:x=100:y=820" \
  -frames:v 1 frame_02_code1.png

# ========================================
# SLIDE 3: Code - Protocol Parsing
# ========================================
echo "Creating code slide 2..."
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=${WIDTH}x${HEIGHT}:d=1" \
  -vf " \
    drawtext=text='Protocol Parsing':fontsize=${TITLESIZE}:fontcolor=white:x=(w-text_w)/2:y=50, \
    drawtext=text='src/capture/packet.rs':fontsize=28:fontcolor=#8b949e:x=100:y=140, \
    drawbox=x=80:y=180:w=1760:h=450:color=#161b22:t=fill, \
    drawtext=text='fn get_protocol_name(ethertype\: u16) -> String \{':fontsize=26:fontcolor=#ff7b72:fontfamily=monospace:x=100:y=200, \
    drawtext=text='    match ethertype \{':fontsize=26:fontcolor=#ff7b72:fontfamily=monospace:x=100:y=240, \
    drawtext=text='        0x0800 => \"IPv4\",':fontsize=26:fontcolor=#a5d6ff:fontfamily=monospace:x=100:y=280, \
    drawtext=text='        0x0806 => \"ARP\",':fontsize=26:fontcolor=#a5d6ff:fontfamily=monospace:x=100:y=320, \
    drawtext=text='        0x86DD => \"IPv6\",':fontsize=26:fontcolor=#a5d6ff:fontfamily=monospace:x=100:y=360, \
    drawtext=text='        0x8100 => \"VLAN\",':fontsize=26:fontcolor=#a5d6ff:fontfamily=monospace:x=100:y=400, \
    drawtext=text='        0x88CC => \"LLDP\",':fontsize=26:fontcolor=#a5d6ff:fontfamily=monospace:x=100:y=440, \
    drawtext=text='        0x88F7 => \"PTP\",    // IEEE 1588':fontsize=26:fontcolor=#7ee787:fontfamily=monospace:x=100:y=480, \
    drawtext=text='        _ => \"Unknown\"':fontsize=26:fontcolor=#a5d6ff:fontfamily=monospace:x=100:y=520, \
    drawtext=text='    \}':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=560, \
    drawtext=text='\}':fontsize=26:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=600, \
    drawtext=text='Supported: Ethernet • IPv4/IPv6 • TCP/UDP • ARP • LLDP • VLAN • PTP':fontsize=32:fontcolor=#58a6ff:x=(w-text_w)/2:y=700, \
    drawtext=text='TSN Protocols: IEEE 802.1Q (VLAN) • IEEE 1588 (PTP) • IEEE 802.1AB (LLDP)':fontsize=32:fontcolor=#f0883e:x=(w-text_w)/2:y=760" \
  -frames:v 1 frame_03_code2.png

# ========================================
# SLIDE 4: Feature - Topology (Screenshot)
# ========================================
echo "Creating feature slides..."
ffmpeg -y -i slide1_topology.png -vf "scale=${WIDTH}:${HEIGHT}:force_original_aspect_ratio=decrease,pad=${WIDTH}:${HEIGHT}:(ow-iw)/2:(oh-ih)/2:color=#0d1117, \
    drawbox=y=0:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='Feature 1\: Real-time Network Topology':fontsize=42:fontcolor=white:x=40:y=30, \
    drawbox=y=ih-100:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='• Auto-discovers network nodes from traffic • Force-directed graph layout • Click nodes to inspect':fontsize=28:fontcolor=#8b949e:x=40:y=h-65" \
  -frames:v 1 frame_04_topology.png

# ========================================
# SLIDE 5: Feature - Statistics (Screenshot)
# ========================================
ffmpeg -y -i slide2_stats.png -vf "scale=${WIDTH}:${HEIGHT}:force_original_aspect_ratio=decrease,pad=${WIDTH}:${HEIGHT}:(ow-iw)/2:(oh-ih)/2:color=#0d1117, \
    drawbox=y=0:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='Feature 2\: Traffic Statistics':fontsize=42:fontcolor=white:x=40:y=30, \
    drawbox=y=ih-100:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='• Protocol distribution • Real-time traffic rate • Top conversations • Packet size analysis':fontsize=28:fontcolor=#8b949e:x=40:y=h-65" \
  -frames:v 1 frame_05_stats.png

# ========================================
# SLIDE 6: Feature - Hosts (Screenshot)
# ========================================
ffmpeg -y -i slide3_hosts.png -vf "scale=${WIDTH}:${HEIGHT}:force_original_aspect_ratio=decrease,pad=${WIDTH}:${HEIGHT}:(ow-iw)/2:(oh-ih)/2:color=#0d1117, \
    drawbox=y=0:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='Feature 3\: Host Discovery':fontsize=42:fontcolor=white:x=40:y=30, \
    drawbox=y=ih-100:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='• MAC address detection • IP address mapping • Vendor identification (OUI lookup)':fontsize=28:fontcolor=#8b949e:x=40:y=h-65" \
  -frames:v 1 frame_06_hosts.png

# ========================================
# SLIDE 7: Feature - Packet Detail (Screenshot)
# ========================================
ffmpeg -y -i slide4_detail1.png -vf "scale=${WIDTH}:${HEIGHT}:force_original_aspect_ratio=decrease,pad=${WIDTH}:${HEIGHT}:(ow-iw)/2:(oh-ih)/2:color=#0d1117, \
    drawbox=y=0:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='Feature 4\: Deep Packet Inspection':fontsize=42:fontcolor=white:x=40:y=30, \
    drawbox=y=ih-100:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='• Layer-by-layer analysis • Protocol headers decoded • Raw hex dump view':fontsize=28:fontcolor=#8b949e:x=40:y=h-65" \
  -frames:v 1 frame_07_detail.png

# ========================================
# SLIDE 8: Feature - Tester (Screenshot)
# ========================================
ffmpeg -y -i slide6_tester.png -vf "scale=${WIDTH}:${HEIGHT}:force_original_aspect_ratio=decrease,pad=${WIDTH}:${HEIGHT}:(ow-iw)/2:(oh-ih)/2:color=#0d1117, \
    drawbox=y=0:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='Feature 5\: Network Testing Tools':fontsize=42:fontcolor=white:x=40:y=30, \
    drawbox=y=ih-100:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='• ICMP ping with latency measurement • UDP packet generator • Real-time throughput chart':fontsize=28:fontcolor=#8b949e:x=40:y=h-65" \
  -frames:v 1 frame_08_tester.png

# ========================================
# SLIDE 9: Feature - Scale (Screenshot)
# ========================================
ffmpeg -y -i slide7_large.png -vf "scale=${WIDTH}:${HEIGHT}:force_original_aspect_ratio=decrease,pad=${WIDTH}:${HEIGHT}:(ow-iw)/2:(oh-ih)/2:color=#0d1117, \
    drawbox=y=0:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='Feature 6\: Large Scale Support':fontsize=42:fontcolor=white:x=40:y=30, \
    drawbox=y=ih-100:w=iw:h=100:color=#0d1117@0.9:t=fill, \
    drawtext=text='• Handles 100+ nodes efficiently • Optimized D3.js rendering • Interactive zoom and pan':fontsize=28:fontcolor=#8b949e:x=40:y=h-65" \
  -frames:v 1 frame_09_scale.png

# ========================================
# SLIDE 10: Closing
# ========================================
echo "Creating closing slide..."
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=${WIDTH}x${HEIGHT}:d=1" \
  -i app_icon.png \
  -filter_complex " \
    [1:v]scale=150:150[logo]; \
    [0:v][logo]overlay=(W-w)/2:(H-h)/2-200[bg]; \
    [bg]drawtext=text='TSN-Map':fontsize=${TITLESIZE}:fontcolor=white:x=(w-text_w)/2:y=(h/2), \
    drawtext=text='Open Source Network Visualization':fontsize=${SUBTITLESIZE}:fontcolor=#8b949e:x=(w-text_w)/2:y=(h/2)+80, \
    drawtext=text='github.com/keti/tsn-map':fontsize=${FONTSIZE}:fontcolor=#58a6ff:x=(w-text_w)/2:y=(h/2)+160, \
    drawtext=text='Rust • Axum • libpcap • D3.js • Chart.js':fontsize=28:fontcolor=#7ee787:x=(w-text_w)/2:y=(h/2)+230, \
    drawtext=text='© KETI - Korea Electronics Technology Institute':fontsize=24:fontcolor=#6e7681:x=(w-text_w)/2:y=h-60" \
  -frames:v 1 frame_10_closing.png

# ========================================
# Combine all frames into video
# ========================================
echo "Combining frames into video..."

# Duration for each slide (in seconds)
DUR_TITLE=4
DUR_ARCH=6
DUR_CODE=5
DUR_FEATURE=4
DUR_CLOSING=4

ffmpeg -y \
  -loop 1 -t $DUR_TITLE -i frame_00_title.png \
  -loop 1 -t $DUR_ARCH -i frame_01_arch.png \
  -loop 1 -t $DUR_CODE -i frame_02_code1.png \
  -loop 1 -t $DUR_CODE -i frame_03_code2.png \
  -loop 1 -t $DUR_FEATURE -i frame_04_topology.png \
  -loop 1 -t $DUR_FEATURE -i frame_05_stats.png \
  -loop 1 -t $DUR_FEATURE -i frame_06_hosts.png \
  -loop 1 -t $DUR_FEATURE -i frame_07_detail.png \
  -loop 1 -t $DUR_FEATURE -i frame_08_tester.png \
  -loop 1 -t $DUR_FEATURE -i frame_09_scale.png \
  -loop 1 -t $DUR_CLOSING -i frame_10_closing.png \
  -filter_complex " \
    [0:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v0]; \
    [1:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v1]; \
    [2:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v2]; \
    [3:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v3]; \
    [4:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v4]; \
    [5:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v5]; \
    [6:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v6]; \
    [7:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v7]; \
    [8:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v8]; \
    [9:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v9]; \
    [10:v]scale=${WIDTH}:${HEIGHT},setsar=1,format=yuv420p[v10]; \
    [v0][v1]xfade=transition=fade:duration=0.5:offset=3.5[f1]; \
    [f1][v2]xfade=transition=fade:duration=0.5:offset=9[f2]; \
    [f2][v3]xfade=transition=fade:duration=0.5:offset=13.5[f3]; \
    [f3][v4]xfade=transition=fade:duration=0.5:offset=18[f4]; \
    [f4][v5]xfade=transition=fade:duration=0.5:offset=21.5[f5]; \
    [f5][v6]xfade=transition=fade:duration=0.5:offset=25[f6]; \
    [f6][v7]xfade=transition=fade:duration=0.5:offset=28.5[f7]; \
    [f7][v8]xfade=transition=fade:duration=0.5:offset=32[f8]; \
    [f8][v9]xfade=transition=fade:duration=0.5:offset=35.5[f9]; \
    [f9][v10]xfade=transition=fade:duration=0.5:offset=39,format=yuv420p[vout]" \
  -map "[vout]" \
  -c:v libx264 -preset medium -crf 18 \
  -r 30 -pix_fmt yuv420p \
  /home/kim/tsn-map/tsn-map-demo-full.mp4

echo ""
echo "========================================="
echo "Video created successfully!"
echo "========================================="
ls -lh /home/kim/tsn-map/tsn-map-demo-full.mp4

# Clean up
cd /home/kim/tsn-map
rm -rf pic/video_work

echo "Done!"
