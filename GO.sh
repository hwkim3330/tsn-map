#!/bin/bash
# Run this: bash /home/kim/tsn-map/GO.sh

exec > /tmp/video_log.txt 2>&1
set -x

cd /home/kim/tsn-map
echo "=== Starting video creation ==="

# Create workspace
rm -rf pic/video_work
mkdir -p pic/video_work
cd pic/video_work

# Copy files
for f in "../Screenshot from"*.png; do cp "$f" .; done
ls -la

# Rename
mv "Screenshot from 2026-01-19 15-31-13.png" s01.png 2>/dev/null || true
mv "Screenshot from 2026-01-19 15-16-01.png" s02.png 2>/dev/null || true
mv "Screenshot from 2026-01-19 15-32-57.png" s03.png 2>/dev/null || true
mv "Screenshot from 2026-01-19 15-33-05.png" s04.png 2>/dev/null || true
mv "Screenshot from 2026-01-19 15-33-10.png" s05.png 2>/dev/null || true
mv "Screenshot from 2026-01-19 15-32-37.png" s06.png 2>/dev/null || true
mv "Screenshot from 2026-01-19 15-18-14.png" s07.png 2>/dev/null || true
mv "Screenshot from 2026-01-19 15-18-44.png" s08.png 2>/dev/null || true

# Simple slideshow
ffmpeg -y \
  -loop 1 -t 5 -i s01.png \
  -loop 1 -t 5 -i s02.png \
  -loop 1 -t 5 -i s03.png \
  -loop 1 -t 5 -i s04.png \
  -loop 1 -t 5 -i s05.png \
  -loop 1 -t 5 -i s06.png \
  -loop 1 -t 5 -i s07.png \
  -loop 1 -t 5 -i s08.png \
  -filter_complex "[0:v]scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[v0];[1:v]scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[v1];[2:v]scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[v2];[3:v]scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[v3];[4:v]scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[v4];[5:v]scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[v5];[6:v]scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[v6];[7:v]scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[v7];[v0][v1][v2][v3][v4][v5][v6][v7]concat=n=8:v=1:a=0[out]" \
  -map "[out]" -c:v libx264 -pix_fmt yuv420p \
  /home/kim/tsn-map/demo.mp4

echo "=== Done ==="
ls -lh /home/kim/tsn-map/demo.mp4

# Cleanup
cd /home/kim/tsn-map
rm -rf pic/video_work
