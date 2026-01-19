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
- **Latency Testing** - ICMP ping with live RTT graph (SSE streaming)
- **Throughput Testing** - UDP bandwidth measurement with real-time Mbps chart
- **PCAP Support** - Save and load pcap files for offline analysis
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

# Set capabilities (one-time, then no sudo needed)
sudo setcap 'cap_net_raw,cap_net_admin+eip' ./target/release/tsn-map

# Run (no sudo required after setcap)
./target/release/tsn-map -i eth0

# Or use the launcher script
./start-web.sh
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
│   ├── protocols/       # Protocol parsing (Ethernet, IP, TCP, UDP, etc.)
│   ├── topology/        # Network graph (petgraph) + OUI lookup
│   ├── tester/          # Latency (ICMP) + Throughput (UDP) testers
│   └── api/             # REST API + SSE handlers
└── web/
    ├── index.html       # Single-page application
    ├── js/app.js        # Frontend logic
    └── css/style.css    # Dark theme styling
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
| GET | `/api/stats` | Traffic statistics |
| GET | `/api/interfaces` | Available network interfaces |
| POST | `/api/test/ping` | Run ping test |
| GET | `/api/test/ping/stream` | SSE streaming ping test |
| POST | `/api/test/throughput` | Run throughput test |
| GET | `/api/test/throughput/stream` | SSE streaming throughput test |

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
