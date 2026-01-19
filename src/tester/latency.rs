//! Rust-based latency tester using raw sockets
//! Inspired by https://github.com/tsnlab/tsn-sdk

use std::io::{self, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Packet header for latency measurement
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct LatencyPacket {
    magic: u32,        // 0x4C415459 = "LATY"
    seq: u32,          // Sequence number
    op: u8,            // Operation: 0=ping, 1=pong
    _pad: [u8; 3],
    tx_sec: u64,       // TX timestamp seconds
    tx_nsec: u32,      // TX timestamp nanoseconds
    rx_sec: u64,       // RX timestamp seconds (filled by receiver)
    rx_nsec: u32,      // RX timestamp nanoseconds
}

const LATENCY_MAGIC: u32 = 0x4C415459;
const LATENCY_PORT: u16 = 7878;
const OP_PING: u8 = 0;
const OP_PONG: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyResult {
    pub seq: u32,
    pub success: bool,
    pub rtt_us: f64,      // Round-trip time in microseconds
    pub tx_time: u64,     // TX timestamp (ns since epoch)
    pub rx_time: u64,     // RX timestamp (ns since epoch)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub count: u32,
    pub success_count: u32,
    pub min_us: f64,
    pub max_us: f64,
    pub avg_us: f64,
    pub jitter_us: f64,   // Standard deviation
    pub loss_percent: f64,
}

pub struct LatencyTester {
    socket: UdpSocket,
    target: SocketAddr,
    packet_size: usize,
}

impl LatencyTester {
    /// Create a new latency tester
    pub fn new(target_ip: IpAddr, port: Option<u16>) -> io::Result<Self> {
        let port = port.unwrap_or(LATENCY_PORT);
        let target = SocketAddr::new(target_ip, port);

        // Bind to any available port
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_secs(2)))?;
        socket.set_write_timeout(Some(Duration::from_secs(1)))?;

        Ok(Self {
            socket,
            target,
            packet_size: std::mem::size_of::<LatencyPacket>(),
        })
    }

    /// Run latency test
    pub fn run(&self, count: u32, interval_ms: u32) -> Vec<LatencyResult> {
        let mut results = Vec::with_capacity(count as usize);
        let interval = Duration::from_millis(interval_ms as u64);

        for seq in 0..count {
            let result = self.ping(seq);
            results.push(result);

            if seq < count - 1 {
                std::thread::sleep(interval);
            }
        }

        results
    }

    /// Send a single ping and wait for pong
    fn ping(&self, seq: u32) -> LatencyResult {
        let now = Instant::now();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();

        // Create ping packet
        let packet = LatencyPacket {
            magic: LATENCY_MAGIC,
            seq,
            op: OP_PING,
            _pad: [0; 3],
            tx_sec: timestamp.as_secs(),
            tx_nsec: timestamp.subsec_nanos(),
            rx_sec: 0,
            rx_nsec: 0,
        };

        // Send packet
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &packet as *const LatencyPacket as *const u8,
                self.packet_size,
            )
        };

        if let Err(_) = self.socket.send_to(bytes, self.target) {
            return LatencyResult {
                seq,
                success: false,
                rtt_us: 0.0,
                tx_time: timestamp.as_nanos() as u64,
                rx_time: 0,
            };
        }

        // Wait for pong
        let mut recv_buf = [0u8; 1024];
        match self.socket.recv_from(&mut recv_buf) {
            Ok((len, _addr)) => {
                let rtt = now.elapsed();

                if len >= self.packet_size {
                    let recv_packet = unsafe {
                        std::ptr::read(recv_buf.as_ptr() as *const LatencyPacket)
                    };

                    if recv_packet.magic == LATENCY_MAGIC && recv_packet.seq == seq && recv_packet.op == OP_PONG {
                        let rx_time = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap();

                        return LatencyResult {
                            seq,
                            success: true,
                            rtt_us: rtt.as_secs_f64() * 1_000_000.0,
                            tx_time: timestamp.as_nanos() as u64,
                            rx_time: rx_time.as_nanos() as u64,
                        };
                    }
                }

                // Invalid response
                LatencyResult {
                    seq,
                    success: false,
                    rtt_us: 0.0,
                    tx_time: timestamp.as_nanos() as u64,
                    rx_time: 0,
                }
            }
            Err(_) => {
                // Timeout or error
                LatencyResult {
                    seq,
                    success: false,
                    rtt_us: 0.0,
                    tx_time: timestamp.as_nanos() as u64,
                    rx_time: 0,
                }
            }
        }
    }

    /// Calculate statistics from results
    pub fn calculate_stats(results: &[LatencyResult]) -> LatencyStats {
        let success_results: Vec<_> = results.iter().filter(|r| r.success).collect();
        let count = results.len() as u32;
        let success_count = success_results.len() as u32;

        if success_results.is_empty() {
            return LatencyStats {
                count,
                success_count: 0,
                min_us: 0.0,
                max_us: 0.0,
                avg_us: 0.0,
                jitter_us: 0.0,
                loss_percent: 100.0,
            };
        }

        let rtts: Vec<f64> = success_results.iter().map(|r| r.rtt_us).collect();
        let min = rtts.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = rtts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = rtts.iter().sum();
        let avg = sum / rtts.len() as f64;

        // Calculate jitter (standard deviation)
        let variance: f64 = rtts.iter().map(|&x| (x - avg).powi(2)).sum::<f64>() / rtts.len() as f64;
        let jitter = variance.sqrt();

        let loss_percent = ((count - success_count) as f64 / count as f64) * 100.0;

        LatencyStats {
            count,
            success_count,
            min_us: min,
            max_us: max,
            avg_us: avg,
            jitter_us: jitter,
            loss_percent,
        }
    }
}

/// Latency test server (responder)
pub struct LatencyServer {
    socket: UdpSocket,
}

impl LatencyServer {
    pub fn new(bind_addr: &str, port: Option<u16>) -> io::Result<Self> {
        let port = port.unwrap_or(LATENCY_PORT);
        let socket = UdpSocket::bind(format!("{}:{}", bind_addr, port))?;
        socket.set_read_timeout(Some(Duration::from_secs(1)))?;

        Ok(Self { socket })
    }

    /// Run server for specified duration (0 = forever)
    pub fn run(&self, duration_secs: u64) -> io::Result<u64> {
        let start = Instant::now();
        let mut packets_handled = 0u64;
        let packet_size = std::mem::size_of::<LatencyPacket>();

        loop {
            if duration_secs > 0 && start.elapsed().as_secs() >= duration_secs {
                break;
            }

            let mut recv_buf = [0u8; 1024];
            match self.socket.recv_from(&mut recv_buf) {
                Ok((len, src_addr)) => {
                    if len >= packet_size {
                        let mut packet = unsafe {
                            std::ptr::read(recv_buf.as_ptr() as *const LatencyPacket)
                        };

                        if packet.magic == LATENCY_MAGIC && packet.op == OP_PING {
                            // Fill RX timestamp
                            let rx_time = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap();
                            packet.rx_sec = rx_time.as_secs();
                            packet.rx_nsec = rx_time.subsec_nanos();
                            packet.op = OP_PONG;

                            // Send pong
                            let bytes = unsafe {
                                std::slice::from_raw_parts(
                                    &packet as *const LatencyPacket as *const u8,
                                    packet_size,
                                )
                            };
                            let _ = self.socket.send_to(bytes, src_addr);
                            packets_handled += 1;
                        }
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(packets_handled)
    }
}

/// ICMP-based ping (requires root privileges)
pub mod icmp {
    use super::*;
    use std::net::Ipv4Addr;

    const ICMP_ECHO_REQUEST: u8 = 8;
    const ICMP_ECHO_REPLY: u8 = 0;

    #[repr(C, packed)]
    struct IcmpHeader {
        icmp_type: u8,
        code: u8,
        checksum: u16,
        id: u16,
        seq: u16,
    }

    fn checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        let mut i = 0;

        while i < data.len() - 1 {
            sum += ((data[i] as u32) << 8) | (data[i + 1] as u32);
            i += 2;
        }

        if i < data.len() {
            sum += (data[i] as u32) << 8;
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !sum as u16
    }

    /// Send ICMP echo request and measure RTT
    pub fn ping_icmp(target: Ipv4Addr, seq: u16, timeout_ms: u64) -> Option<f64> {
        use socket2::{Socket, Domain, Type, Protocol};
        use std::mem::MaybeUninit;

        let socket = match Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4)) {
            Ok(s) => s,
            Err(_) => return None,
        };

        socket.set_read_timeout(Some(Duration::from_millis(timeout_ms))).ok()?;

        let id = std::process::id() as u16;

        // Build ICMP packet
        let mut packet = vec![0u8; 64];
        let header = IcmpHeader {
            icmp_type: ICMP_ECHO_REQUEST,
            code: 0,
            checksum: 0,
            id: id.to_be(),
            seq: seq.to_be(),
        };

        unsafe {
            std::ptr::copy_nonoverlapping(
                &header as *const IcmpHeader as *const u8,
                packet.as_mut_ptr(),
                std::mem::size_of::<IcmpHeader>(),
            );
        }

        // Fill payload with timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        packet[8..16].copy_from_slice(&timestamp.to_be_bytes());

        // Calculate checksum
        let cksum = checksum(&packet);
        packet[2..4].copy_from_slice(&cksum.to_be_bytes());

        let dest = SocketAddr::new(IpAddr::V4(target), 0);
        let start = Instant::now();

        socket.send_to(&packet, &dest.into()).ok()?;

        // Receive reply (socket2 0.5 requires MaybeUninit buffer)
        let mut recv_buf: [MaybeUninit<u8>; 1024] = unsafe { MaybeUninit::uninit().assume_init() };
        match socket.recv(&mut recv_buf) {
            Ok(len) => {
                if len >= 28 {
                    // Convert MaybeUninit to slice safely
                    let buf: &[u8] = unsafe {
                        std::slice::from_raw_parts(recv_buf.as_ptr() as *const u8, len)
                    };
                    // IP header (20) + ICMP header (8)
                    let icmp_type = buf[20];
                    let recv_id = u16::from_be_bytes([buf[24], buf[25]]);
                    let recv_seq = u16::from_be_bytes([buf[26], buf[27]]);

                    if icmp_type == ICMP_ECHO_REPLY && recv_id == id && recv_seq == seq {
                        return Some(start.elapsed().as_secs_f64() * 1000.0); // ms
                    }
                }
                None
            }
            Err(_) => None,
        }
    }

    /// Run ICMP ping test (requires root)
    pub fn run_icmp_test(target: Ipv4Addr, count: u32, interval_ms: u32) -> (Vec<LatencyResult>, LatencyStats) {
        let mut results = Vec::with_capacity(count as usize);

        for seq in 0..count {
            let tx_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;

            let result = match ping_icmp(target, seq as u16, 2000) {
                Some(rtt_ms) => LatencyResult {
                    seq,
                    success: true,
                    rtt_us: rtt_ms * 1000.0,
                    tx_time,
                    rx_time: tx_time + (rtt_ms * 1_000_000.0) as u64,
                },
                None => LatencyResult {
                    seq,
                    success: false,
                    rtt_us: 0.0,
                    tx_time,
                    rx_time: 0,
                },
            };

            results.push(result);

            if seq < count - 1 {
                std::thread::sleep(Duration::from_millis(interval_ms as u64));
            }
        }

        let stats = LatencyTester::calculate_stats(&results);
        (results, stats)
    }
}
