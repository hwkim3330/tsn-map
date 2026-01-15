use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::capture::CapturedPacket;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PtpStats {
    pub sync_count: u64,
    pub follow_up_count: u64,
    pub delay_req_count: u64,
    pub delay_resp_count: u64,
    pub announce_count: u64,
    pub pdelay_req_count: u64,
    pub pdelay_resp_count: u64,
    pub grandmaster_id: Option<String>,
    pub domain: Option<u8>,
    pub avg_offset: Option<f64>,
    pub avg_delay: Option<f64>,
    pub clock_accuracy: Option<f64>,
    pub sync_interval: Option<f64>,
    pub clocks: Vec<PtpClock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtpClock {
    pub clock_id: String,
    pub port_id: u16,
    pub is_grandmaster: bool,
    pub domain: u8,
    pub last_seen: DateTime<Utc>,
    pub sync_messages: u64,
    pub announce_messages: u64,
}

#[derive(Debug, Clone)]
struct SyncPair {
    sync_time: DateTime<Utc>,
    sync_seq: u16,
    follow_up_time: Option<DateTime<Utc>>,
}

pub struct PtpAnalyzer {
    stats: PtpStats,
    clocks: HashMap<String, PtpClock>,
    sync_pairs: HashMap<u16, SyncPair>,
    offset_samples: Vec<f64>,
    delay_samples: Vec<f64>,
    last_sync_time: Option<DateTime<Utc>>,
    sync_intervals: Vec<f64>,
}

impl PtpAnalyzer {
    pub fn new() -> Self {
        Self {
            stats: PtpStats::default(),
            clocks: HashMap::new(),
            sync_pairs: HashMap::new(),
            offset_samples: Vec::with_capacity(1000),
            delay_samples: Vec::with_capacity(1000),
            last_sync_time: None,
            sync_intervals: Vec::with_capacity(100),
        }
    }

    pub fn process_packet(&mut self, packet: &CapturedPacket) {
        if let Some(ref tsn_info) = packet.tsn_info {
            if let Some(ref ptp_info) = tsn_info.ptp_info {
                // Update message counts
                match ptp_info.message_type.as_str() {
                    "Sync" => {
                        self.stats.sync_count += 1;
                        self.process_sync(packet, ptp_info);
                    }
                    "Follow_Up" => {
                        self.stats.follow_up_count += 1;
                        self.process_follow_up(packet, ptp_info);
                    }
                    "Delay_Req" => self.stats.delay_req_count += 1,
                    "Delay_Resp" => {
                        self.stats.delay_resp_count += 1;
                        self.process_delay_resp(packet, ptp_info);
                    }
                    "Announce" => {
                        self.stats.announce_count += 1;
                        self.process_announce(packet, ptp_info);
                    }
                    "Pdelay_Req" => self.stats.pdelay_req_count += 1,
                    "Pdelay_Resp" => self.stats.pdelay_resp_count += 1,
                    _ => {}
                }

                // Update domain
                if self.stats.domain.is_none() {
                    self.stats.domain = Some(ptp_info.domain);
                }

                // Update clock info
                self.update_clock(packet, ptp_info);
            }
        }
    }

    fn process_sync(&mut self, packet: &CapturedPacket, ptp_info: &crate::capture::PtpInfo) {
        // Track sync interval
        if let Some(last_time) = self.last_sync_time {
            let interval = (packet.timestamp - last_time).num_microseconds().unwrap_or(0) as f64 / 1000.0;
            if interval > 0.0 && interval < 10000.0 {
                self.sync_intervals.push(interval);
                if self.sync_intervals.len() > 100 {
                    self.sync_intervals.remove(0);
                }
            }
        }
        self.last_sync_time = Some(packet.timestamp);

        // Store sync for follow-up matching
        self.sync_pairs.insert(ptp_info.sequence_id, SyncPair {
            sync_time: packet.timestamp,
            sync_seq: ptp_info.sequence_id,
            follow_up_time: None,
        });

        // Update sync interval stat
        if !self.sync_intervals.is_empty() {
            let avg: f64 = self.sync_intervals.iter().sum::<f64>() / self.sync_intervals.len() as f64;
            self.stats.sync_interval = Some(avg);
        }
    }

    fn process_follow_up(&mut self, _packet: &CapturedPacket, ptp_info: &crate::capture::PtpInfo) {
        if let Some(sync_pair) = self.sync_pairs.get_mut(&ptp_info.sequence_id) {
            // Calculate offset from correction field (nanoseconds)
            let offset_ns = ptp_info.correction_field as f64 / 65536.0;
            self.offset_samples.push(offset_ns);

            if self.offset_samples.len() > 1000 {
                self.offset_samples.remove(0);
            }

            // Update average offset
            if !self.offset_samples.is_empty() {
                let avg: f64 = self.offset_samples.iter().sum::<f64>() / self.offset_samples.len() as f64;
                self.stats.avg_offset = Some(avg);
            }
        }
    }

    fn process_delay_resp(&mut self, _packet: &CapturedPacket, ptp_info: &crate::capture::PtpInfo) {
        // Extract delay from correction field
        let delay_ns = (ptp_info.correction_field as f64 / 65536.0).abs();
        if delay_ns > 0.0 && delay_ns < 1_000_000_000.0 {
            self.delay_samples.push(delay_ns);

            if self.delay_samples.len() > 1000 {
                self.delay_samples.remove(0);
            }

            // Update average delay
            if !self.delay_samples.is_empty() {
                let avg: f64 = self.delay_samples.iter().sum::<f64>() / self.delay_samples.len() as f64;
                self.stats.avg_delay = Some(avg);
            }
        }
    }

    fn process_announce(&mut self, _packet: &CapturedPacket, ptp_info: &crate::capture::PtpInfo) {
        // Set grandmaster from announce source
        if self.stats.grandmaster_id.is_none() {
            self.stats.grandmaster_id = Some(ptp_info.source_port_identity.clone());
        }
    }

    fn update_clock(&mut self, packet: &CapturedPacket, ptp_info: &crate::capture::PtpInfo) {
        let clock_id = ptp_info.source_port_identity.clone();

        let entry = self.clocks.entry(clock_id.clone()).or_insert_with(|| {
            PtpClock {
                clock_id: clock_id.clone(),
                port_id: 0,
                is_grandmaster: false,
                domain: ptp_info.domain,
                last_seen: packet.timestamp,
                sync_messages: 0,
                announce_messages: 0,
            }
        });

        entry.last_seen = packet.timestamp;
        entry.domain = ptp_info.domain;

        match ptp_info.message_type.as_str() {
            "Sync" | "Follow_Up" => entry.sync_messages += 1,
            "Announce" => {
                entry.announce_messages += 1;
                // Grandmaster sends Announce messages
                if self.stats.grandmaster_id.as_ref() == Some(&clock_id) {
                    entry.is_grandmaster = true;
                }
            }
            _ => {}
        }

        // Update stats clocks
        self.stats.clocks = self.clocks.values().cloned().collect();
    }

    pub fn get_stats(&self) -> &PtpStats {
        &self.stats
    }

    pub fn get_clocks(&self) -> Vec<&PtpClock> {
        self.clocks.values().collect()
    }
}

impl Default for PtpAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
