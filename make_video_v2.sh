#!/bin/bash
# Video creator v2 - No overlay on top (preserve logo)
# Title appears at BOTTOM of screen

set -e
cd /home/kim/tsn-map
mkdir -p pic/video_work
cd pic/video_work

echo "=== Creating Video (Logo Preserved) ==="

# Copy screenshots (all including new ones)
echo "[1/3] Copying screenshots..."
cp "../Screenshot from 2026-01-19 15-31-13.png" s01.png  # Overview
cp "../Screenshot from 2026-01-19 15-16-01.png" s02.png  # Interface
cp "../Screenshot from 2026-01-20 10-29-30.png" s03.png  # New: Topology
cp "../Screenshot from 2026-01-20 10-29-57.png" s04.png  # New: Statistics
cp "../Screenshot from 2026-01-20 10-30-16.png" s05.png  # New: IO Graph
cp "../Screenshot from 2026-01-19 15-32-37.png" s06.png  # Hosts
cp "../Screenshot from 2026-01-19 15-18-14.png" s07.png  # Details
cp "../Screenshot from 2026-01-19 15-18-44.png" s08.png  # Tester

ICON="/home/kim/tsn-map/icon.png"

echo "[2/3] Creating frames..."

# Title slide
ffmpeg -y -f lavfi -i "color=c=#1c1c1e:s=1920x1080:d=1" -i "$ICON" \
  -filter_complex "[1:v]scale=160:160[l];[0:v][l]overlay=880:280[b];[b]drawtext=text='TSN-Map':fontsize=80:fontcolor=white:x=(w-text_w)/2:y=500,drawtext=text='Network Visualization & Analysis Tool':fontsize=32:fontcolor=#98989d:x=(w-text_w)/2:y=600,drawtext=text='KETI':fontsize=24:fontcolor=#58a6ff:x=(w-text_w)/2:y=1000" \
  -frames:v 1 f00.png 2>/dev/null
echo "  Title"

# Architecture slide
ffmpeg -y -f lavfi -i "color=c=#1c1c1e:s=1920x1080:d=1" \
  -vf "drawtext=text='Architecture':fontsize=56:fontcolor=white:x=(w-text_w)/2:y=80,drawtext=text='Backend':fontsize=40:fontcolor=#0a84ff:x=(w-text_w)/2:y=280,drawtext=text='Rust + Axum + libpcap':fontsize=28:fontcolor=#98989d:x=(w-text_w)/2:y=340,drawtext=text='Frontend':fontsize=40:fontcolor=#30d158:x=(w-text_w)/2:y=480,drawtext=text='D3.js + Chart.js + Vanilla JS':fontsize=28:fontcolor=#98989d:x=(w-text_w)/2:y=540,drawtext=text='Features':fontsize=40:fontcolor=#ff9f0a:x=(w-text_w)/2:y=680,drawtext=text='Real-time Capture | IO Graph | TSN Support':fontsize=28:fontcolor=#98989d:x=(w-text_w)/2:y=740" \
  -frames:v 1 f01.png 2>/dev/null
echo "  Architecture"

# Screenshots - title at BOTTOM (not covering logo at top)
TITLES=("" "Interface Selection" "Network Topology" "Traffic Statistics" "IO Graph Analysis" "Host Discovery" "Packet Details" "Tester - Ping & Generator")
for i in 1 2 3 4 5 6 7 8; do
  ffmpeg -y -i "s0${i}.png" \
    -vf "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:#1c1c1e,drawbox=y=ih-60:w=iw:h=60:color=#000000@0.7:t=fill,drawtext=text='${TITLES[$i]}':fontsize=28:fontcolor=white:x=(w-text_w)/2:y=h-45" \
    -frames:v 1 "f0$((i+1)).png" 2>/dev/null
  echo "  ${TITLES[$i]}"
done

# Closing slide
ffmpeg -y -f lavfi -i "color=c=#1c1c1e:s=1920x1080:d=1" -i "$ICON" \
  -filter_complex "[1:v]scale=100:100[l];[0:v][l]overlay=910:350[b];[b]drawtext=text='TSN-Map':fontsize=60:fontcolor=white:x=(w-text_w)/2:y=480,drawtext=text='github.com/hwkim3330/tsn-map':fontsize=24:fontcolor=#58a6ff:x=(w-text_w)/2:y=560,drawtext=text='Open Source by KETI':fontsize=20:fontcolor=#6e6e73:x=(w-text_w)/2:y=1020" \
  -frames:v 1 f10.png 2>/dev/null
echo "  Closing"

echo "[3/3] Creating video..."
# Create slideshow (4 seconds per slide)
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
  -filter_complex "[0:v][1:v][2:v][3:v][4:v][5:v][6:v][7:v][8:v][9:v][10:v]concat=n=11:v=1:a=0,fps=30[out]" \
  -map "[out]" \
  -c:v libx264 -preset medium -crf 23 -pix_fmt yuv420p \
  /home/kim/tsn-map/tsn-map-demo-v2.mp4 2>/dev/null

cd /home/kim/tsn-map
rm -rf pic/video_work

echo ""
echo "=== Done! ==="
echo "Video: /home/kim/tsn-map/tsn-map-demo-v2.mp4"
ls -lh /home/kim/tsn-map/tsn-map-demo-v2.mp4
echo ""
echo "Duration: ~45 seconds"
echo "Resolution: 1920x1080"
