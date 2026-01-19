#!/usr/bin/env python3
"""
TSN-Map Video Creator
Handles common Python issues automatically
"""
import os
import sys
import subprocess
import shutil

def main():
    # Change to project directory
    os.chdir("/home/kim/tsn-map")
    print("=== TSN-Map Video Creator ===\n")

    # Create isolated venv to avoid system Python issues
    venv_path = "/home/kim/tsn-map/.video_venv"
    work_dir = "/home/kim/tsn-map/pic/video_work"

    print("[1/6] Creating isolated Python environment...")
    if os.path.exists(venv_path):
        shutil.rmtree(venv_path)

    result = subprocess.run([sys.executable, "-m", "venv", venv_path],
                          capture_output=True, text=True)
    if result.returncode != 0:
        print(f"venv creation failed: {result.stderr}")
        print("Trying with --without-pip...")
        subprocess.run([sys.executable, "-m", "venv", "--without-pip", venv_path])

    # Get venv Python path
    if os.name == 'nt':
        venv_python = os.path.join(venv_path, "Scripts", "python.exe")
    else:
        venv_python = os.path.join(venv_path, "bin", "python3")

    if not os.path.exists(venv_python):
        venv_python = os.path.join(venv_path, "bin", "python")

    print(f"  Using: {venv_python}")

    # Install pip if needed
    print("[2/6] Installing dependencies...")
    subprocess.run([venv_python, "-m", "ensurepip", "--upgrade"],
                  capture_output=True)
    subprocess.run([venv_python, "-m", "pip", "install", "--upgrade", "pip"],
                  capture_output=True)

    # Try to install gTTS
    result = subprocess.run([venv_python, "-m", "pip", "install", "gtts"],
                          capture_output=True, text=True)

    gtts_ok = result.returncode == 0
    if gtts_ok:
        print("  gTTS installed successfully")
    else:
        print("  gTTS failed, will create silent video")
        print(f"  Error: {result.stderr[:200] if result.stderr else 'unknown'}")

    # Setup workspace
    print("[3/6] Setting up workspace...")
    if os.path.exists(work_dir):
        shutil.rmtree(work_dir)
    os.makedirs(work_dir)
    os.chdir(work_dir)

    # Copy screenshots
    screenshots = [
        ("Screenshot from 2026-01-19 15-31-13.png", "s01.png"),
        ("Screenshot from 2026-01-19 15-16-01.png", "s02.png"),
        ("Screenshot from 2026-01-19 15-32-57.png", "s03.png"),
        ("Screenshot from 2026-01-19 15-33-05.png", "s04.png"),
        ("Screenshot from 2026-01-19 15-33-10.png", "s05.png"),
        ("Screenshot from 2026-01-19 15-32-37.png", "s06.png"),
        ("Screenshot from 2026-01-19 15-18-14.png", "s07.png"),
        ("Screenshot from 2026-01-19 15-18-44.png", "s08.png"),
    ]

    for src, dst in screenshots:
        src_path = f"/home/kim/tsn-map/pic/{src}"
        if os.path.exists(src_path):
            shutil.copy(src_path, dst)
    print("  Screenshots copied")

    # Generate TTS if available
    print("[4/6] Generating TTS narration...")
    if gtts_ok:
        tts_script = '''
import sys
sys.path.insert(0, "{venv_path}/lib/python3.12/site-packages")
sys.path.insert(0, "{venv_path}/lib/python3.11/site-packages")
sys.path.insert(0, "{venv_path}/lib/python3.10/site-packages")
from gtts import gTTS
texts = [
    ("a00.mp3", "Welcome to TSN Map. A real-time network topology visualization tool by KETI."),
    ("a01.mp3", "Architecture: Rust backend with Axum and libpcap. D3.js frontend."),
    ("a02.mp3", "Select your network interface."),
    ("a03.mp3", "Topology auto-discovers nodes."),
    ("a04.mp3", "Filter by IP or protocol."),
    ("a05.mp3", "Statistics dashboard."),
    ("a06.mp3", "Host discovery."),
    ("a07.mp3", "Packet details."),
    ("a08.mp3", "Packet generator."),
    ("a09.mp3", "Large scale support."),
    ("a10.mp3", "Thank you. Open source by KETI."),
]
for f, t in texts:
    try:
        gTTS(text=t, lang='en').save(f)
        print(f"  {{f}}")
    except Exception as e:
        print(f"  {{f}} FAILED: {{e}}")
'''.format(venv_path=venv_path)

        result = subprocess.run([venv_python, "-c", tts_script],
                              capture_output=True, text=True)
        print(result.stdout)
        if result.stderr:
            print(f"  Warnings: {result.stderr[:200]}")

        has_audio = os.path.exists("a00.mp3")
    else:
        has_audio = False
        print("  Skipping TTS (not available)")

    # Create video frames
    print("[5/6] Creating video frames...")
    icon = "/home/kim/tsn-map/src-tauri/icons/icon.png"

    def ffmpeg(cmd):
        subprocess.run(f"ffmpeg -y {cmd} 2>/dev/null", shell=True)

    # Title frame
    ffmpeg(f'-f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" -i "{icon}" '
           f'-filter_complex "[1:v]scale=140:140[l];[0:v][l]overlay=890:340[b];'
           f'[b]drawtext=text=TSN-Map:fontsize=80:fontcolor=white:x=(w-text_w)/2:y=570,'
           f'drawtext=text=Network Topology Visualization:fontsize=32:fontcolor=#888888:x=(w-text_w)/2:y=660" '
           f'-frames:v 1 f00.png')
    print("  Title")

    # Architecture frame
    ffmpeg('-f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" '
           '-vf "drawtext=text=Architecture:fontsize=56:fontcolor=white:x=(w-text_w)/2:y=80,'
           'drawtext=text=Backend - Rust + Axum + libpcap:fontsize=36:fontcolor=#58a6ff:x=(w-text_w)/2:y=300,'
           'drawtext=text=Frontend - D3.js + Chart.js + SSE:fontsize=36:fontcolor=#f0883e:x=(w-text_w)/2:y=400" '
           '-frames:v 1 f01.png')
    print("  Architecture")

    # Screenshot frames
    titles = ["", "Interface", "Topology", "Filtering", "Statistics",
              "Hosts", "Details", "Tester", "Scale"]
    for i in range(1, 9):
        ffmpeg(f'-i s0{i}.png -vf "scale=1920:1080:force_original_aspect_ratio=decrease,'
               f'pad=1920:1080:(ow-iw)/2:(oh-ih)/2:#0d1117,'
               f'drawbox=y=0:w=iw:h=70:color=#0d1117:t=fill,'
               f'drawtext=text={titles[i]}:fontsize=32:fontcolor=white:x=30:y=18" '
               f'-frames:v 1 f0{i+1}.png')
    print("  Screenshot frames")

    # Closing frame
    ffmpeg('-f lavfi -i "color=c=#0d1117:s=1920x1080:d=1" '
           '-vf "drawtext=text=TSN-Map:fontsize=60:fontcolor=white:x=(w-text_w)/2:y=480,'
           'drawtext=text=Open Source by KETI:fontsize=26:fontcolor=#888888:x=(w-text_w)/2:y=560" '
           '-frames:v 1 f10.png')
    print("  Closing")

    # Create video
    print("[6/6] Creating final video...")

    if has_audio:
        # With audio - create segments and concat
        for i in range(11):
            idx = f"{i:02d}"
            ffmpeg(f'-loop 1 -i f{idx}.png -i a{idx}.mp3 '
                   f'-c:v libx264 -tune stillimage -c:a aac -shortest -pix_fmt yuv420p v{idx}.mp4')

        with open("concat.txt", "w") as f:
            for i in range(11):
                f.write(f"file 'v{i:02d}.mp4'\n")

        ffmpeg('-f concat -safe 0 -i concat.txt -c:v libx264 -crf 20 -c:a aac '
               '/home/kim/tsn-map/tsn-map-demo-full.mp4')
        output = "/home/kim/tsn-map/tsn-map-demo-full.mp4"
    else:
        # Silent video
        ffmpeg('-loop 1 -t 4 -i f00.png -loop 1 -t 5 -i f01.png '
               '-loop 1 -t 4 -i f02.png -loop 1 -t 4 -i f03.png '
               '-loop 1 -t 4 -i f04.png -loop 1 -t 4 -i f05.png '
               '-loop 1 -t 4 -i f06.png -loop 1 -t 4 -i f07.png '
               '-loop 1 -t 4 -i f08.png -loop 1 -t 4 -i f09.png '
               '-loop 1 -t 4 -i f10.png '
               '-filter_complex "[0][1][2][3][4][5][6][7][8][9][10]concat=n=11:v=1:a=0[out]" '
               '-map "[out]" -c:v libx264 -pix_fmt yuv420p '
               '/home/kim/tsn-map/tsn-map-demo-silent.mp4')
        output = "/home/kim/tsn-map/tsn-map-demo-silent.mp4"

    # Cleanup
    os.chdir("/home/kim/tsn-map")
    shutil.rmtree(work_dir)
    shutil.rmtree(venv_path)

    print("\n" + "=" * 40)
    if os.path.exists(output):
        size = os.path.getsize(output) / (1024 * 1024)
        print(f"SUCCESS! Video: {output}")
        print(f"Size: {size:.1f} MB")
    else:
        print("ERROR: Video creation failed")
    print("=" * 40)

if __name__ == "__main__":
    main()
