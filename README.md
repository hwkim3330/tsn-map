# NetMap

<p align="center">
  <img src="web/img/keti-logo.png" alt="KETI Logo" height="60">
</p>

**실시간 네트워크 패킷 캡처 및 시각화 도구**

Wireshark와 nmap의 기능을 결합한 웹 기반 네트워크 분석 도구입니다.

![NetMap](https://img.shields.io/badge/NetMap-2f81f7?style=for-the-badge)
![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)

## Features

### Packet Capture
- Real-time packet capture using libpcap
- TSN-aware packet parsing (PTP, CBS, TAS, FRER)
- PCAP file save/load support
- Packet filtering and search
- Hex dump viewer

### TSN Protocol Analysis
- **PTP (IEEE 1588)**: Sync, Follow_Up, Delay_Req/Resp analysis
- **CBS (IEEE 802.1Qav)**: Credit-Based Shaper traffic classification
- **TAS (IEEE 802.1Qbv)**: Time-Aware Shaper schedule detection
- **FRER (IEEE 802.1CB)**: Frame Replication and Elimination tracking

### Network Topology
- Automatic device discovery from traffic
- MAC/IP address mapping
- Vendor identification (OUI lookup)
- TSN-capable device detection
- PTP role identification (Grandmaster, OrdinaryClock)
- Interactive D3.js force-directed graph

### Visualization
- Real-time packet stream (Server-Sent Events)
- Traffic rate charts
- Protocol distribution
- PTP timing analysis
- Network topology graph

## Installation

### Prerequisites
- Rust 1.70+ with Cargo
- libpcap development libraries
- Linux (tested on Ubuntu 22.04)

```bash
# Install libpcap
sudo apt-get install libpcap-dev

# Clone repository
git clone https://github.com/hwkim3330/tsn-map.git
cd tsn-map

# Build
cargo build --release
```

## Usage

```bash
# Run with default settings (requires root for packet capture)
sudo ./target/release/tsn-map

# Specify interface and port
sudo ./target/release/tsn-map -i enp11s0 -p 8080

# With verbose logging
RUST_LOG=debug sudo ./target/release/tsn-map
```

### Command Line Options
```
Options:
  -i, --interface <INTERFACE>  Network interface [default: enp11s0]
  -p, --port <PORT>            Web server port [default: 8080]
      --promiscuous            Enable promiscuous mode [default: true]
      --buffer-size <MB>       Capture buffer size [default: 64]
  -h, --help                   Print help
```

### Web Interface
Open `http://localhost:8080` in your browser.

## Architecture

```
tsn-map/
├── src/
│   ├── main.rs              # Axum web server
│   ├── capture/             # Packet capture module
│   │   ├── mod.rs           # Capture manager
│   │   ├── packet.rs        # Packet parsing
│   │   └── pcap_handler.rs  # PCAP file handling
│   ├── protocols/           # TSN protocol analyzers
│   │   ├── ptp.rs           # IEEE 1588 PTP
│   │   ├── cbs.rs           # IEEE 802.1Qav CBS
│   │   ├── tas.rs           # IEEE 802.1Qbv TAS
│   │   └── frer.rs          # IEEE 802.1CB FRER
│   ├── topology/            # Network topology
│   │   └── mod.rs           # Topology manager
│   └── api/                 # REST API handlers
│       ├── mod.rs
│       └── handlers.rs
└── web/                     # Frontend
    ├── index.html
    ├── css/style.css
    └── js/app.js
```

## Filter Syntax

```
tcp                    # TCP 패킷만
udp                    # UDP 패킷만
arp                    # ARP 패킷만
icmp                   # ICMP 패킷만
dns                    # DNS 패킷 (포트 53)
http                   # HTTP 패킷 (포트 80)
https                  # HTTPS 패킷 (포트 443)
ip.addr==192.168.1.1   # 특정 IP 주소
ip.src==10.0.0.1       # 출발지 IP
ip.dst==10.0.0.1       # 목적지 IP
port==443              # 특정 포트
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/status` | GET | 서버 상태 |
| `/api/capture/start` | POST | 캡처 시작 |
| `/api/capture/stop` | POST | 캡처 중지 |
| `/api/capture/stats` | GET | 캡처 통계 |
| `/api/packets` | GET | 패킷 목록 |
| `/api/packets/stream` | GET | SSE 실시간 스트림 |
| `/api/topology` | GET | 네트워크 토폴로지 |
| `/api/pcap/upload` | POST | PCAP 파일 업로드 |
| `/api/pcap/download` | POST | PCAP 파일 다운로드 |
| `/api/interfaces` | GET | 인터페이스 목록 |
| `/api/interface/set` | POST | 인터페이스 변경 |

## TSN Protocol Support

### PTP (IEEE 1588)
- Message types: Sync, Follow_Up, Delay_Req, Delay_Resp, Announce
- Both Layer 2 (EtherType 0x88F7) and UDP (ports 319/320)
- Clock identification and grandmaster detection
- Sync interval and offset calculation

### CBS (IEEE 802.1Qav)
- Traffic class mapping from VLAN PCP
- Bandwidth measurement per traffic class
- Burst size tracking

### TAS (IEEE 802.1Qbv)
- Per-queue statistics
- Cycle time detection from inter-arrival times
- Latency measurement

### FRER (IEEE 802.1CB)
- Stream identification
- Duplicate detection
- Sequence number tracking

## References

- [tsn-sdk](https://github.com/tsnlab/tsn-sdk) - TSN SDK reference implementation
- IEEE 802.1Q - Bridges and Bridged Networks
- IEEE 1588 - Precision Time Protocol
- [Wireshark](https://www.wireshark.org/) - Network protocol analyzer
- [nmap](https://nmap.org/) - Network scanner

## License

MIT License - see [LICENSE](LICENSE) for details.

## Author

- hwkim3330 - [GitHub](https://github.com/hwkim3330)
