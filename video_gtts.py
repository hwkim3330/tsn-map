#!/usr/bin/env python3
"""TSN-Map Video Creator using gTTS"""
import os
import sys
import subprocess

# Change to valid directory first
os.chdir("/home/kim/tsn-map")

def run(cmd):
    print(f"$ {cmd[:80]}..." if len(cmd) > 80 else f"$ {cmd}")
    r = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if r.stdout: print(r.stdout)
    if r.stderr and r.returncode != 0: print(r.stderr)
    return r.returncode == 0

print("=== TSN-Map Video Creator (gTTS) ===\n")

# Install gTTS
print("[1/6] Installing gTTS...")
run("pip3 install --break-system-packages --quiet gtts")

# Setup
work = "/home/kim/tsn-map/pic/video_work"
os.makedirs(work, exist_ok=True)
os.chdir(work)
print(f"Working in: {os.getcwd()}")

# Copy screenshots
print("\n[2/6] Copying screenshots...")
pics = [
    ("Screenshot from 2026-01-19 15-31-13.png", "s01.png"),
    ("Screenshot from 2026-01-19 15-16-01.png", "s02.png"),
    ("Screenshot from 2026-01-19 15-32-57.png", "s03.png"),
    ("Screenshot from 2026-01-19 15-33-05.png", "s04.png"),
    ("Screenshot from 2026-01-19 15-33-10.png", "s05.png"),
    ("Screenshot from 2026-01-19 15-32-37.png", "s06.png"),
    ("Screenshot from 2026-01-19 15-18-14.png", "s07.png"),
    ("Screenshot from 2026-01-19 15-18-44.png", "s08.png"),
]
for src, dst in pics:
    run(f'cp "/home/kim/tsn-map/pic/{src}" "{dst}"')

# Generate TTS with gTTS
print("\n[3/6] Generating TTS with gTTS...")
from gtts import gTTS

narrations = [
    ("a00.mp3", "Welcome to TSN Map. A real-time network topology visualization tool. Built with Rust and D3.js by KETI."),
    ("a01.mp3", "The architecture uses Rust with Axum and libpcap for backend. D3.js and Chart.js power the frontend."),
    ("a02.mp3", "Select your network interface to start capturing packets."),
    ("a03.mp3", "The topology view auto discovers network nodes with force directed layout."),
    ("a04.mp3", "Filter packets by IP address or protocol. Updates in real time."),
    ("a05.mp3", "Statistics show protocol distribution, traffic rate, and conversations."),
    ("a06.mp3", "Host discovery shows MAC, IP, vendor, and port information."),
    ("a07.mp3", "Packet details show layer by layer breakdown with hex dump."),
    ("a08.mp3", "The packet generator sends UDP traffic for testing."),
    ("a09.mp3", "Handles large networks with over 100 nodes efficiently."),
    ("a10.mp3", "Thank you for watching. TSN Map is open source by KETI."),
]

for fname, text in narrations:
    tts = gTTS(text=text, lang='en', slow=False)
    tts.save(fname)
    print(f"  {fname} done")

# Create frames
print("\n[4/6] Creating video frames...")
icon = "/home/kim/tsn-map/src-tauri/icons/icon.png"

# Title frame
run(f'''ffmpeg -y -f lavfi -i color=c=#0d1117:s=1920x1080:d=1 -i "{icon}" -filter_complex "[1:v]scale=140:140[l];[0:v][l]overlay=(W-w)/2:(H-h)/2-140[b];[b]drawtext=text='TSN-Map':fontsize=80:fontcolor=white:x=(w-text_w)/2:y=(h/2)+30,drawtext=text='Network Topology Visualization':fontsize=32:fontcolor=#888888:x=(w-text_w)/2:y=(h/2)+110,drawtext=text='KETI':fontsize=24:fontcolor=#666666:x=(w-text_w)/2:y=h-60" -frames:v 1 f00.png 2>/dev/null''')

# Arch frame
run('''ffmpeg -y -f lavfi -i color=c=#0d1117:s=1920x1080:d=1 -vf "drawtext=text='Architecture':fontsize=56:fontcolor=white:x=(w-text_w)/2:y=60,drawtext=text='Backend\\: Rust + Axum + libpcap':fontsize=36:fontcolor=#58a6ff:x=(w-text_w)/2:y=280,drawtext=text='Frontend\\: D3.js + Chart.js + SSE':fontsize=36:fontcolor=#f0883e:x=(w-text_w)/2:y=380,drawtext=text='Protocols\\: Ethernet IPv4 TCP UDP ARP LLDP VLAN PTP':fontsize=26:fontcolor=#7ee787:x=(w-text_w)/2:y=520" -frames:v 1 f01.png 2>/dev/null''')

# Screenshot frames
titles = ["", "Interface Selection", "Network Topology", "Packet Filtering", "Statistics", "Host Discovery", "Packet Details", "Packet Generator", "Large Scale"]
for i in range(1, 9):
    run(f'''ffmpeg -y -i s0{i}.png -vf "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:#0d1117,drawbox=y=0:w=iw:h=70:color=#0d1117@0.9:t=fill,drawtext=text='{titles[i]}':fontsize=32:fontcolor=white:x=30:y=18" -frames:v 1 f0{i+1}.png 2>/dev/null''')

# Closing frame
run(f'''ffmpeg -y -f lavfi -i color=c=#0d1117:s=1920x1080:d=1 -i "{icon}" -filter_complex "[1:v]scale=100:100[l];[0:v][l]overlay=(W-w)/2:(H-h)/2-100[b];[b]drawtext=text='TSN-Map':fontsize=60:fontcolor=white:x=(w-text_w)/2:y=(h/2)+30,drawtext=text='Open Source by KETI':fontsize=26:fontcolor=#888888:x=(w-text_w)/2:y=(h/2)+100" -frames:v 1 f10.png 2>/dev/null''')
print("  Frames done")

# Create video segments
print("\n[5/6] Creating video segments...")
for i in range(11):
    idx = f"{i:02d}"
    run(f'ffmpeg -y -loop 1 -i f{idx}.png -i a{idx}.mp3 -c:v libx264 -tune stillimage -c:a aac -shortest -pix_fmt yuv420p v{idx}.mp4 2>/dev/null')
    print(f"  Segment {i}/10")

# Concat
print("\n[6/6] Combining video...")
with open("concat.txt", "w") as f:
    for i in range(11):
        f.write(f"file 'v{i:02d}.mp4'\n")

run('ffmpeg -y -f concat -safe 0 -i concat.txt -c:v libx264 -crf 20 -c:a aac /home/kim/tsn-map/tsn-map-demo-full.mp4 2>/dev/null')

# Done
print("\n=== Complete! ===")
run('ls -lh /home/kim/tsn-map/tsn-map-demo-full.mp4')

# Cleanup
os.chdir("/home/kim/tsn-map")
run('rm -rf /home/kim/tsn-map/pic/video_work')
print("Done!")
