#!/usr/bin/env python3
"""
TSN-Map Demo Video Creator - Simple Version
Run: python3 make_video_simple.py
"""
import subprocess
import os
import sys

def run(cmd, check=True):
    print(f"$ {cmd}")
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if result.stdout:
        print(result.stdout)
    if result.stderr:
        print(result.stderr, file=sys.stderr)
    if check and result.returncode != 0:
        print(f"Error: Command failed with code {result.returncode}")
    return result.returncode == 0

def main():
    print("=" * 50)
    print("TSN-Map Demo Video Creator")
    print("=" * 50)

    # Install edge-tts
    print("\n[1/5] Installing edge-tts...")
    run("pip3 install --break-system-packages edge-tts", check=False)

    # Setup directories
    base = "/home/kim/tsn-map"
    work = f"{base}/pic/video_work"
    os.makedirs(work, exist_ok=True)
    os.chdir(work)
    print(f"Working in: {work}")

    # Screenshot mapping
    screenshots = [
        ("Screenshot from 2026-01-19 15-31-13.png", "s01_interface.png"),
        ("Screenshot from 2026-01-19 15-16-01.png", "s02_topology.png"),
        ("Screenshot from 2026-01-19 15-32-57.png", "s03_filter.png"),
        ("Screenshot from 2026-01-19 15-33-05.png", "s04_stats.png"),
        ("Screenshot from 2026-01-19 15-33-10.png", "s05_hosts.png"),
        ("Screenshot from 2026-01-19 15-32-37.png", "s06_detail.png"),
        ("Screenshot from 2026-01-19 15-18-14.png", "s07_tester.png"),
        ("Screenshot from 2026-01-19 15-18-44.png", "s08_large.png"),
    ]

    print("\n[2/5] Copying screenshots...")
    for src, dst in screenshots:
        run(f'cp "{base}/pic/{src}" "{dst}"', check=False)

    # TTS narrations
    print("\n[3/5] Generating TTS narration...")
    voice = "en-US-AriaNeural"
    narrations = [
        ("a00.mp3", "Welcome to TSN-Map. A real-time network topology visualization tool built with Rust and D3.js. Developed by KETI."),
        ("a01.mp3", "The system architecture uses Rust with Axum for the backend and libpcap for packet capture. The frontend uses D3.js for topology and Chart.js for statistics."),
        ("a02.mp3", "Select your network interface to start capturing packets."),
        ("a03.mp3", "The topology view auto-discovers network nodes from traffic with force-directed layout."),
        ("a04.mp3", "Filter packets by IP address or protocol. The view updates in real-time."),
        ("a05.mp3", "Statistics show protocol distribution, traffic rate, and top conversations."),
        ("a06.mp3", "Host discovery displays MAC, IP, vendor, and port information."),
        ("a07.mp3", "Deep packet inspection shows layer by layer breakdown with hex dump."),
        ("a08.mp3", "The packet generator sends UDP traffic for performance testing."),
        ("a09.mp3", "TSN-Map handles large networks with over 100 nodes efficiently."),
        ("a10.mp3", "Thank you for watching. TSN-Map is open source by KETI."),
    ]

    for fname, text in narrations:
        run(f'edge-tts --voice {voice} --text "{text}" --write-media {fname}')

    # Create frames
    print("\n[4/5] Creating video frames...")
    icon = f"{base}/src-tauri/icons/icon.png"

    # Title frame
    run(f'''ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" -i "{icon}" \
        -filter_complex "[1:v]scale=140:140[logo];[0:v][logo]overlay=(W-w)/2:(H-h)/2-140[bg]; \
        [bg]drawtext=text='TSN-Map':fontsize=80:fontcolor=white:x=(w-text_w)/2:y=(h/2)+30, \
        drawtext=text='Network Topology Visualization':fontsize=32:fontcolor=#888888:x=(w-text_w)/2:y=(h/2)+120" \
        -frames:v 1 f00.png''')

    # Architecture frame
    run(f'''ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" \
        -vf "drawtext=text='Architecture':fontsize=60:fontcolor=white:x=(w-text_w)/2:y=80, \
        drawtext=text='Backend\\: Rust + Axum + libpcap':fontsize=36:fontcolor=#58a6ff:x=(w-text_w)/2:y=300, \
        drawtext=text='Frontend\\: D3.js + Chart.js + SSE':fontsize=36:fontcolor=#f0883e:x=(w-text_w)/2:y=400, \
        drawtext=text='Protocols\\: Ethernet, IPv4/6, TCP, UDP, ARP, LLDP, VLAN, PTP':fontsize=28:fontcolor=#7ee787:x=(w-text_w)/2:y=550" \
        -frames:v 1 f01.png''')

    # Screenshot frames
    frames = [
        ("s01_interface.png", "f02.png", "Interface Selection"),
        ("s02_topology.png", "f03.png", "Network Topology"),
        ("s03_filter.png", "f04.png", "Packet Filtering"),
        ("s04_stats.png", "f05.png", "Statistics"),
        ("s05_hosts.png", "f06.png", "Host Discovery"),
        ("s06_detail.png", "f07.png", "Packet Details"),
        ("s07_tester.png", "f08.png", "Packet Generator"),
        ("s08_large.png", "f09.png", "Large Scale"),
    ]

    for src, dst, title in frames:
        run(f'''ffmpeg -y -i {src} -vf "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=#0d1117, \
            drawbox=y=0:w=iw:h=70:color=#0d1117@0.9:t=fill, \
            drawtext=text='{title}':fontsize=32:fontcolor=white:x=30:y=18" \
            -frames:v 1 {dst}''')

    # Closing frame
    run(f'''ffmpeg -y -f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" -i "{icon}" \
        -filter_complex "[1:v]scale=100:100[logo];[0:v][logo]overlay=(W-w)/2:(H-h)/2-120[bg]; \
        [bg]drawtext=text='TSN-Map':fontsize=60:fontcolor=white:x=(w-text_w)/2:y=(h/2)+20, \
        drawtext=text='Open Source - KETI':fontsize=28:fontcolor=#888888:x=(w-text_w)/2:y=(h/2)+100" \
        -frames:v 1 f10.png''')

    # Create video segments
    print("\n[5/5] Creating video segments...")
    segments = [
        ("f00.png", "a00.mp3", "v00.mp4"),
        ("f01.png", "a01.mp3", "v01.mp4"),
        ("f02.png", "a02.mp3", "v02.mp4"),
        ("f03.png", "a03.mp3", "v03.mp4"),
        ("f04.png", "a04.mp3", "v04.mp4"),
        ("f05.png", "a05.mp3", "v05.mp4"),
        ("f06.png", "a06.mp3", "v06.mp4"),
        ("f07.png", "a07.mp3", "v07.mp4"),
        ("f08.png", "a08.mp3", "v08.mp4"),
        ("f09.png", "a09.mp3", "v09.mp4"),
        ("f10.png", "a10.mp3", "v10.mp4"),
    ]

    for frame, audio, video in segments:
        run(f'ffmpeg -y -loop 1 -i {frame} -i {audio} -c:v libx264 -tune stillimage -c:a aac -shortest -pix_fmt yuv420p {video}')

    # Concat list
    with open("concat.txt", "w") as f:
        for _, _, video in segments:
            f.write(f"file '{video}'\n")

    # Final video
    output = f"{base}/tsn-map-demo-full.mp4"
    run(f'ffmpeg -y -f concat -safe 0 -i concat.txt -c:v libx264 -crf 20 -c:a aac "{output}"')

    print("\n" + "=" * 50)
    print(f"Done! Video: {output}")
    run(f'ls -lh "{output}"')

    # Cleanup
    os.chdir(base)
    run(f'rm -rf "{work}"', check=False)

if __name__ == "__main__":
    main()
