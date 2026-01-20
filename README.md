# TSN-Map

<p align="center">
  <img src="icon.png" alt="TSN-Map Logo" height="120">
</p>

<p align="center">
  <strong>Network Visualization and Analysis Tool</strong><br>
  Real-time packet capture with interactive topology visualization
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Axum-000000?style=flat-square" alt="Axum">
  <img src="https://img.shields.io/badge/D3.js-F9A03C?style=flat-square&logo=d3.js&logoColor=white" alt="D3.js">
  <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License">
</p>

---

## Features

- **Real-time Packet Capture** - Live packet capture with libpcap, protocol detection, and color-coded display
- **Network Topology** - Interactive D3.js force-directed graph with automatic device discovery
- **Traffic Statistics** - Real-time charts for bandwidth, protocol distribution, and packet rates
- **IO Graph** - Wireshark-style time vs packets/bytes/bits graph with adjustable intervals (1ms ~ 10s)
- **Tester** - Integrated testing tools (Ping latency test + Packet Generator)
- **TSN Support** - CBS (Credit-Based Shaper) and TAS (Time-Aware Shaper) configuration
- **PCAP Support** - Save/load/upload/download pcap files for offline analysis
- **Display Filter** - Wireshark-style filter syntax (tcp, udp, ip.addr==x.x.x.x, port==80, etc.)
- **Vendor Detection** - OUI-based MAC vendor identification

## Quick Start

### Prerequisites

```bash
# Ubuntu/Debian
sudo apt install libpcap-dev build-essential

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build & Run

```bash
# Clone
git clone https://github.com/hwkim3330/tsn-map.git
cd tsn-map

# Build
cargo build --release

# Easy launch (recommended)
./start-web.sh
# Opens browser automatically at http://localhost:8080

# Or manual setup:
# Set capabilities once (requires sudo password)
sudo setcap 'cap_net_raw,cap_net_admin+eip' ./target/release/tsn-map

# Run (no sudo needed after setcap)
./target/release/tsn-map -i eth0 -p 8080
# Then open http://localhost:8080 in your browser
```

### Options

```
-i, --interface <NAME>    Network interface [default: enp11s0]
-p, --port <PORT>         Web server port [default: 8080]
    --promiscuous         Enable promiscuous mode
    --buffer-size <MB>    Capture buffer size [default: 64]
```

## Architecture

```
tsn-map/
├── src/
│   ├── main.rs          # Axum web server + routes
│   ├── capture/         # Packet capture (libpcap/pcap crate)
│   ├── protocols/       # Protocol parsing (Ethernet, IP, TCP, UDP, LLDP, PTP, etc.)
│   ├── topology/        # Network graph (petgraph) + OUI lookup + LLDP discovery
│   ├── tester/          # Ping (ICMP) + Throughput (UDP) + Packet Generator
│   └── api/             # REST API + SSE handlers
└── web/
    ├── index.html       # Single-page application
    ├── js/app.js        # Frontend logic + Chart.js visualization
    └── css/style.css    # Hybrid theme (white header + dark content)
```

## Filter Syntax

```bash
# Protocols
tcp, udp, icmp, arp, dns, http, https, lldp, ptp

# IP address
ip.addr==192.168.1.1
ip.src==10.0.0.1
ip.dst==10.0.0.1

# Port
port==443
port==80

# Combined
tcp && port==443
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/status` | Server status and capture info |
| POST | `/api/capture/start` | Start packet capture |
| POST | `/api/capture/stop` | Stop packet capture |
| GET | `/api/packets` | Get captured packets (paginated) |
| GET | `/api/packets/stream` | SSE real-time packet stream |
| GET | `/api/topology` | Network topology (nodes + links) |
| POST | `/api/topology/scan` | Scan network topology |
| GET | `/api/iograph` | IO graph data |
| GET | `/api/tsn/flows` | TSN flow information |
| GET | `/api/tsn/streams` | TSN stream information |
| GET | `/api/interfaces` | Available network interfaces |
| POST | `/api/interface/set` | Set capture interface |
| POST | `/api/pcap/save` | Save capture to file |
| POST | `/api/pcap/load` | Load capture from file |
| POST | `/api/pcap/download` | Download pcap file |
| POST | `/api/pcap/upload` | Upload pcap file |
| POST | `/api/test/ping` | Run ping test |
| GET | `/api/test/ping/stream` | SSE streaming ping test |
| POST | `/api/test/throughput` | Run throughput test |
| GET | `/api/test/throughput/stream` | SSE streaming throughput test |
| GET | `/api/test/pktgen/stream` | Packet generator stream |
| POST | `/api/tsn/cbs` | Configure CBS (Credit-Based Shaper) |
| POST | `/api/tsn/tas` | Configure TAS (Time-Aware Shaper) |

## Tech Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust, Axum, tokio |
| Packet Capture | pcap crate (libpcap) |
| Network Graph | petgraph |
| Frontend | Vanilla JS, D3.js, Chart.js |
| Streaming | Server-Sent Events (SSE) |

## License

MIT License - see [LICENSE](LICENSE)

## Author

**hwkim3330** - [GitHub](https://github.com/hwkim3330)
