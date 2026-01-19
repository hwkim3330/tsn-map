//! Rust-based throughput tester using raw sockets
//! Inspired by https://github.com/tsnlab/tsn-sdk

use std::io::{self, ErrorKind};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Throughput test packet header
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct ThroughputPacket {
    magic: u32,          // 0x54485054 = "THPT"
    op: u8,              // 0=start, 1=data, 2=end, 3=result
    _pad: [u8; 3],
    seq: u64,            // Sequence number
    timestamp: u64,      // Timestamp in nanoseconds
    total_bytes: u64,    // Total bytes (for result)
    total_packets: u64,  // Total packets (for result)
}

const THROUGHPUT_MAGIC: u32 = 0x54485054;
const THROUGHPUT_PORT: u16 = 7879;

const OP_START: u8 = 0;
const OP_DATA: u8 = 1;
const OP_END: u8 = 2;
const OP_RESULT: u8 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputResult {
    pub duration_secs: f64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bandwidth_bps: f64,      // Bits per second
    pub bandwidth_mbps: f64,     // Megabits per second
    pub packet_loss_percent: f64,
    pub avg_packet_size: f64,
}

pub struct ThroughputTester {
    target: SocketAddr,
    packet_size: usize,
    bandwidth_limit_bps: Option<u64>,
}

impl ThroughputTester {
    /// Create a new throughput tester
    pub fn new(target_ip: IpAddr, port: Option<u16>) -> Self {
        let port = port.unwrap_or(THROUGHPUT_PORT);
        Self {
            target: SocketAddr::new(target_ip, port),
            packet_size: 1400, // Default MTU-safe size
            bandwidth_limit_bps: None,
        }
    }

    /// Set packet size
    pub fn with_packet_size(mut self, size: usize) -> Self {
        self.packet_size = size.max(64).min(65000);
        self
    }

    /// Set bandwidth limit in bits per second
    pub fn with_bandwidth_limit(mut self, bps: u64) -> Self {
        self.bandwidth_limit_bps = Some(bps);
        self
    }

    /// Run throughput test as client (sender)
    pub fn run_client(&self, duration_secs: u32) -> io::Result<ThroughputResult> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_write_timeout(Some(Duration::from_secs(1)))?;

        let header_size = std::mem::size_of::<ThroughputPacket>();
        let mut packet_buf = vec![0u8; self.packet_size];

        // Fill with pattern
        for i in header_size..self.packet_size {
            packet_buf[i] = (i & 0xFF) as u8;
        }

        let start = Instant::now();
        let duration = Duration::from_secs(duration_secs as u64);
        let mut seq: u64 = 0;
        let mut bytes_sent: u64 = 0;

        // Calculate inter-packet delay for bandwidth limiting
        let packet_delay = if let Some(bps) = self.bandwidth_limit_bps {
            let bits_per_packet = (self.packet_size * 8) as f64;
            Duration::from_secs_f64(bits_per_packet / bps as f64)
        } else {
            Duration::ZERO
        };

        // Send start packet
        let start_packet = ThroughputPacket {
            magic: THROUGHPUT_MAGIC,
            op: OP_START,
            _pad: [0; 3],
            seq: 0,
            timestamp: start.elapsed().as_nanos() as u64,
            total_bytes: 0,
            total_packets: 0,
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                &start_packet as *const ThroughputPacket as *const u8,
                packet_buf.as_mut_ptr(),
                header_size,
            );
        }
        socket.send_to(&packet_buf[..header_size], self.target)?;

        // Send data packets
        while start.elapsed() < duration {
            let packet = ThroughputPacket {
                magic: THROUGHPUT_MAGIC,
                op: OP_DATA,
                _pad: [0; 3],
                seq,
                timestamp: start.elapsed().as_nanos() as u64,
                total_bytes: 0,
                total_packets: 0,
            };

            unsafe {
                std::ptr::copy_nonoverlapping(
                    &packet as *const ThroughputPacket as *const u8,
                    packet_buf.as_mut_ptr(),
                    header_size,
                );
            }

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

        // Send end packet
        let end_packet = ThroughputPacket {
            magic: THROUGHPUT_MAGIC,
            op: OP_END,
            _pad: [0; 3],
            seq,
            timestamp: elapsed.as_nanos() as u64,
            total_bytes: bytes_sent,
            total_packets: seq,
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                &end_packet as *const ThroughputPacket as *const u8,
                packet_buf.as_mut_ptr(),
                header_size,
            );
        }
        socket.send_to(&packet_buf[..header_size], self.target)?;

        // Wait for result from server
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut recv_buf = [0u8; 1024];

        let (packets_received, bytes_received) = match socket.recv_from(&mut recv_buf) {
            Ok((len, _)) if len >= header_size => {
                let result_packet = unsafe {
                    std::ptr::read(recv_buf.as_ptr() as *const ThroughputPacket)
                };
                if result_packet.magic == THROUGHPUT_MAGIC && result_packet.op == OP_RESULT {
                    (result_packet.total_packets, result_packet.total_bytes)
                } else {
                    (seq, bytes_sent) // Assume all received
                }
            }
            _ => (seq, bytes_sent),
        };

        let duration_secs = elapsed.as_secs_f64();
        let bandwidth_bps = (bytes_received as f64 * 8.0) / duration_secs;
        let packet_loss = if seq > 0 {
            ((seq - packets_received) as f64 / seq as f64) * 100.0
        } else {
            0.0
        };

        Ok(ThroughputResult {
            duration_secs,
            bytes_sent,
            bytes_received,
            packets_sent: seq,
            packets_received,
            bandwidth_bps,
            bandwidth_mbps: bandwidth_bps / 1_000_000.0,
            packet_loss_percent: packet_loss,
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

    /// Get a handle to stop the server
    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    /// Run server
    pub fn run(&self) -> io::Result<ThroughputResult> {
        self.running.store(true, Ordering::SeqCst);

        let header_size = std::mem::size_of::<ThroughputPacket>();
        let mut recv_buf = [0u8; 65536];

        let mut bytes_received: u64 = 0;
        let mut packets_received: u64 = 0;
        let mut client_addr: Option<SocketAddr> = None;
        let mut test_start: Option<Instant> = None;
        let mut last_seq: u64 = 0;

        while self.running.load(Ordering::SeqCst) {
            match self.socket.recv_from(&mut recv_buf) {
                Ok((len, src_addr)) => {
                    if len >= header_size {
                        let packet = unsafe {
                            std::ptr::read(recv_buf.as_ptr() as *const ThroughputPacket)
                        };

                        if packet.magic != THROUGHPUT_MAGIC {
                            continue;
                        }

                        match packet.op {
                            OP_START => {
                                // New test session
                                client_addr = Some(src_addr);
                                test_start = Some(Instant::now());
                                bytes_received = 0;
                                packets_received = 0;
                                last_seq = 0;
                            }
                            OP_DATA => {
                                if Some(src_addr) == client_addr {
                                    bytes_received += len as u64;
                                    packets_received += 1;
                                    last_seq = packet.seq;
                                }
                            }
                            OP_END => {
                                if Some(src_addr) == client_addr {
                                    // Send result back
                                    let result_packet = ThroughputPacket {
                                        magic: THROUGHPUT_MAGIC,
                                        op: OP_RESULT,
                                        _pad: [0; 3],
                                        seq: last_seq,
                                        timestamp: test_start.map(|t| t.elapsed().as_nanos() as u64).unwrap_or(0),
                                        total_bytes: bytes_received,
                                        total_packets: packets_received,
                                    };

                                    let bytes = unsafe {
                                        std::slice::from_raw_parts(
                                            &result_packet as *const ThroughputPacket as *const u8,
                                            header_size,
                                        )
                                    };
                                    let _ = self.socket.send_to(bytes, src_addr);

                                    // End this test session
                                    let duration = test_start.map(|t| t.elapsed().as_secs_f64()).unwrap_or(1.0);
                                    let bandwidth_bps = (bytes_received as f64 * 8.0) / duration;

                                    return Ok(ThroughputResult {
                                        duration_secs: duration,
                                        bytes_sent: 0,
                                        bytes_received,
                                        packets_sent: 0,
                                        packets_received,
                                        bandwidth_bps,
                                        bandwidth_mbps: bandwidth_bps / 1_000_000.0,
                                        packet_loss_percent: 0.0,
                                        avg_packet_size: if packets_received > 0 {
                                            bytes_received as f64 / packets_received as f64
                                        } else {
                                            0.0
                                        },
                                    });
                                }
                            }
                            _ => {}
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

        // Server stopped without completing a test
        Ok(ThroughputResult {
            duration_secs: 0.0,
            bytes_sent: 0,
            bytes_received,
            packets_sent: 0,
            packets_received,
            bandwidth_bps: 0.0,
            bandwidth_mbps: 0.0,
            packet_loss_percent: 0.0,
            avg_packet_size: 0.0,
        })
    }

    /// Stop the server
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Simple one-way bandwidth measurement (no server needed)
pub fn measure_send_bandwidth(
    target: SocketAddr,
    duration_secs: u32,
    packet_size: usize,
) -> io::Result<ThroughputResult> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let packet = vec![0xABu8; packet_size];

    let start = Instant::now();
    let duration = Duration::from_secs(duration_secs as u64);
    let mut packets_sent: u64 = 0;
    let mut bytes_sent: u64 = 0;

    while start.elapsed() < duration {
        match socket.send_to(&packet, target) {
            Ok(sent) => {
                packets_sent += 1;
                bytes_sent += sent as u64;
            }
            Err(_) => continue,
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let bandwidth_bps = (bytes_sent as f64 * 8.0) / elapsed;

    Ok(ThroughputResult {
        duration_secs: elapsed,
        bytes_sent,
        bytes_received: 0,
        packets_sent,
        packets_received: 0,
        bandwidth_bps,
        bandwidth_mbps: bandwidth_bps / 1_000_000.0,
        packet_loss_percent: 0.0,
        avg_packet_size: if packets_sent > 0 { bytes_sent as f64 / packets_sent as f64 } else { 0.0 },
    })
}
