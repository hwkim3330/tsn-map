//! Hardware timestamping support using SO_TIMESTAMPING
//!
//! This module provides hardware-level timestamp support for precise latency measurement.
//! Requires Linux kernel 2.6.30+ and a NIC that supports hardware timestamping.
//!
//! Supported NICs include:
//! - Intel: i210, i225, i350, i40e, ice, ixgbe
//! - Mellanox: mlx4, mlx5
//! - Most 10GbE+ NICs

use std::io;
use std::mem;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::os::unix::io::AsRawFd;
use std::time::Duration;

use libc::{
    c_int, c_void, cmsghdr, iovec, msghdr, recvmsg, sendmsg, setsockopt,
    timespec, CMSG_DATA, CMSG_FIRSTHDR, CMSG_NXTHDR, MSG_ERRQUEUE,
    SCM_TIMESTAMPING, SO_TIMESTAMPING, SOL_SOCKET,
};

use serde::{Deserialize, Serialize};

// SO_TIMESTAMPING flags (from linux/net_tstamp.h)
const SOF_TIMESTAMPING_TX_HARDWARE: u32 = 1 << 0;
const SOF_TIMESTAMPING_TX_SOFTWARE: u32 = 1 << 1;
const SOF_TIMESTAMPING_RX_HARDWARE: u32 = 1 << 2;
const SOF_TIMESTAMPING_RX_SOFTWARE: u32 = 1 << 3;
const SOF_TIMESTAMPING_SOFTWARE: u32 = 1 << 4;
const SOF_TIMESTAMPING_RAW_HARDWARE: u32 = 1 << 6;
const SOF_TIMESTAMPING_OPT_TSONLY: u32 = 1 << 11;

// SIOCETHTOOL for checking HW timestamp capability
const SIOCETHTOOL: libc::c_ulong = 0x8946;
const ETHTOOL_GET_TS_INFO: u32 = 0x00000041;

/// Hardware timestamp info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwTimestamp {
    pub sec: i64,
    pub nsec: i64,
    pub source: TimestampSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TimestampSource {
    Hardware,
    Software,
    None,
}

/// Result of a hardware-timestamped ping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwLatencyResult {
    pub seq: u32,
    pub success: bool,
    pub rtt_ns: i64,          // Round-trip time in nanoseconds
    pub rtt_us: f64,          // Round-trip time in microseconds
    pub tx_timestamp: Option<HwTimestamp>,
    pub rx_timestamp: Option<HwTimestamp>,
    pub timestamp_source: TimestampSource,
}

/// Statistics for hardware-timestamped latency test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwLatencyStats {
    pub count: u32,
    pub success_count: u32,
    pub min_ns: i64,
    pub max_ns: i64,
    pub avg_ns: f64,
    pub jitter_ns: f64,       // Standard deviation in ns
    pub min_us: f64,
    pub max_us: f64,
    pub avg_us: f64,
    pub jitter_us: f64,
    pub loss_percent: f64,
    pub hw_timestamp_count: u32,  // How many used HW timestamps
    pub sw_timestamp_count: u32,  // How many fell back to SW
}

/// Timestamp capability of an interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampCapability {
    pub interface: String,
    pub hw_tx_supported: bool,
    pub hw_rx_supported: bool,
    pub sw_tx_supported: bool,
    pub sw_rx_supported: bool,
    pub phc_index: i32,  // PTP Hardware Clock index (-1 if none)
}

/// Check if an interface supports hardware timestamping
pub fn check_timestamp_capability(interface: &str) -> io::Result<TimestampCapability> {
    // Try to get ethtool timestamp info
    // This is a simplified check - full implementation would use SIOCETHTOOL

    // For now, we'll try to enable timestamps and see if it works
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let fd = socket.as_raw_fd();

    // Try to enable HW timestamping
    let flags: u32 = SOF_TIMESTAMPING_TX_HARDWARE
        | SOF_TIMESTAMPING_RX_HARDWARE
        | SOF_TIMESTAMPING_RAW_HARDWARE;

    let hw_supported = unsafe {
        setsockopt(
            fd,
            SOL_SOCKET,
            SO_TIMESTAMPING,
            &flags as *const u32 as *const c_void,
            mem::size_of::<u32>() as u32,
        ) == 0
    };

    // Try SW timestamping
    let sw_flags: u32 = SOF_TIMESTAMPING_TX_SOFTWARE
        | SOF_TIMESTAMPING_RX_SOFTWARE
        | SOF_TIMESTAMPING_SOFTWARE;

    let sw_supported = unsafe {
        setsockopt(
            fd,
            SOL_SOCKET,
            SO_TIMESTAMPING,
            &sw_flags as *const u32 as *const c_void,
            mem::size_of::<u32>() as u32,
        ) == 0
    };

    Ok(TimestampCapability {
        interface: interface.to_string(),
        hw_tx_supported: hw_supported,
        hw_rx_supported: hw_supported,
        sw_tx_supported: sw_supported,
        sw_rx_supported: sw_supported,
        phc_index: -1,  // Would need ETHTOOL_GET_TS_INFO to get this
    })
}

/// Hardware-timestamped latency tester
pub struct HwLatencyTester {
    socket: UdpSocket,
    fd: c_int,
    target: SocketAddr,
    use_hw_timestamps: bool,
}

impl HwLatencyTester {
    /// Create a new hardware-timestamped latency tester
    pub fn new(target_ip: IpAddr, port: u16, interface: Option<&str>) -> io::Result<Self> {
        let target = SocketAddr::new(target_ip, port);
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let fd = socket.as_raw_fd();

        socket.set_read_timeout(Some(Duration::from_secs(2)))?;
        socket.set_write_timeout(Some(Duration::from_secs(1)))?;

        // Bind to specific interface if specified
        if let Some(iface) = interface {
            unsafe {
                let iface_cstr = std::ffi::CString::new(iface).unwrap();
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_BINDTODEVICE,
                    iface_cstr.as_ptr() as *const c_void,
                    iface.len() as u32 + 1,
                );
            }
        }

        // Try to enable hardware timestamps first
        let hw_flags: u32 = SOF_TIMESTAMPING_TX_HARDWARE
            | SOF_TIMESTAMPING_RX_HARDWARE
            | SOF_TIMESTAMPING_RAW_HARDWARE
            | SOF_TIMESTAMPING_TX_SOFTWARE
            | SOF_TIMESTAMPING_RX_SOFTWARE
            | SOF_TIMESTAMPING_SOFTWARE;

        let hw_enabled = unsafe {
            setsockopt(
                fd,
                SOL_SOCKET,
                SO_TIMESTAMPING,
                &hw_flags as *const u32 as *const c_void,
                mem::size_of::<u32>() as u32,
            ) == 0
        };

        if !hw_enabled {
            // Fall back to software timestamps only
            let sw_flags: u32 = SOF_TIMESTAMPING_TX_SOFTWARE
                | SOF_TIMESTAMPING_RX_SOFTWARE
                | SOF_TIMESTAMPING_SOFTWARE;

            unsafe {
                setsockopt(
                    fd,
                    SOL_SOCKET,
                    SO_TIMESTAMPING,
                    &sw_flags as *const u32 as *const c_void,
                    mem::size_of::<u32>() as u32,
                );
            }
        }

        Ok(Self {
            socket,
            fd,
            target,
            use_hw_timestamps: hw_enabled,
        })
    }

    /// Check if hardware timestamps are enabled
    pub fn hw_timestamps_enabled(&self) -> bool {
        self.use_hw_timestamps
    }

    /// Send a ping with hardware timestamps
    pub fn ping(&self, seq: u32) -> HwLatencyResult {
        // Prepare packet
        let packet = create_ping_packet(seq);

        // Send with timestamp capture
        let tx_ts = self.send_with_timestamp(&packet);

        // Receive with timestamp
        let (rx_ts, success, recv_seq) = self.recv_with_timestamp(seq);

        if !success || recv_seq != Some(seq) {
            return HwLatencyResult {
                seq,
                success: false,
                rtt_ns: 0,
                rtt_us: 0.0,
                tx_timestamp: tx_ts,
                rx_timestamp: None,
                timestamp_source: TimestampSource::None,
            };
        }

        // Calculate RTT
        let (rtt_ns, source) = match (&tx_ts, &rx_ts) {
            (Some(tx), Some(rx)) => {
                let rtt = (rx.sec - tx.sec) * 1_000_000_000 + (rx.nsec - tx.nsec);
                let src = if tx.source == TimestampSource::Hardware
                    && rx.source == TimestampSource::Hardware {
                    TimestampSource::Hardware
                } else {
                    TimestampSource::Software
                };
                (rtt, src)
            }
            _ => (0, TimestampSource::None),
        };

        HwLatencyResult {
            seq,
            success: true,
            rtt_ns,
            rtt_us: rtt_ns as f64 / 1000.0,
            tx_timestamp: tx_ts,
            rx_timestamp: rx_ts,
            timestamp_source: source,
        }
    }

    /// Send packet and get TX timestamp
    fn send_with_timestamp(&self, data: &[u8]) -> Option<HwTimestamp> {
        let dest: libc::sockaddr_in = unsafe { mem::zeroed() };
        let mut dest = dest;
        dest.sin_family = libc::AF_INET as u16;
        dest.sin_port = self.target.port().to_be();

        if let IpAddr::V4(ip) = self.target.ip() {
            dest.sin_addr.s_addr = u32::from_ne_bytes(ip.octets());
        }

        let mut iov = iovec {
            iov_base: data.as_ptr() as *mut c_void,
            iov_len: data.len(),
        };

        let mut msg: msghdr = unsafe { mem::zeroed() };
        msg.msg_name = &mut dest as *mut _ as *mut c_void;
        msg.msg_namelen = mem::size_of::<libc::sockaddr_in>() as u32;
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;

        let sent = unsafe { sendmsg(self.fd, &msg, 0) };
        if sent < 0 {
            return None;
        }

        // Get TX timestamp from error queue
        self.get_tx_timestamp()
    }

    /// Get TX timestamp from error queue
    fn get_tx_timestamp(&self) -> Option<HwTimestamp> {
        let mut buf = [0u8; 256];
        let mut control = [0u8; 256];

        let mut iov = iovec {
            iov_base: buf.as_mut_ptr() as *mut c_void,
            iov_len: buf.len(),
        };

        let mut msg: msghdr = unsafe { mem::zeroed() };
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = control.as_mut_ptr() as *mut c_void;
        msg.msg_controllen = control.len();

        // Poll error queue with timeout
        let mut timeout = libc::timeval {
            tv_sec: 0,
            tv_usec: 100_000, // 100ms
        };

        let mut readfds: libc::fd_set = unsafe { mem::zeroed() };
        unsafe {
            libc::FD_ZERO(&mut readfds);
            libc::FD_SET(self.fd, &mut readfds);
        }

        // Wait for TX timestamp
        for _ in 0..10 {
            let ret = unsafe {
                recvmsg(self.fd, &mut msg, MSG_ERRQUEUE)
            };

            if ret >= 0 {
                return self.parse_timestamp(&msg);
            }

            std::thread::sleep(Duration::from_micros(1000));
        }

        None
    }

    /// Receive packet and get RX timestamp
    fn recv_with_timestamp(&self, expected_seq: u32) -> (Option<HwTimestamp>, bool, Option<u32>) {
        let mut buf = [0u8; 128];
        let mut control = [0u8; 256];
        let mut src_addr: libc::sockaddr_in = unsafe { mem::zeroed() };

        let mut iov = iovec {
            iov_base: buf.as_mut_ptr() as *mut c_void,
            iov_len: buf.len(),
        };

        let mut msg: msghdr = unsafe { mem::zeroed() };
        msg.msg_name = &mut src_addr as *mut _ as *mut c_void;
        msg.msg_namelen = mem::size_of::<libc::sockaddr_in>() as u32;
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = control.as_mut_ptr() as *mut c_void;
        msg.msg_controllen = control.len();

        let ret = unsafe { recvmsg(self.fd, &mut msg, 0) };

        if ret < 12 {
            return (None, false, None);
        }

        // Parse packet
        let recv_seq = parse_pong_packet(&buf[..ret as usize]);
        let ts = self.parse_timestamp(&msg);

        (ts, recv_seq == Some(expected_seq), recv_seq)
    }

    /// Parse timestamp from control message
    fn parse_timestamp(&self, msg: &msghdr) -> Option<HwTimestamp> {
        let mut cmsg: *mut cmsghdr = unsafe { CMSG_FIRSTHDR(msg) };

        while !cmsg.is_null() {
            let hdr = unsafe { &*cmsg };

            if hdr.cmsg_level == SOL_SOCKET && hdr.cmsg_type == SCM_TIMESTAMPING {
                let data = unsafe { CMSG_DATA(cmsg) };
                let ts_array = unsafe { &*(data as *const [timespec; 3]) };

                // ts_array[0] = software timestamp
                // ts_array[1] = deprecated
                // ts_array[2] = hardware timestamp

                // Prefer hardware timestamp
                if ts_array[2].tv_sec != 0 || ts_array[2].tv_nsec != 0 {
                    return Some(HwTimestamp {
                        sec: ts_array[2].tv_sec,
                        nsec: ts_array[2].tv_nsec,
                        source: TimestampSource::Hardware,
                    });
                }

                // Fall back to software timestamp
                if ts_array[0].tv_sec != 0 || ts_array[0].tv_nsec != 0 {
                    return Some(HwTimestamp {
                        sec: ts_array[0].tv_sec,
                        nsec: ts_array[0].tv_nsec,
                        source: TimestampSource::Software,
                    });
                }
            }

            cmsg = unsafe { CMSG_NXTHDR(msg, cmsg) };
        }

        None
    }

    /// Run a full latency test
    pub fn run(&self, count: u32, interval_ms: u32) -> Vec<HwLatencyResult> {
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

    /// Calculate statistics from results
    pub fn calculate_stats(results: &[HwLatencyResult]) -> HwLatencyStats {
        let success_results: Vec<_> = results.iter().filter(|r| r.success).collect();
        let count = results.len() as u32;
        let success_count = success_results.len() as u32;

        if success_results.is_empty() {
            return HwLatencyStats {
                count,
                success_count: 0,
                min_ns: 0,
                max_ns: 0,
                avg_ns: 0.0,
                jitter_ns: 0.0,
                min_us: 0.0,
                max_us: 0.0,
                avg_us: 0.0,
                jitter_us: 0.0,
                loss_percent: 100.0,
                hw_timestamp_count: 0,
                sw_timestamp_count: 0,
            };
        }

        let rtts_ns: Vec<i64> = success_results.iter().map(|r| r.rtt_ns).collect();
        let min_ns = *rtts_ns.iter().min().unwrap_or(&0);
        let max_ns = *rtts_ns.iter().max().unwrap_or(&0);
        let sum_ns: i64 = rtts_ns.iter().sum();
        let avg_ns = sum_ns as f64 / rtts_ns.len() as f64;

        // Standard deviation
        let variance: f64 = rtts_ns.iter()
            .map(|&x| (x as f64 - avg_ns).powi(2))
            .sum::<f64>() / rtts_ns.len() as f64;
        let jitter_ns = variance.sqrt();

        let hw_count = success_results.iter()
            .filter(|r| r.timestamp_source == TimestampSource::Hardware)
            .count() as u32;
        let sw_count = success_results.iter()
            .filter(|r| r.timestamp_source == TimestampSource::Software)
            .count() as u32;

        let loss_percent = ((count - success_count) as f64 / count as f64) * 100.0;

        HwLatencyStats {
            count,
            success_count,
            min_ns,
            max_ns,
            avg_ns,
            jitter_ns,
            min_us: min_ns as f64 / 1000.0,
            max_us: max_ns as f64 / 1000.0,
            avg_us: avg_ns / 1000.0,
            jitter_us: jitter_ns / 1000.0,
            loss_percent,
            hw_timestamp_count: hw_count,
            sw_timestamp_count: sw_count,
        }
    }
}

// Packet format for ping/pong
const MAGIC: [u8; 4] = *b"HWTS";
const OP_PING: u8 = 0;
const OP_PONG: u8 = 1;

fn create_ping_packet(seq: u32) -> Vec<u8> {
    let mut packet = vec![0u8; 64];
    packet[0..4].copy_from_slice(&MAGIC);
    packet[4..8].copy_from_slice(&seq.to_le_bytes());
    packet[8] = OP_PING;
    packet
}

fn parse_pong_packet(data: &[u8]) -> Option<u32> {
    if data.len() < 12 {
        return None;
    }

    if &data[0..4] != &MAGIC {
        return None;
    }

    if data[8] != OP_PONG {
        return None;
    }

    let seq = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    Some(seq)
}

/// Hardware timestamp responder (server side)
pub struct HwLatencyServer {
    socket: UdpSocket,
}

impl HwLatencyServer {
    pub fn new(bind_addr: &str, port: u16) -> io::Result<Self> {
        let socket = UdpSocket::bind(format!("{}:{}", bind_addr, port))?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        Ok(Self { socket })
    }

    /// Run server, responding to ping packets
    pub fn run(&self, duration_secs: u64) -> io::Result<u64> {
        let start = std::time::Instant::now();
        let mut count = 0u64;

        loop {
            if duration_secs > 0 && start.elapsed().as_secs() >= duration_secs {
                break;
            }

            let mut buf = [0u8; 128];
            match self.socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    if len >= 12 && &buf[0..4] == &MAGIC && buf[8] == OP_PING {
                        // Create pong response
                        let mut response = buf[..len].to_vec();
                        response[8] = OP_PONG;
                        let _ = self.socket.send_to(&response, src);
                        count += 1;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut => continue,
                Err(e) => return Err(e),
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_format() {
        let packet = create_ping_packet(42);
        assert_eq!(&packet[0..4], &MAGIC);
        assert_eq!(u32::from_le_bytes([packet[4], packet[5], packet[6], packet[7]]), 42);
        assert_eq!(packet[8], OP_PING);
    }

    #[test]
    fn test_check_capability() {
        // This test just checks that the function doesn't crash
        let cap = check_timestamp_capability("lo");
        assert!(cap.is_ok());
    }
}
