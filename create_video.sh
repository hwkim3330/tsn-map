#!/bin/bash
# TSN-Map Video Creator with Virtual Environment
set -e

cd /home/kim/tsn-map
echo "=== TSN-Map Demo Video Creator ==="

# Create and activate virtual environment
echo "[1/6] Setting up Python virtual environment..."
python3 -m venv venv
source venv/bin/activate

# Install dependencies
echo "[2/6] Installing dependencies..."
pip install --upgrade pip
pip install edge-tts pydub

# Create working directory
echo "[3/6] Preparing files..."
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

# Generate TTS
echo "[4/6] Generating TTS narration..."
V="en-US-AriaNeural"
edge-tts --voice $V --text "Welcome to TSN-Map. A real-time network topology visualization tool built with Rust and D3.js by KETI." -w a00.mp3
edge-tts --voice $V --text "The architecture uses Rust with Axum and libpcap for backend. D3.js and Chart.js for frontend visualization." -w a01.mp3
edge-tts --voice $V --text "Select your network interface to begin packet capture." -w a02.mp3
edge-tts --voice $V --text "Topology view auto-discovers nodes with force-directed layout." -w a03.mp3
edge-tts --voice $V --text "Filter packets by IP or protocol in real-time." -w a04.mp3
edge-tts --voice $V --text "Statistics show protocol distribution and traffic analysis." -w a05.mp3
edge-tts --voice $V --text "Host discovery shows MAC, IP, vendor, and ports." -w a06.mp3
edge-tts --voice $V --text "Packet details show layer by layer breakdown." -w a07.mp3
edge-tts --voice $V --text "Packet generator for network performance testing." -w a08.mp3
edge-tts --voice $V --text "Handles large networks with 100 plus nodes." -w a09.mp3
edge-tts --voice $V --text "Thank you. TSN-Map is open source by KETI." -w a10.mp3

# Create frames
echo "[5/6] Creating video frames..."
ICON="/home/kim/tsn-map/src-tauri/icons/icon.png"

# Title
ffmpeg -y -f lavfi -i color=c=#0d1117:s=1920x1080:d=1 -i "$ICON" -filter_complex \
  "[1:v]scale=140:140[l];[0:v][l]overlay=(W-w)/2:(H-h)/2-140[b];[b]drawtext=text='TSN-Map':fontsize=80:fontcolor=white:x=(w-text_w)/2:y=(h/2)+30,drawtext=text='Network Topology Visualization':fontsize=32:fontcolor=#888:x=(w-text_w)/2:y=(h/2)+110,drawtext=text='KETI':fontsize=24:fontcolor=#666:x=(w-text_w)/2:y=h-60" \
  -frames:v 1 f00.png 2>/dev/null

# Architecture
ffmpeg -y -f lavfi -i color=c=#0d1117:s=1920x1080:d=1 -vf \
  "drawtext=text='Architecture':fontsize=56:fontcolor=white:x=(w-text_w)/2:y=60,drawtext=text='Backend\: Rust + Axum + libpcap':fontsize=36:fontcolor=#58a6ff:x=(w-text_w)/2:y=280,drawtext=text='Frontend\: D3.js + Chart.js + SSE':fontsize=36:fontcolor=#f0883e:x=(w-text_w)/2:y=380,drawtext=text='Protocols\: Ethernet IPv4 TCP UDP ARP LLDP VLAN PTP':fontsize=26:fontcolor=#7ee787:x=(w-text_w)/2:y=520" \
  -frames:v 1 f01.png 2>/dev/null

# Screenshot frames
for i in 1 2 3 4 5 6 7 8; do
  t=("" "Interface" "Topology" "Filtering" "Statistics" "Hosts" "Details" "Tester" "Scale")
  ffmpeg -y -i s0$i.png -vf "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:#0d1117,drawbox=y=0:w=iw:h=70:color=#0d1117@0.9:t=fill,drawtext=text='${t[$i]}':fontsize=32:fontcolor=white:x=30:y=18" -frames:v 1 f0$((i+1)).png 2>/dev/null
done

# Closing
ffmpeg -y -f lavfi -i color=c=#0d1117:s=1920x1080:d=1 -i "$ICON" -filter_complex \
  "[1:v]scale=100:100[l];[0:v][l]overlay=(W-w)/2:(H-h)/2-100[b];[b]drawtext=text='TSN-Map':fontsize=60:fontcolor=white:x=(w-text_w)/2:y=(h/2)+30,drawtext=text='Open Source by KETI':fontsize=26:fontcolor=#888:x=(w-text_w)/2:y=(h/2)+100" \
  -frames:v 1 f10.png 2>/dev/null

# Create segments
echo "[6/6] Creating video..."
for i in 00 01 02 03 04 05 06 07 08 09 10; do
  ffmpeg -y -loop 1 -i f$i.png -i a$i.mp3 -c:v libx264 -tune stillimage -c:a aac -shortest -pix_fmt yuv420p v$i.mp4 2>/dev/null
  echo "  Segment $i done"
done

# Concat
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

ffmpeg -y -f concat -safe 0 -i concat.txt -c:v libx264 -crf 20 -c:a aac /home/kim/tsn-map/tsn-map-demo-full.mp4 2>/dev/null

# Cleanup
cd /home/kim/tsn-map
deactivate
rm -rf pic/video_work

echo ""
echo "=== Done! ==="
ls -lh tsn-map-demo-full.mp4
