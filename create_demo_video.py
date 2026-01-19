#!/usr/bin/env python3
"""
TSN-Map Demo Video Creator
Creates a professional demo video with TTS narration
"""

import os
import subprocess
import sys

# Check and install required packages
def install_packages():
    try:
        import gtts
    except ImportError:
        print("Installing gTTS...")
        subprocess.run([sys.executable, "-m", "pip", "install", "--break-system-packages", "gtts"], check=True)

    try:
        from pydub import AudioSegment
    except ImportError:
        print("Installing pydub...")
        subprocess.run([sys.executable, "-m", "pip", "install", "--break-system-packages", "pydub"], check=True)

install_packages()

from gtts import gTTS
from pydub import AudioSegment
import shutil
import tempfile

# Configuration
PROJECT_DIR = "/home/kim/tsn-map"
PIC_DIR = f"{PROJECT_DIR}/pic"
WORK_DIR = f"{PIC_DIR}/video_work"
OUTPUT_VIDEO = f"{PROJECT_DIR}/tsn-map-demo-full.mp4"

WIDTH = 1920
HEIGHT = 1080

# Slide definitions: (image_source, title, description, tts_text, duration)
SLIDES = [
    # Slide 0: Title
    {
        "type": "title",
        "title": "TSN-Map",
        "subtitle": "Real-time Network Topology Visualization",
        "text3": "Built with Rust + D3.js",
        "text4": "KETI",
        "tts": "Welcome to TSN-Map. A real-time network topology visualization tool built with Rust and D3.js. Developed by KETI.",
        "duration": 5
    },
    # Slide 1: Architecture
    {
        "type": "architecture",
        "tts": "TSN-Map uses a modern architecture. The backend is written in Rust with Axum web framework. Packet capture is handled by libpcap. The frontend uses D3.js for topology visualization and Chart.js for statistics. Real-time updates are delivered through Server-Sent Events.",
        "duration": 8
    },
    # Slide 2: Code - Packet Capture
    {
        "type": "code1",
        "tts": "Here's how packet capture works. We use libpcap to capture raw packets in promiscuous mode. Each captured packet is parsed and used to update the network topology in real-time.",
        "duration": 6
    },
    # Slide 3: Code - Protocol Parsing
    {
        "type": "code2",
        "tts": "The protocol parser supports multiple network protocols including IPv4, IPv6, ARP, LLDP, VLAN tagging, and TSN protocols like PTP for time synchronization.",
        "duration": 6
    },
    # Slide 4: Interface Selection
    {
        "type": "screenshot",
        "image": "Screenshot from 2026-01-19 15-31-13.png",
        "title": "Network Interface Selection",
        "desc": "Select from available network interfaces including physical NICs, loopback, and virtual interfaces",
        "tts": "When you start TSN-Map, you can select which network interface to capture from. This includes physical network cards, loopback interface, and virtual interfaces like Docker.",
        "duration": 5
    },
    # Slide 5: Topology View
    {
        "type": "screenshot",
        "image": "Screenshot from 2026-01-19 15-16-01.png",
        "title": "Real-time Network Topology",
        "desc": "Auto-discovers network nodes from traffic with force-directed graph layout",
        "tts": "The topology view automatically discovers network nodes from captured traffic. Nodes are displayed in a force-directed graph layout. You can drag, zoom, and click on nodes for more details.",
        "duration": 5
    },
    # Slide 6: Filtered Topology
    {
        "type": "screenshot",
        "image": "Screenshot from 2026-01-19 15-32-57.png",
        "title": "Topology Filtering",
        "desc": "Filter packets by IP address, protocol, or any criteria - topology updates accordingly",
        "tts": "You can filter packets by IP address, protocol, or any criteria. The topology view updates to show only relevant connections. Notice the device type legend showing routers, servers, hosts, and end stations.",
        "duration": 5
    },
    # Slide 7: Statistics Dashboard
    {
        "type": "screenshot",
        "image": "Screenshot from 2026-01-19 15-33-05.png",
        "title": "Traffic Statistics",
        "desc": "Protocol distribution, real-time traffic rate, top conversations, and packet size analysis",
        "tts": "The statistics dashboard provides comprehensive traffic analysis. View protocol distribution, real-time traffic rate over time, top ten conversations by traffic volume, and packet size distribution.",
        "duration": 5
    },
    # Slide 8: Hosts View
    {
        "type": "screenshot",
        "image": "Screenshot from 2026-01-19 15-33-10.png",
        "title": "Host Discovery",
        "desc": "Detailed host information with MAC, IP, vendor, packets, protocols, and ports",
        "tts": "The hosts view shows all discovered network devices. Each host displays MAC address, IP address, vendor identification from OUI lookup, packet counts, protocols used, and active ports.",
        "duration": 5
    },
    # Slide 9: Packet Detail
    {
        "type": "screenshot",
        "image": "Screenshot from 2026-01-19 15-32-37.png",
        "title": "Deep Packet Inspection",
        "desc": "Layer-by-layer packet analysis - Frame, Ethernet, IP, Transport, and raw data",
        "tts": "Click on any packet to see detailed inspection. The detail view shows layer by layer breakdown including frame information, Ethernet header, IP header, transport layer, and raw hexadecimal data.",
        "duration": 5
    },
    # Slide 10: Packet Detail 2
    {
        "type": "screenshot",
        "image": "Screenshot from 2026-01-19 15-17-14.png",
        "title": "Protocol Header Analysis",
        "desc": "Full protocol header decoding with field-by-field breakdown",
        "tts": "Each protocol layer is fully decoded. You can see source and destination MAC addresses, IP addresses, TTL values, port numbers, and protocol-specific flags.",
        "duration": 5
    },
    # Slide 11: Tester - Packet Generator
    {
        "type": "screenshot",
        "image": "Screenshot from 2026-01-19 15-18-14.png",
        "title": "Network Testing Tools",
        "desc": "Built-in packet generator for throughput testing with real-time monitoring",
        "tts": "TSN-Map includes network testing tools. The packet generator can send UDP traffic at configurable rates. Real-time throughput is displayed in the chart, useful for network performance testing.",
        "duration": 5
    },
    # Slide 12: Large Scale Topology
    {
        "type": "screenshot",
        "image": "Screenshot from 2026-01-19 15-18-44.png",
        "title": "Large Scale Support",
        "desc": "Efficiently handles 100+ nodes with optimized D3.js rendering",
        "tts": "TSN-Map efficiently handles large networks. This example shows over 100 nodes with more than 200 links. The D3.js rendering is optimized for smooth interaction even with complex topologies.",
        "duration": 5
    },
    # Slide 13: Closing
    {
        "type": "closing",
        "tts": "Thank you for watching. TSN-Map is open source and available on GitHub. Built with Rust, Axum, libpcap, D3.js, and Chart.js. Developed by KETI, Korea Electronics Technology Institute.",
        "duration": 6
    }
]

def create_work_dir():
    """Create working directory"""
    if os.path.exists(WORK_DIR):
        shutil.rmtree(WORK_DIR)
    os.makedirs(WORK_DIR)
    print(f"Created working directory: {WORK_DIR}")

def generate_tts(text, output_file):
    """Generate TTS audio using gTTS"""
    tts = gTTS(text=text, lang='en', slow=False)
    tts.save(output_file)
    print(f"Generated TTS: {output_file}")

def get_audio_duration(audio_file):
    """Get duration of audio file in seconds"""
    audio = AudioSegment.from_mp3(audio_file)
    return len(audio) / 1000.0

def create_title_slide(idx, slide):
    """Create title slide with logo"""
    output = f"{WORK_DIR}/frame_{idx:02d}.png"
    icon = f"{PROJECT_DIR}/src-tauri/icons/icon.png"

    cmd = f'''ffmpeg -y -f lavfi -i "color=c=#0d1117:s={WIDTH}x{HEIGHT}:d=1" \
        -i "{icon}" \
        -filter_complex " \
            [1:v]scale=180:180[logo]; \
            [0:v][logo]overlay=(W-w)/2:(H-h)/2-180[bg]; \
            [bg]drawtext=text='{slide["title"]}':fontsize=80:fontcolor=white:x=(w-text_w)/2:y=(h/2)+50, \
            drawtext=text='{slide["subtitle"]}':fontsize=40:fontcolor=#8b949e:x=(w-text_w)/2:y=(h/2)+140, \
            drawtext=text='{slide["text3"]}':fontsize=32:fontcolor=#58a6ff:x=(w-text_w)/2:y=(h/2)+200, \
            drawtext=text='{slide["text4"]}':fontsize=28:fontcolor=#6e7681:x=(w-text_w)/2:y=h-80" \
        -frames:v 1 "{output}" 2>/dev/null'''
    subprocess.run(cmd, shell=True, check=True)
    print(f"Created title slide: {output}")

def create_architecture_slide(idx, slide):
    """Create architecture diagram slide"""
    output = f"{WORK_DIR}/frame_{idx:02d}.png"

    cmd = f'''ffmpeg -y -f lavfi -i "color=c=#0d1117:s={WIDTH}x{HEIGHT}:d=1" \
        -vf " \
            drawtext=text='System Architecture':fontsize=60:fontcolor=white:x=(w-text_w)/2:y=40, \
            drawbox=x=100:y=120:w=800:h=280:color=#161b22:t=fill, \
            drawtext=text='Backend (Rust)':fontsize=32:fontcolor=#58a6ff:x=120:y=140, \
            drawtext=text='• Axum Web Framework':fontsize=24:fontcolor=#c9d1d9:x=140:y=190, \
            drawtext=text='• libpcap Packet Capture':fontsize=24:fontcolor=#c9d1d9:x=140:y=225, \
            drawtext=text='• Real-time SSE Streaming':fontsize=24:fontcolor=#c9d1d9:x=140:y=260, \
            drawtext=text='• Topology Graph Builder':fontsize=24:fontcolor=#c9d1d9:x=140:y=295, \
            drawtext=text='• Protocol Parsers':fontsize=24:fontcolor=#c9d1d9:x=140:y=330, \
            drawbox=x=1020:y=120:w=800:h=280:color=#161b22:t=fill, \
            drawtext=text='Frontend (Web)':fontsize=32:fontcolor=#f0883e:x=1040:y=140, \
            drawtext=text='• D3.js Force Graph':fontsize=24:fontcolor=#c9d1d9:x=1060:y=190, \
            drawtext=text='• Chart.js Statistics':fontsize=24:fontcolor=#c9d1d9:x=1060:y=225, \
            drawtext=text='• EventSource (SSE)':fontsize=24:fontcolor=#c9d1d9:x=1060:y=260, \
            drawtext=text='• Responsive UI':fontsize=24:fontcolor=#c9d1d9:x=1060:y=295, \
            drawtext=text='• Dark Theme':fontsize=24:fontcolor=#c9d1d9:x=1060:y=330, \
            drawtext=text='───────────────────────────────▶':fontsize=36:fontcolor=#7ee787:x=920:y=240, \
            drawbox=x=100:y=450:w=1720:h=200:color=#161b22:t=fill, \
            drawtext=text='Supported Protocols':fontsize=32:fontcolor=#7ee787:x=120:y=470, \
            drawtext=text='Ethernet • IPv4 • IPv6 • TCP • UDP • ARP • ICMP • LLDP • VLAN (802.1Q) • PTP (1588)':fontsize=26:fontcolor=#c9d1d9:x=120:y=520, \
            drawtext=text='MVRP • MRP • CDP • STP • LACP • HomePlug • IGMP • DNS • HTTP • TLS':fontsize=26:fontcolor=#c9d1d9:x=120:y=560, \
            drawbox=x=100:y=700:w=1720:h=120:color=#161b22:t=fill, \
            drawtext=text='Data Flow':fontsize=32:fontcolor=#58a6ff:x=120:y=720, \
            drawtext=text='Network Interface → Packet Capture → Protocol Parsing → Topology Building → SSE Stream → Web UI':fontsize=24:fontcolor=#c9d1d9:x=120:y=770" \
        -frames:v 1 "{output}" 2>/dev/null'''
    subprocess.run(cmd, shell=True, check=True)
    print(f"Created architecture slide: {output}")

def create_code_slide1(idx, slide):
    """Create code slide for packet capture"""
    output = f"{WORK_DIR}/frame_{idx:02d}.png"

    # Escape special characters for ffmpeg
    cmd = f'''ffmpeg -y -f lavfi -i "color=c=#0d1117:s={WIDTH}x{HEIGHT}:d=1" \
        -vf " \
            drawtext=text='Packet Capture - src/capture/mod.rs':fontsize=48:fontcolor=white:x=100:y=40, \
            drawbox=x=80:y=100:w=1760:h=520:color=#161b22:t=fill, \
            drawtext=text='pub async fn start_capture(iface\\: String) -> Result<()> {{':fontsize=24:fontcolor=#ff7b72:fontfamily=monospace:x=100:y=130, \
            drawtext=text='    let mut cap = Capture\\:\\:from_device(iface.as_str())?':fontsize=24:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=170, \
            drawtext=text='        .promisc(true)      // Capture all packets':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=100:y=210, \
            drawtext=text='        .snaplen(65535)     // Full packet capture':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=100:y=250, \
            drawtext=text='        .timeout(1000)      // 1 second timeout':fontsize=24:fontcolor=#8b949e:fontfamily=monospace:x=100:y=290, \
            drawtext=text='        .open()?;':fontsize=24:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=330, \
            drawtext=text='':fontsize=24:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=370, \
            drawtext=text='    while let Ok(packet) = cap.next_packet() {{':fontsize=24:fontcolor=#ff7b72:fontfamily=monospace:x=100:y=410, \
            drawtext=text='        let info = parse_packet(packet.data);':fontsize=24:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=450, \
            drawtext=text='        topology_tx.send(info).await?;  // Send to SSE':fontsize=24:fontcolor=#79c0ff:fontfamily=monospace:x=100:y=490, \
            drawtext=text='    }}':fontsize=24:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=530, \
            drawtext=text='}}':fontsize=24:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=570, \
            drawbox=x=80:y=650:w=1760:h=150:color=#238636@0.3:t=fill, \
            drawtext=text='Key Features':fontsize=32:fontcolor=#7ee787:x=100:y=670, \
            drawtext=text='• Uses libpcap for cross-platform packet capture':fontsize=26:fontcolor=#c9d1d9:x=100:y=720, \
            drawtext=text='• Async streaming to web clients via SSE':fontsize=26:fontcolor=#c9d1d9:x=100:y=760" \
        -frames:v 1 "{output}" 2>/dev/null'''
    subprocess.run(cmd, shell=True, check=True)
    print(f"Created code slide 1: {output}")

def create_code_slide2(idx, slide):
    """Create code slide for protocol parsing"""
    output = f"{WORK_DIR}/frame_{idx:02d}.png"

    cmd = f'''ffmpeg -y -f lavfi -i "color=c=#0d1117:s={WIDTH}x{HEIGHT}:d=1" \
        -vf " \
            drawtext=text='Protocol Parsing - src/capture/packet.rs':fontsize=48:fontcolor=white:x=100:y=40, \
            drawbox=x=80:y=100:w=1760:h=480:color=#161b22:t=fill, \
            drawtext=text='fn get_protocol_name(ethertype\\: u16) -> String {{':fontsize=24:fontcolor=#ff7b72:fontfamily=monospace:x=100:y=130, \
            drawtext=text='    match ethertype {{':fontsize=24:fontcolor=#ff7b72:fontfamily=monospace:x=100:y=170, \
            drawtext=text='        0x0800 => \"IPv4\".to_string(),':fontsize=24:fontcolor=#a5d6ff:fontfamily=monospace:x=100:y=210, \
            drawtext=text='        0x0806 => \"ARP\".to_string(),':fontsize=24:fontcolor=#a5d6ff:fontfamily=monospace:x=100:y=250, \
            drawtext=text='        0x86DD => \"IPv6\".to_string(),':fontsize=24:fontcolor=#a5d6ff:fontfamily=monospace:x=100:y=290, \
            drawtext=text='        0x8100 => \"VLAN\".to_string(),   // 802.1Q':fontsize=24:fontcolor=#7ee787:fontfamily=monospace:x=100:y=330, \
            drawtext=text='        0x88CC => \"LLDP\".to_string(),   // 802.1AB':fontsize=24:fontcolor=#7ee787:fontfamily=monospace:x=100:y=370, \
            drawtext=text='        0x88F7 => \"PTP\".to_string(),    // IEEE 1588':fontsize=24:fontcolor=#7ee787:fontfamily=monospace:x=100:y=410, \
            drawtext=text='        _ => format!(\"0x{{:04X}}\", ethertype)':fontsize=24:fontcolor=#a5d6ff:fontfamily=monospace:x=100:y=450, \
            drawtext=text='    }}':fontsize=24:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=490, \
            drawtext=text='}}':fontsize=24:fontcolor=#c9d1d9:fontfamily=monospace:x=100:y=530, \
            drawbox=x=80:y=620:w=850:h=180:color=#161b22:t=fill, \
            drawtext=text='Standard Protocols':fontsize=28:fontcolor=#58a6ff:x=100:y=640, \
            drawtext=text='IPv4, IPv6, TCP, UDP':fontsize=22:fontcolor=#c9d1d9:x=100:y=680, \
            drawtext=text='ARP, ICMP, DNS, HTTP':fontsize=22:fontcolor=#c9d1d9:x=100:y=710, \
            drawtext=text='IGMP, STP, LACP':fontsize=22:fontcolor=#c9d1d9:x=100:y=740, \
            drawbox=x=990:y=620:w=850:h=180:color=#161b22:t=fill, \
            drawtext=text='TSN Protocols':fontsize=28:fontcolor=#f0883e:x=1010:y=640, \
            drawtext=text='VLAN (802.1Q)':fontsize=22:fontcolor=#c9d1d9:x=1010:y=680, \
            drawtext=text='LLDP (802.1AB)':fontsize=22:fontcolor=#c9d1d9:x=1010:y=710, \
            drawtext=text='PTP (IEEE 1588)':fontsize=22:fontcolor=#c9d1d9:x=1010:y=740" \
        -frames:v 1 "{output}" 2>/dev/null'''
    subprocess.run(cmd, shell=True, check=True)
    print(f"Created code slide 2: {output}")

def create_screenshot_slide(idx, slide):
    """Create slide from screenshot with overlay"""
    output = f"{WORK_DIR}/frame_{idx:02d}.png"
    image = f"{PIC_DIR}/{slide['image']}"
    title = slide['title'].replace("'", "\\'")
    desc = slide['desc'].replace("'", "\\'")

    cmd = f'''ffmpeg -y -i "{image}" \
        -vf "scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=decrease,pad={WIDTH}:{HEIGHT}:(ow-iw)/2:(oh-ih)/2:color=#0d1117, \
            drawbox=y=0:w=iw:h=90:color=#0d1117@0.95:t=fill, \
            drawtext=text='{title}':fontsize=40:fontcolor=white:x=40:y=25, \
            drawbox=y=ih-80:w=iw:h=80:color=#0d1117@0.95:t=fill, \
            drawtext=text='{desc}':fontsize=24:fontcolor=#8b949e:x=40:y=h-55" \
        -frames:v 1 "{output}" 2>/dev/null'''
    subprocess.run(cmd, shell=True, check=True)
    print(f"Created screenshot slide: {output}")

def create_closing_slide(idx, slide):
    """Create closing slide"""
    output = f"{WORK_DIR}/frame_{idx:02d}.png"
    icon = f"{PROJECT_DIR}/src-tauri/icons/icon.png"

    cmd = f'''ffmpeg -y -f lavfi -i "color=c=#0d1117:s={WIDTH}x{HEIGHT}:d=1" \
        -i "{icon}" \
        -filter_complex " \
            [1:v]scale=140:140[logo]; \
            [0:v][logo]overlay=(W-w)/2:(H-h)/2-180[bg]; \
            [bg]drawtext=text='TSN-Map':fontsize=72:fontcolor=white:x=(w-text_w)/2:y=(h/2)+10, \
            drawtext=text='Open Source Network Visualization':fontsize=36:fontcolor=#8b949e:x=(w-text_w)/2:y=(h/2)+90, \
            drawtext=text='github.com/keti/tsn-map':fontsize=32:fontcolor=#58a6ff:x=(w-text_w)/2:y=(h/2)+160, \
            drawtext=text='Rust • Axum • libpcap • D3.js • Chart.js':fontsize=26:fontcolor=#7ee787:x=(w-text_w)/2:y=(h/2)+220, \
            drawtext=text='KETI - Korea Electronics Technology Institute':fontsize=24:fontcolor=#6e7681:x=(w-text_w)/2:y=h-80" \
        -frames:v 1 "{output}" 2>/dev/null'''
    subprocess.run(cmd, shell=True, check=True)
    print(f"Created closing slide: {output}")

def create_all_slides():
    """Create all slides"""
    for idx, slide in enumerate(SLIDES):
        slide_type = slide["type"]
        if slide_type == "title":
            create_title_slide(idx, slide)
        elif slide_type == "architecture":
            create_architecture_slide(idx, slide)
        elif slide_type == "code1":
            create_code_slide1(idx, slide)
        elif slide_type == "code2":
            create_code_slide2(idx, slide)
        elif slide_type == "screenshot":
            create_screenshot_slide(idx, slide)
        elif slide_type == "closing":
            create_closing_slide(idx, slide)

def generate_all_tts():
    """Generate TTS audio for all slides"""
    for idx, slide in enumerate(SLIDES):
        output = f"{WORK_DIR}/audio_{idx:02d}.mp3"
        generate_tts(slide["tts"], output)

        # Get actual audio duration and update slide
        duration = get_audio_duration(output)
        SLIDES[idx]["actual_duration"] = max(duration + 1.0, slide["duration"])  # Add 1 second buffer

def combine_audio():
    """Combine all audio files with silence gaps"""
    combined = AudioSegment.empty()

    for idx, slide in enumerate(SLIDES):
        audio_file = f"{WORK_DIR}/audio_{idx:02d}.mp3"
        audio = AudioSegment.from_mp3(audio_file)

        # Add audio
        combined += audio

        # Add silence to match duration
        actual_duration = slide.get("actual_duration", slide["duration"])
        silence_duration = (actual_duration * 1000) - len(audio)
        if silence_duration > 0:
            combined += AudioSegment.silent(duration=int(silence_duration))

    output = f"{WORK_DIR}/combined_audio.mp3"
    combined.export(output, format="mp3")
    print(f"Combined audio: {output}")
    return output

def create_video():
    """Create final video with all slides and audio"""
    # Build input arguments
    inputs = []
    filter_parts = []

    for idx, slide in enumerate(SLIDES):
        duration = slide.get("actual_duration", slide["duration"])
        inputs.append(f'-loop 1 -t {duration} -i "{WORK_DIR}/frame_{idx:02d}.png"')
        filter_parts.append(f'[{idx}:v]scale={WIDTH}:{HEIGHT},setsar=1,format=yuv420p[v{idx}]')

    # Build xfade chain
    xfade_parts = []
    offset = 0
    fade_duration = 0.5

    for idx in range(len(SLIDES) - 1):
        current_duration = SLIDES[idx].get("actual_duration", SLIDES[idx]["duration"])
        offset = offset + current_duration - fade_duration if idx > 0 else current_duration - fade_duration

        if idx == 0:
            xfade_parts.append(f'[v0][v1]xfade=transition=fade:duration={fade_duration}:offset={offset:.1f}[f1]')
        else:
            xfade_parts.append(f'[f{idx}][v{idx+1}]xfade=transition=fade:duration={fade_duration}:offset={offset:.1f}[f{idx+1}]')

    # Final output label
    final_label = f'f{len(SLIDES)-1}'

    # Build filter complex
    filter_complex = '; '.join(filter_parts) + '; ' + '; '.join(xfade_parts) + f',format=yuv420p[vout]'

    # Build command
    input_str = ' '.join(inputs)
    audio_file = f"{WORK_DIR}/combined_audio.mp3"

    cmd = f'''ffmpeg -y {input_str} -i "{audio_file}" \
        -filter_complex "{filter_complex}" \
        -map "[vout]" -map {len(SLIDES)}:a \
        -c:v libx264 -preset medium -crf 20 \
        -c:a aac -b:a 192k \
        -r 30 -pix_fmt yuv420p \
        -shortest \
        "{OUTPUT_VIDEO}"'''

    print("Creating video...")
    print(f"Command length: {len(cmd)} chars")

    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error: {result.stderr}")
        # Try simpler approach
        create_video_simple()
    else:
        print(f"Video created: {OUTPUT_VIDEO}")

def create_video_simple():
    """Create video using simpler concat approach"""
    print("Using simpler concat approach...")

    # Create individual video clips with audio
    clips = []
    for idx, slide in enumerate(SLIDES):
        duration = slide.get("actual_duration", slide["duration"])
        frame = f"{WORK_DIR}/frame_{idx:02d}.png"
        audio = f"{WORK_DIR}/audio_{idx:02d}.mp3"
        clip = f"{WORK_DIR}/clip_{idx:02d}.mp4"

        cmd = f'''ffmpeg -y -loop 1 -t {duration} -i "{frame}" -i "{audio}" \
            -c:v libx264 -preset fast -crf 22 \
            -c:a aac -b:a 128k \
            -vf "scale={WIDTH}:{HEIGHT},format=yuv420p" \
            -shortest -r 30 \
            "{clip}" 2>/dev/null'''
        subprocess.run(cmd, shell=True, check=True)
        clips.append(clip)
        print(f"Created clip {idx}")

    # Create concat list
    concat_file = f"{WORK_DIR}/concat.txt"
    with open(concat_file, 'w') as f:
        for clip in clips:
            f.write(f"file '{clip}'\n")

    # Concat all clips
    cmd = f'''ffmpeg -y -f concat -safe 0 -i "{concat_file}" \
        -c:v libx264 -preset medium -crf 20 \
        -c:a aac -b:a 192k \
        "{OUTPUT_VIDEO}" 2>/dev/null'''
    subprocess.run(cmd, shell=True, check=True)
    print(f"Video created: {OUTPUT_VIDEO}")

def main():
    print("=" * 50)
    print("TSN-Map Demo Video Creator")
    print("=" * 50)

    # Create working directory
    create_work_dir()

    # Create all slides
    print("\n[1/4] Creating slides...")
    create_all_slides()

    # Generate TTS
    print("\n[2/4] Generating TTS narration...")
    generate_all_tts()

    # Combine audio
    print("\n[3/4] Combining audio...")
    combine_audio()

    # Create video
    print("\n[4/4] Creating video...")
    create_video_simple()

    # Cleanup
    print("\nCleaning up...")
    # shutil.rmtree(WORK_DIR)  # Keep for debugging

    print("\n" + "=" * 50)
    print("Done!")
    print(f"Output: {OUTPUT_VIDEO}")
    print("=" * 50)

    # Show file info
    if os.path.exists(OUTPUT_VIDEO):
        size = os.path.getsize(OUTPUT_VIDEO) / (1024 * 1024)
        print(f"File size: {size:.1f} MB")

if __name__ == "__main__":
    main()
