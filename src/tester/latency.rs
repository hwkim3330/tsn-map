//! UDP-based application-layer latency tester
//!
//! Note: This measures application-layer RTT, NOT TSN/PHY-level latency.
//! For TSN latency measurement, use HW timestamps (SO_TIMESTAMPING) or PTP PHC.

use std::io::{self, ErrorKind};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Simple ping packet for RTT measurement
/// Uses little-endian byte order for cross-platform compatibility
#[repr(C)]
#[derive(Clone, Copy)]
struct LatencyPacket {
    magic: [u8; 4],    // "LATY" magic bytes
    seq: u32,          // Sequence number (LE)
    op: u8,            // Operation: 0=ping, 1=pong
    _pad: [u8; 3],
}

const LATENCY_MAGIC: [u8; 4] = *b"LATY";
const LATENCY_PORT: u16 = 7878;
const PACKET_SIZE: usize = std::mem::size_of::<LatencyPacket>();
const OP_PING: u8 = 0;
const OP_PONG: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyResult {
    pub seq: u32,
    pub success: bool,
    pub rtt_us: f64,      // Round-trip time in microseconds (Instant-based)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub count: u32,
    pub success_count: u32,
    pub min_us: f64,
    pub max_us: f64,
    pub avg_us: f64,
    pub jitter_us: f64,   // Standard deviation (note: not RFC 3393 IPDV)
    pub loss_percent: f64,
}

pub struct LatencyTester {
    socket: UdpSocket,
    target: SocketAddr,
}

impl LatencyTester {
    /// Create a new latency tester
    pub fn new(target_ip: IpAddr, port: Option<u16>) -> io::Result<Self> {
        let port = port.unwrap_or(LATENCY_PORT);
        let target = SocketAddr::new(target_ip, port);

        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_secs(2)))?;
        socket.set_write_timeout(Some(Duration::from_secs(1)))?;

        Ok(Self { socket, target })
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

        // Create ping packet with explicit byte order
        let packet = LatencyPacket {
            magic: LATENCY_MAGIC,
            seq: seq.to_le(),
            op: OP_PING,
            _pad: [0; 3],
        };

        // Safe serialization
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                &packet as *const LatencyPacket as *const u8,
                PACKET_SIZE,
            )
        };

        if self.socket.send_to(bytes, self.target).is_err() {
            return LatencyResult {
                seq,
                success: false,
                rtt_us: 0.0,
            };
        }

        // Receive with exact buffer size
        let mut recv_buf = [0u8; PACKET_SIZE];
        match self.socket.recv_from(&mut recv_buf) {
            Ok((len, addr)) => {
                // Verify source address
                if addr != self.target {
                    return LatencyResult {
                        seq,
                        success: false,
                        rtt_us: 0.0,
                    };
                }

                let rtt = now.elapsed();

                if len >= PACKET_SIZE {
                    // Parse with explicit byte order
                    let recv_magic = &recv_buf[0..4];
                    let recv_seq = u32::from_le_bytes([recv_buf[4], recv_buf[5], recv_buf[6], recv_buf[7]]);
                    let recv_op = recv_buf[8];

                    if recv_magic == &LATENCY_MAGIC && recv_seq == seq && recv_op == OP_PONG {
                        return LatencyResult {
                            seq,
                            success: true,
                            rtt_us: rtt.as_secs_f64() * 1_000_000.0,
                        };
                    }
                }

                LatencyResult {
                    seq,
                    success: false,
                    rtt_us: 0.0,
                }
            }
            Err(_) => LatencyResult {
                seq,
                success: false,
                rtt_us: 0.0,
            },
        }
    }

    /// Calculate statistics from results
    /// Note: jitter is standard deviation, not RFC 3393 IPDV
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

        // Standard deviation (not RFC 3393 IPDV)
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

        loop {
            if duration_secs > 0 && start.elapsed().as_secs() >= duration_secs {
                break;
            }

            let mut recv_buf = [0u8; PACKET_SIZE];
            match self.socket.recv_from(&mut recv_buf) {
                Ok((len, src_addr)) => {
                    if len >= PACKET_SIZE {
                        let recv_magic = &recv_buf[0..4];
                        let recv_op = recv_buf[8];

                        if recv_magic == &LATENCY_MAGIC && recv_op == OP_PING {
                            // Create pong response
                            let mut response = recv_buf;
                            response[8] = OP_PONG;

                            let _ = self.socket.send_to(&response, src_addr);
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

    fn checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        let mut i = 0;

        while i + 1 < data.len() {
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
        // Type (1) + Code (1) + Checksum (2) + ID (2) + Seq (2) = 8 bytes header
        packet[0] = ICMP_ECHO_REQUEST;
        packet[1] = 0; // code
        packet[4..6].copy_from_slice(&id.to_be_bytes());
        packet[6..8].copy_from_slice(&seq.to_be_bytes());

        // Calculate checksum
        let cksum = checksum(&packet);
        packet[2..4].copy_from_slice(&cksum.to_be_bytes());

        let dest = SocketAddr::new(IpAddr::V4(target), 0);
        let start = Instant::now();

        socket.send_to(&packet, &dest.into()).ok()?;

        // Receive reply
        let mut recv_buf: [MaybeUninit<u8>; 1024] = unsafe { MaybeUninit::uninit().assume_init() };
        match socket.recv(&mut recv_buf) {
            Ok(len) => {
                if len < 20 {
                    return None;
                }

                let buf: &[u8] = unsafe {
                    std::slice::from_raw_parts(recv_buf.as_ptr() as *const u8, len)
                };

                // Read IP header length (IHL) from first byte
                let ihl = ((buf[0] & 0x0f) * 4) as usize;

                if len < ihl + 8 {
                    return None;
                }

                let icmp_type = buf[ihl];
                let recv_id = u16::from_be_bytes([buf[ihl + 4], buf[ihl + 5]]);
                let recv_seq = u16::from_be_bytes([buf[ihl + 6], buf[ihl + 7]]);

                if icmp_type == ICMP_ECHO_REPLY && recv_id == id && recv_seq == seq {
                    return Some(start.elapsed().as_secs_f64() * 1000.0); // ms
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
            let result = match ping_icmp(target, seq as u16, 2000) {
                Some(rtt_ms) => LatencyResult {
                    seq,
                    success: true,
                    rtt_us: rtt_ms * 1000.0,
                },
                None => LatencyResult {
                    seq,
                    success: false,
                    rtt_us: 0.0,
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
