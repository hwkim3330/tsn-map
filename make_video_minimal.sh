#!/bin/bash
# Minimal video creator - no TTS, just slides with text
# If Python/TTS doesn't work, this will at least create a silent video

set -e
cd /home/kim/tsn-map
mkdir -p pic/video_work
cd pic/video_work

echo "=== Creating Video (No TTS) ==="

# Copy screenshots
echo "[1/3] Copying screenshots..."
cp "../Screenshot from 2026-01-19 15-31-13.png" s01.png
cp "../Screenshot from 2026-01-19 15-16-01.png" s02.png
cp "../Screenshot from 2026-01-19 15-32-57.png" s03.png
cp "../Screenshot from 2026-01-19 15-33-05.png" s04.png
cp "../Screenshot from 2026-01-19 15-33-10.png" s05.png
cp "../Screenshot from 2026-01-19 15-32-37.png" s06.png
cp "../Screenshot from 2026-01-19 15-18-14.png" s07.png
cp "../Screenshot from 2026-01-19 15-18-44.png" s08.png

ICON="/home/kim/tsn-map/src-tauri/icons/icon.png"

echo "[2/3] Creating frames..."
# Title
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" -i "$ICON" \
  -filter_complex "[1:v]scale=140:140[l];[0:v][l]overlay=890:340[b];[b]drawtext=text='TSN-Map':fontsize=80:fontcolor=white:x=(w-text_w)/2:y=570,drawtext=text='Network Topology Visualization':fontsize=32:fontcolor=#888888:x=(w-text_w)/2:y=660,drawtext=text='KETI':fontsize=24:fontcolor=#666666:x=(w-text_w)/2:y=1000" \
  -frames:v 1 f00.png 2>/dev/null
echo "  Title"

# Arch
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" \
  -vf "drawtext=text='Architecture':fontsize=56:fontcolor=white:x=(w-text_w)/2:y=60,drawtext=text='Backend - Rust + Axum + libpcap':fontsize=36:fontcolor=#58a6ff:x=(w-text_w)/2:y=280,drawtext=text='Frontend - D3.js + Chart.js':fontsize=36:fontcolor=#f0883e:x=(w-text_w)/2:y=380" \
  -frames:v 1 f01.png 2>/dev/null
echo "  Architecture"

# Screenshots with titles
TITLES=("" "Interface Selection" "Network Topology" "Packet Filtering" "Statistics" "Host Discovery" "Packet Details" "Packet Generator" "Large Scale")
for i in 1 2 3 4 5 6 7 8; do
  ffmpeg -y -i "s0${i}.png" \
    -vf "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:#0d1117,drawbox=y=0:w=iw:h=70:color=#0d1117:t=fill,drawtext=text='${TITLES[$i]}':fontsize=32:fontcolor=white:x=30:y=18" \
    -frames:v 1 "f0$((i+1)).png" 2>/dev/null
  echo "  ${TITLES[$i]}"
done

# Closing
ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" \
  -vf "drawtext=text='TSN-Map':fontsize=60:fontcolor=white:x=(w-text_w)/2:y=450,drawtext=text='Open Source by KETI':fontsize=26:fontcolor=#888888:x=(w-text_w)/2:y=540" \
  -frames:v 1 f10.png 2>/dev/null
echo "  Closing"

echo "[3/3] Creating video..."
# Create slideshow (5 seconds per slide, no audio)
ffmpeg -y \
  -loop 1 -t 4 -i f00.png \
  -loop 1 -t 5 -i f01.png \
  -loop 1 -t 4 -i f02.png \
  -loop 1 -t 4 -i f03.png \
  -loop 1 -t 4 -i f04.png \
  -loop 1 -t 4 -i f05.png \
  -loop 1 -t 4 -i f06.png \
  -loop 1 -t 4 -i f07.png \
  -loop 1 -t 4 -i f08.png \
  -loop 1 -t 4 -i f09.png \
  -loop 1 -t 4 -i f10.png \
  -filter_complex "[0:v][1:v][2:v][3:v][4:v][5:v][6:v][7:v][8:v][9:v][10:v]concat=n=11:v=1:a=0[out]" \
  -map "[out]" \
  -c:v libx264 -pix_fmt yuv420p -r 30 \
  /home/kim/tsn-map/tsn-map-demo-silent.mp4 2>/dev/null

cd /home/kim/tsn-map
rm -rf pic/video_work

echo ""
echo "=== Done! ==="
ls -lh /home/kim/tsn-map/tsn-map-demo-silent.mp4
