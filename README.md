# TSN-Map

<p align="center">
  <img src="icon.png" alt="TSN-Map Logo" height="120">
</p>

**Network Visualization and Analysis Tool**

A desktop application combining Wireshark-like packet capture with nmap-style network visualization.

![TSN-Map](https://img.shields.io/badge/TSN--Map-2f81f7?style=for-the-badge)
![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-FFC131?style=for-the-badge&logo=tauri&logoColor=black)
![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)

## Features

- **Packet Capture**: Real-time packet capture with libpcap
- **Network Topology**: Interactive D3.js force-directed graph visualization
- **Host Discovery**: Automatic device discovery with vendor identification (OUI lookup)
- **Statistics**: Real-time traffic charts and protocol distribution
- **Latency Testing**: Ping test with RTT statistics
- **Throughput Testing**: UDP-based bandwidth measurement
- **PCAP Support**: Save/load pcap files

## Screenshots

| Packet Capture | Network Topology |
|:---:|:---:|
| Real-time packet list with protocol coloring | Interactive network graph |

## Installation

### Prerequisites
- Rust 1.70+
- libpcap-dev
- GTK3 + WebKit2GTK (for Tauri)

```bash
# Ubuntu/Debian
sudo apt-get install libpcap-dev libgtk-3-dev libwebkit2gtk-4.1-dev

# Clone
git clone https://github.com/hwkim3330/tsn-map.git
cd tsn-map

# Build & Install
./install.sh
```

## Usage

### Desktop App (Recommended)
```bash
# Run with packet capture (requires root)
sudo ./run.sh

# Or from app menu after install
```

### CLI Only
```bash
# Build
cargo build --release

# Run (requires root for packet capture)
sudo ./target/release/tsn-map -i eth0 -p 8080

# Open browser
xdg-open http://localhost:8080
```

### Options
```
-i, --interface <INTERFACE>  Network interface [default: enp11s0]
-p, --port <PORT>            Web server port [default: 8080]
    --promiscuous            Enable promiscuous mode
    --buffer-size <MB>       Capture buffer size [default: 64]
```

## Architecture

```
tsn-map/
├── src/
│   ├── main.rs              # Axum web server
│   ├── capture/             # Packet capture (libpcap)
│   ├── topology/            # Network topology manager
│   ├── tester/              # Latency/throughput testing
│   └── api/                 # REST API handlers
├── src-tauri/               # Tauri desktop wrapper
└── web/                     # Frontend (HTML/JS/CSS)
```

## Filter Syntax

```
# Protocols
tcp, udp, icmp, arp, dns, http, https

# IP Filters
ip.addr==192.168.1.1
ip.src==10.0.0.1
ip.dst==10.0.0.1

# Port Filters
port==443
```

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/status` | Server status |
| `POST /api/capture/start` | Start capture |
| `POST /api/capture/stop` | Stop capture |
| `GET /api/packets` | Packet list |
| `GET /api/packets/stream` | SSE real-time stream |
| `GET /api/topology` | Network topology |
| `GET /api/interfaces` | Interface list |
| `POST /api/tester/ping` | Ping test |
| `POST /api/tester/throughput` | Throughput test |

## Tech Stack

- **Backend**: Rust + Axum + libpcap
- **Frontend**: Vanilla JS + D3.js + Chart.js
- **Desktop**: Tauri 2.0

## License

MIT License - see [LICENSE](LICENSE)

## Author

hwkim3330 - [GitHub](https://github.com/hwkim3330)
