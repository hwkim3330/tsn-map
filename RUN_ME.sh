#!/bin/bash
# TSN-Map Video Creator - Run this script!
# Usage: bash /home/kim/tsn-map/RUN_ME.sh

set -e
cd /home/kim/tsn-map

echo "=== TSN-Map Demo Video Creator ==="
echo ""

# Check Python
if ! command -v python3 &> /dev/null; then
    echo "Error: python3 not found"
    exit 1
fi

# Create venv to avoid system Python issues
echo "[1/6] Creating Python virtual environment..."
rm -rf .venv
python3 -m venv .venv
source .venv/bin/activate
pip install -q --upgrade pip
pip install -q gtts

echo "[2/6] Setting up workspace..."
rm -rf pic/video_work
mkdir -p pic/video_work
cd pic/video_work

# Copy screenshots
cp "../Screenshot from 2026-01-19 15-31-13.png" s01.png
cp "../Screenshot from 2026-01-19 15-16-01.png" s02.png
cp "../Screenshot from 2026-01-19 15-32-57.png" s03.png
cp "../Screenshot from 2026-01-19 15-33-05.png" s04.png
cp "../Screenshot from 2026-01-19 15-33-10.png" s05.png
cp "../Screenshot from 2026-01-19 15-32-37.png" s06.png
cp "../Screenshot from 2026-01-19 15-18-14.png" s07.png
cp "../Screenshot from 2026-01-19 15-18-44.png" s08.png
echo "  Screenshots ready"

echo "[3/6] Generating English TTS narration..."
python3 << 'PYEOF'
from gtts import gTTS
texts = [
    ("a00.mp3", "Welcome to TSN Map. A real-time network topology visualization tool built with Rust and D3.js. Developed by KETI."),
    ("a01.mp3", "The system architecture uses Rust with Axum for the backend and libpcap for packet capture. The frontend uses D3.js for topology visualization."),
    ("a02.mp3", "Select your network interface to start capturing packets."),
    ("a03.mp3", "The topology view auto-discovers network nodes from traffic with force-directed layout."),
    ("a04.mp3", "Filter packets by IP address or protocol. The view updates in real-time."),
    ("a05.mp3", "The statistics dashboard shows protocol distribution, traffic rate, and conversations."),
    ("a06.mp3", "Host discovery displays MAC, IP, vendor, and port information."),
    ("a07.mp3", "Deep packet inspection shows layer by layer breakdown with hex dump."),
    ("a08.mp3", "The packet generator sends UDP traffic for performance testing."),
    ("a09.mp3", "TSN Map handles large networks with over 100 nodes efficiently."),
    ("a10.mp3", "Thank you for watching. TSN Map is open source by KETI."),
]
for f, t in texts:
    gTTS(text=t, lang='en').save(f)
    print(f"  {f} done")
PYEOF

echo "[4/6] Creating video frames with ffmpeg..."
ICON="/home/kim/tsn-map/src-tauri/icons/icon.png"

# Title frame
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" -i "$ICON" \
  -filter_complex "[1:v]scale=140:140[logo];[0:v][logo]overlay=(W-w)/2:(H-h)/2-140[bg];[bg]drawtext=text='TSN-Map':fontsize=80:fontcolor=white:x=(w-text_w)/2:y=(h/2)+30,drawtext=text='Network Topology Visualization':fontsize=32:fontcolor=#888888:x=(w-text_w)/2:y=(h/2)+110,drawtext=text='KETI':fontsize=24:fontcolor=#666666:x=(w-text_w)/2:y=h-60" \
  -frames:v 1 f00.png 2>/dev/null && echo "  Title frame"

# Architecture frame
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" \
  -vf "drawtext=text='Architecture':fontsize=56:fontcolor=white:x=(w-text_w)/2:y=60,drawtext=text='Backend\: Rust + Axum + libpcap':fontsize=36:fontcolor=#58a6ff:x=(w-text_w)/2:y=280,drawtext=text='Frontend\: D3.js + Chart.js + SSE':fontsize=36:fontcolor=#f0883e:x=(w-text_w)/2:y=380,drawtext=text='Protocols\: Ethernet IPv4 TCP UDP ARP LLDP VLAN PTP':fontsize=26:fontcolor=#7ee787:x=(w-text_w)/2:y=520" \
  -frames:v 1 f01.png 2>/dev/null && echo "  Architecture frame"

# Screenshot frames
TITLES=("" "Interface Selection" "Network Topology" "Packet Filtering" "Statistics" "Host Discovery" "Packet Details" "Packet Generator" "Large Scale")
for i in 1 2 3 4 5 6 7 8; do
  ffmpeg -y -i "s0${i}.png" \
    -vf "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:#0d1117,drawbox=y=0:w=iw:h=70:color=#0d1117@0.9:t=fill,drawtext=text='${TITLES[$i]}':fontsize=32:fontcolor=white:x=30:y=18" \
    -frames:v 1 "f0$((i+1)).png" 2>/dev/null && echo "  Frame $((i+1))"
done

# Closing frame
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" -i "$ICON" \
  -filter_complex "[1:v]scale=100:100[logo];[0:v][logo]overlay=(W-w)/2:(H-h)/2-100[bg];[bg]drawtext=text='TSN-Map':fontsize=60:fontcolor=white:x=(w-text_w)/2:y=(h/2)+30,drawtext=text='Open Source by KETI':fontsize=26:fontcolor=#888888:x=(w-text_w)/2:y=(h/2)+100" \
  -frames:v 1 f10.png 2>/dev/null && echo "  Closing frame"

echo "[5/6] Creating video segments..."
for i in 00 01 02 03 04 05 06 07 08 09 10; do
  ffmpeg -y -loop 1 -i "f${i}.png" -i "a${i}.mp3" \
    -c:v libx264 -tune stillimage -c:a aac -shortest -pix_fmt yuv420p \
    "v${i}.mp4" 2>/dev/null && echo "  Segment $i"
done

echo "[6/6] Combining final video..."
cat > concat.txt << EOF
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

ffmpeg -y -f concat -safe 0 -i concat.txt -c:v libx264 -crf 20 -c:a aac \
  /home/kim/tsn-map/tsn-map-demo-full.mp4 2>/dev/null

# Cleanup
cd /home/kim/tsn-map
deactivate
rm -rf pic/video_work .venv

echo ""
echo "=========================================="
echo "SUCCESS!"
echo "=========================================="
ls -lh /home/kim/tsn-map/tsn-map-demo-full.mp4
echo ""
echo "Play: vlc /home/kim/tsn-map/tsn-map-demo-full.mp4"
