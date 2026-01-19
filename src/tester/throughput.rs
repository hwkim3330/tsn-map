//! UDP-based application-layer throughput generator
//!
//! Note: This measures application-layer throughput, NOT network/PHY throughput.
//! For accurate network performance testing, use iperf3 or dedicated HW tools.
//!
//! Limitations:
//! - UDP packet loss/reorder not fully tracked
//! - Bandwidth limiting uses sleep() - inaccurate at high rates
//! - Single client only

use std::io::{self, ErrorKind};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Simple packet header for throughput test
/// Uses little-endian byte order
const MAGIC: [u8; 4] = *b"THPT";
const HEADER_SIZE: usize = 16;  // magic(4) + op(1) + pad(3) + seq(8)
const THROUGHPUT_PORT: u16 = 7879;

const OP_DATA: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputResult {
    pub duration_secs: f64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bandwidth_bps: f64,
    pub bandwidth_mbps: f64,
    pub packet_loss_percent: f64,
    pub avg_packet_size: f64,
}

pub struct ThroughputTester {
    target: SocketAddr,
    packet_size: usize,
    bandwidth_limit_bps: Option<u64>,
}

impl ThroughputTester {
    pub fn new(target_ip: IpAddr, port: Option<u16>) -> Self {
        let port = port.unwrap_or(THROUGHPUT_PORT);
        Self {
            target: SocketAddr::new(target_ip, port),
            packet_size: 1400,
            bandwidth_limit_bps: None,
        }
    }

    pub fn with_packet_size(mut self, size: usize) -> Self {
        self.packet_size = size.max(64).min(65000);
        self
    }

    pub fn with_bandwidth_limit(mut self, bps: u64) -> Self {
        self.bandwidth_limit_bps = Some(bps);
        self
    }

    /// Run throughput test as client (sender)
    pub fn run_client(&self, duration_secs: u32) -> io::Result<ThroughputResult> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_write_timeout(Some(Duration::from_secs(1)))?;

        let mut packet_buf = vec![0u8; self.packet_size];

        // Set magic header
        packet_buf[0..4].copy_from_slice(&MAGIC);
        packet_buf[4] = OP_DATA;

        // Fill payload with pattern
        for i in HEADER_SIZE..self.packet_size {
            packet_buf[i] = (i & 0xFF) as u8;
        }

        let start = Instant::now();
        let duration = Duration::from_secs(duration_secs as u64);
        let mut seq: u64 = 0;
        let mut bytes_sent: u64 = 0;

        // Bandwidth limiting (note: sleep-based, not accurate at high rates)
        let packet_delay = if let Some(bps) = self.bandwidth_limit_bps {
            let bits_per_packet = (self.packet_size * 8) as f64;
            Duration::from_secs_f64(bits_per_packet / bps as f64)
        } else {
            Duration::from_micros(10)  // Small delay to prevent CPU spin
        };

        while start.elapsed() < duration {
            // Update sequence number (little-endian)
            packet_buf[8..16].copy_from_slice(&seq.to_le_bytes());

            match socket.send_to(&packet_buf, self.target) {
                Ok(sent) => {
                    bytes_sent += sent as u64;
                    seq += 1;
                }
                Err(_) => continue,
            }

            if !packet_delay.is_zero() {
                std::thread::sleep(packet_delay);
            }
        }

        let elapsed = start.elapsed();
        let duration_secs = elapsed.as_secs_f64();
        let bandwidth_bps = (bytes_sent as f64 * 8.0) / duration_secs;

        Ok(ThroughputResult {
            duration_secs,
            bytes_sent,
            bytes_received: bytes_sent, // Client doesn't know actual received
            packets_sent: seq,
            packets_received: seq,
            bandwidth_bps,
            bandwidth_mbps: bandwidth_bps / 1_000_000.0,
            packet_loss_percent: 0.0,  // Unknown without server feedback
            avg_packet_size: if seq > 0 { bytes_sent as f64 / seq as f64 } else { 0.0 },
        })
    }
}

/// Throughput test server (receiver)
pub struct ThroughputServer {
    socket: UdpSocket,
    running: Arc<AtomicBool>,
}

impl ThroughputServer {
    pub fn new(bind_addr: &str, port: Option<u16>) -> io::Result<Self> {
        let port = port.unwrap_or(THROUGHPUT_PORT);
        let socket = UdpSocket::bind(format!("{}:{}", bind_addr, port))?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        Ok(Self {
            socket,
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    pub fn run(&self) -> io::Result<ThroughputResult> {
        self.running.store(true, Ordering::SeqCst);

        let mut recv_buf = [0u8; 65536];
        let mut bytes_received: u64 = 0;
        let mut packets_received: u64 = 0;
        let test_start = Instant::now();
        let timeout = Duration::from_secs(60);

        while self.running.load(Ordering::SeqCst) && test_start.elapsed() < timeout {
            match self.socket.recv_from(&mut recv_buf) {
                Ok((len, _addr)) => {
                    if len >= HEADER_SIZE && &recv_buf[0..4] == &MAGIC {
                        bytes_received += len as u64;
                        packets_received += 1;
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                    // Check if we should stop after no data for a while
                    if packets_received > 0 && test_start.elapsed() > Duration::from_secs(5) {
                        break;
                    }
                    continue;
                }
                Err(_) => continue,
            }
        }

        self.running.store(false, Ordering::SeqCst);

        let duration_secs = test_start.elapsed().as_secs_f64();
        let bandwidth_bps = if duration_secs > 0.0 {
            (bytes_received as f64 * 8.0) / duration_secs
        } else {
            0.0
        };

        Ok(ThroughputResult {
            duration_secs,
            bytes_sent: 0,
            bytes_received,
            packets_sent: 0,
            packets_received,
            bandwidth_bps,
            bandwidth_mbps: bandwidth_bps / 1_000_000.0,
            packet_loss_percent: 0.0,  // Cannot determine without seq tracking
            avg_packet_size: if packets_received > 0 {
                bytes_received as f64 / packets_received as f64
            } else {
                0.0
            },
        })
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
