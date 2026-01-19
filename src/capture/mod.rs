mod packet;
mod pcap_handler;
mod interval;

pub use packet::{CapturedPacket, TsnType, PtpInfo};
pub use pcap_handler::PcapHandler;
pub use interval::{IntervalTracker, IntervalData, IntervalStats, IntervalSample, RttSample};

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use pcap::Device;
use chrono::{DateTime, Utc};

const MAX_PACKETS_BUFFER: usize = 100_000;

#[derive(Clone)]
pub struct CaptureStats {
    pub packets_captured: u64,
    pub bytes_captured: u64,
    pub packets_dropped: u64,
    pub tsn_packets: u64,
    pub ptp_packets: u64,
    pub start_time: Option<DateTime<Utc>>,
    pub capture_rate: f64,
}

impl Default for CaptureStats {
    fn default() -> Self {
        Self {
            packets_captured: 0,
            bytes_captured: 0,
            packets_dropped: 0,
            tsn_packets: 0,
            ptp_packets: 0,
            start_time: None,
            capture_rate: 0.0,
        }
    }
}

pub struct CaptureManager {
    interface: String,
    buffer_size: usize,
    packets: VecDeque<CapturedPacket>,
    is_capturing: Arc<AtomicBool>,
    stats: CaptureStats,
    packet_sender: broadcast::Sender<CapturedPacket>,
    packets_captured: Arc<AtomicU64>,
    bytes_captured: Arc<AtomicU64>,
    interval_tracker: IntervalTracker,
}

impl CaptureManager {
    pub fn new(interface: &str, buffer_size_mb: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let (packet_sender, _) = broadcast::channel(10000);

        Ok(Self {
            interface: interface.to_string(),
            buffer_size: buffer_size_mb * 1024 * 1024,
            packets: VecDeque::with_capacity(MAX_PACKETS_BUFFER),
            is_capturing: Arc::new(AtomicBool::new(false)),
            stats: CaptureStats::default(),
            packet_sender,
            packets_captured: Arc::new(AtomicU64::new(0)),
            bytes_captured: Arc::new(AtomicU64::new(0)),
            interval_tracker: IntervalTracker::new(),
        })
    }

    pub fn start_capture(&mut self) -> Result<broadcast::Receiver<CapturedPacket>, Box<dyn std::error::Error + Send + Sync>> {
        if self.is_capturing.load(Ordering::SeqCst) {
            return Ok(self.packet_sender.subscribe());
        }

        // Verify interface exists
        let _device = Device::list()?
            .into_iter()
            .find(|d| d.name == self.interface)
            .ok_or_else(|| format!("Interface {} not found", self.interface))?;

        self.is_capturing.store(true, Ordering::SeqCst);
        self.stats.start_time = Some(Utc::now());
        self.packets.clear();

        Ok(self.packet_sender.subscribe())
    }

    pub fn stop_capture(&mut self) {
        self.is_capturing.store(false, Ordering::SeqCst);
    }

    pub fn is_capturing(&self) -> bool {
        self.is_capturing.load(Ordering::SeqCst)
    }

    pub fn get_stats(&self) -> CaptureStats {
        self.stats.clone()
    }

    pub fn get_packets(&self, offset: usize, limit: usize) -> Vec<CapturedPacket> {
        self.packets
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn get_packet_count(&self) -> usize {
        self.packets.len()
    }

    pub fn add_packet(&mut self, packet: CapturedPacket) {
        // Update stats
        self.stats.packets_captured += 1;
        self.stats.bytes_captured += packet.length as u64;

        if packet.tsn_info.is_some() {
            self.stats.tsn_packets += 1;
        }
        if packet.info.is_ptp {
            self.stats.ptp_packets += 1;
        }

        // Track packet intervals and TCP RTT
        let is_tcp = packet.info.protocol.as_deref() == Some("TCP");
        let tcp_flags_ack = packet.info.tcp_flags.as_ref().map(|f| f.ack).unwrap_or(false);

        // Calculate TCP payload length
        let payload_len = if is_tcp {
            // Approximate: total length - headers (14 eth + 20 ip + 20 tcp minimum)
            packet.length.saturating_sub(54)
        } else {
            0
        };

        // Format source and destination for display
        let src = packet.info.src_ip.as_ref()
            .map(|ip| {
                packet.info.src_port.map(|p| format!("{}:{}", ip, p))
                    .unwrap_or_else(|| ip.clone())
            })
            .unwrap_or_else(|| packet.info.src_mac.clone());
        let dst = packet.info.dst_ip.as_ref()
            .map(|ip| {
                packet.info.dst_port.map(|p| format!("{}:{}", ip, p))
                    .unwrap_or_else(|| ip.clone())
            })
            .unwrap_or_else(|| packet.info.dst_mac.clone());

        self.interval_tracker.process_packet(
            packet.id,
            packet.timestamp,
            Instant::now(),  // capture instant
            packet.length,
            &src,
            &dst,
            packet.info.protocol.as_deref().unwrap_or(&packet.info.ethertype_name),
            packet.info.src_ip.as_deref(),
            packet.info.dst_ip.as_deref(),
            packet.info.src_port,
            packet.info.dst_port,
            is_tcp,
            packet.info.seq_num,
            packet.info.ack_num,
            tcp_flags_ack,
            payload_len,
        );

        // Broadcast to subscribers
        let _ = self.packet_sender.send(packet.clone());

        // Add to buffer (remove old if full)
        if self.packets.len() >= MAX_PACKETS_BUFFER {
            self.packets.pop_front();
        }
        self.packets.push_back(packet);
    }

    pub fn get_sender(&self) -> broadcast::Sender<CapturedPacket> {
        self.packet_sender.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CapturedPacket> {
        self.packet_sender.subscribe()
    }

    pub fn set_interface(&mut self, interface: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_capturing() {
            return Err("Cannot change interface while capturing".into());
        }
        self.interface = interface.to_string();
        Ok(())
    }

    pub fn get_interface(&self) -> &str {
        &self.interface
    }

    pub fn get_buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn clear_packets(&mut self) {
        self.packets.clear();
        self.stats = CaptureStats::default();
        self.interval_tracker.reset();
    }

    /// Get interval and RTT data for API
    pub fn get_interval_data(&self, limit: usize) -> IntervalData {
        self.interval_tracker.get_data(limit)
    }
}

pub fn list_interfaces() -> Result<Vec<InterfaceInfo>, Box<dyn std::error::Error>> {
    let devices = Device::list()?;
    Ok(devices.into_iter().map(|d| InterfaceInfo {
        name: d.name,
        description: d.desc.unwrap_or_default(),
        addresses: d.addresses.iter().map(|a| format!("{:?}", a.addr)).collect(),
    }).collect())
}

#[derive(serde::Serialize, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub description: String,
    pub addresses: Vec<String>,
}
