mod ptp;
mod cbs;
mod tas;
mod frer;

pub use ptp::{PtpAnalyzer, PtpStats};
pub use cbs::{CbsAnalyzer, CbsStats};
pub use tas::{TasAnalyzer, TasStats};
pub use frer::{FrerAnalyzer, FrerStats};

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::capture::CapturedPacket;

/// TSN Stream information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsnStream {
    pub stream_id: String,
    pub src_mac: String,
    pub dst_mac: String,
    pub vlan_id: Option<u16>,
    pub priority: u8,
    pub bandwidth: f64,          // Mbps
    pub packet_count: u64,
    pub byte_count: u64,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub avg_latency: Option<f64>, // microseconds
    pub jitter: Option<f64>,      // microseconds
}

/// TSN Flow - aggregated traffic pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsnFlow {
    pub flow_id: String,
    pub src: String,
    pub dst: String,
    pub protocol: String,
    pub traffic_class: u8,
    pub packets: u64,
    pub bytes: u64,
    pub rate: f64,
    pub streams: Vec<String>,
}

/// Protocol Analyzer - main entry point for protocol analysis
pub struct ProtocolAnalyzer {
    pub ptp: PtpAnalyzer,
    pub cbs: CbsAnalyzer,
    pub tas: TasAnalyzer,
    pub frer: FrerAnalyzer,
    streams: HashMap<String, TsnStream>,
    flows: HashMap<String, TsnFlow>,
}

impl ProtocolAnalyzer {
    pub fn new() -> Self {
        Self {
            ptp: PtpAnalyzer::new(),
            cbs: CbsAnalyzer::new(),
            tas: TasAnalyzer::new(),
            frer: FrerAnalyzer::new(),
            streams: HashMap::new(),
            flows: HashMap::new(),
        }
    }

    pub fn analyze_packet(&mut self, packet: &CapturedPacket) {
        // Update PTP analysis
        if packet.info.is_ptp {
            self.ptp.process_packet(packet);
        }

        // Update stream tracking
        if let Some(ref tsn_info) = packet.tsn_info {
            if let Some(ref stream_id) = tsn_info.stream_id {
                self.update_stream(stream_id, packet);
            }

            // Process by TSN type
            match tsn_info.tsn_type {
                crate::capture::TsnType::Cbs => self.cbs.process_packet(packet),
                crate::capture::TsnType::Tas => self.tas.process_packet(packet),
                crate::capture::TsnType::Frer => self.frer.process_packet(packet),
                _ => {}
            }
        }

        // Update flow tracking
        self.update_flow(packet);
    }

    fn update_stream(&mut self, stream_id: &str, packet: &CapturedPacket) {
        let entry = self.streams.entry(stream_id.to_string()).or_insert_with(|| {
            TsnStream {
                stream_id: stream_id.to_string(),
                src_mac: packet.info.src_mac.clone(),
                dst_mac: packet.info.dst_mac.clone(),
                vlan_id: packet.info.vlan_id,
                priority: packet.info.vlan_pcp.unwrap_or(0),
                bandwidth: 0.0,
                packet_count: 0,
                byte_count: 0,
                first_seen: packet.timestamp,
                last_seen: packet.timestamp,
                avg_latency: None,
                jitter: None,
            }
        });

        entry.packet_count += 1;
        entry.byte_count += packet.length as u64;
        entry.last_seen = packet.timestamp;

        // Calculate bandwidth
        let duration = (entry.last_seen - entry.first_seen).num_milliseconds() as f64 / 1000.0;
        if duration > 0.0 {
            entry.bandwidth = (entry.byte_count as f64 * 8.0) / (duration * 1_000_000.0); // Mbps
        }
    }

    fn update_flow(&mut self, packet: &CapturedPacket) {
        let flow_id = format!(
            "{}:{}:{}:{}",
            packet.info.src_mac,
            packet.info.dst_mac,
            packet.info.vlan_id.unwrap_or(0),
            packet.info.protocol.as_deref().unwrap_or("ETH")
        );

        let entry = self.flows.entry(flow_id.clone()).or_insert_with(|| {
            TsnFlow {
                flow_id: flow_id.clone(),
                src: packet.info.src_mac.clone(),
                dst: packet.info.dst_mac.clone(),
                protocol: packet.info.protocol.clone().unwrap_or_else(|| "ETH".to_string()),
                traffic_class: packet.info.vlan_pcp.unwrap_or(0),
                packets: 0,
                bytes: 0,
                rate: 0.0,
                streams: Vec::new(),
            }
        });

        entry.packets += 1;
        entry.bytes += packet.length as u64;

        if let Some(ref tsn_info) = packet.tsn_info {
            if let Some(ref stream_id) = tsn_info.stream_id {
                if !entry.streams.contains(stream_id) {
                    entry.streams.push(stream_id.clone());
                }
            }
        }
    }

    pub fn get_streams(&self) -> Vec<&TsnStream> {
        self.streams.values().collect()
    }

    pub fn get_flows(&self) -> Vec<&TsnFlow> {
        self.flows.values().collect()
    }

    pub fn get_ptp_stats(&self) -> &PtpStats {
        self.ptp.get_stats()
    }

    pub fn get_cbs_stats(&self) -> &CbsStats {
        self.cbs.get_stats()
    }

    pub fn reset(&mut self) {
        self.streams.clear();
        self.flows.clear();
        self.ptp = PtpAnalyzer::new();
        self.cbs = CbsAnalyzer::new();
        self.tas = TasAnalyzer::new();
        self.frer = FrerAnalyzer::new();
    }
}

impl Default for ProtocolAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
