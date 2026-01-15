use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::capture::CapturedPacket;

/// CBS (Credit-Based Shaper) Statistics per Traffic Class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbsTrafficClass {
    pub tc: u8,
    pub packets: u64,
    pub bytes: u64,
    pub bandwidth_mbps: f64,
    pub avg_packet_size: f64,
    pub max_burst_size: u32,
    pub idle_slope: Option<u32>,
    pub send_slope: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CbsStats {
    pub total_packets: u64,
    pub total_bytes: u64,
    pub traffic_classes: Vec<CbsTrafficClass>,
    pub priority_distribution: HashMap<u8, u64>,
    pub avg_bandwidth_mbps: f64,
    pub peak_bandwidth_mbps: f64,
}

struct TcTracker {
    packets: u64,
    bytes: u64,
    first_time: DateTime<Utc>,
    last_time: DateTime<Utc>,
    burst_bytes: u32,
    max_burst: u32,
    packet_sizes: Vec<u32>,
}

pub struct CbsAnalyzer {
    stats: CbsStats,
    tc_trackers: HashMap<u8, TcTracker>,
    bandwidth_samples: Vec<f64>,
    last_calculation: Option<DateTime<Utc>>,
    bytes_in_window: u64,
}

impl CbsAnalyzer {
    pub fn new() -> Self {
        Self {
            stats: CbsStats::default(),
            tc_trackers: HashMap::new(),
            bandwidth_samples: Vec::with_capacity(100),
            last_calculation: None,
            bytes_in_window: 0,
        }
    }

    pub fn process_packet(&mut self, packet: &CapturedPacket) {
        let priority = packet.info.vlan_pcp.unwrap_or(0);
        let tc = Self::pcp_to_tc(priority);

        // Update total stats
        self.stats.total_packets += 1;
        self.stats.total_bytes += packet.length as u64;

        // Update priority distribution
        *self.stats.priority_distribution.entry(priority).or_insert(0) += 1;

        // Update per-TC tracker
        let tracker = self.tc_trackers.entry(tc).or_insert_with(|| TcTracker {
            packets: 0,
            bytes: 0,
            first_time: packet.timestamp,
            last_time: packet.timestamp,
            burst_bytes: 0,
            max_burst: 0,
            packet_sizes: Vec::with_capacity(1000),
        });

        tracker.packets += 1;
        tracker.bytes += packet.length as u64;

        // Track burst
        let time_gap = (packet.timestamp - tracker.last_time).num_microseconds().unwrap_or(0);
        if time_gap < 1000 {
            // Within 1ms - same burst
            tracker.burst_bytes += packet.length;
            if tracker.burst_bytes > tracker.max_burst {
                tracker.max_burst = tracker.burst_bytes;
            }
        } else {
            tracker.burst_bytes = packet.length;
        }

        tracker.last_time = packet.timestamp;
        tracker.packet_sizes.push(packet.length);
        if tracker.packet_sizes.len() > 1000 {
            tracker.packet_sizes.remove(0);
        }

        // Calculate bandwidth
        self.bytes_in_window += packet.length as u64;
        if let Some(last) = self.last_calculation {
            let elapsed = (packet.timestamp - last).num_milliseconds() as f64;
            if elapsed >= 100.0 {
                let bandwidth = (self.bytes_in_window as f64 * 8.0) / (elapsed * 1000.0); // Mbps
                self.bandwidth_samples.push(bandwidth);
                if self.bandwidth_samples.len() > 100 {
                    self.bandwidth_samples.remove(0);
                }

                self.bytes_in_window = 0;
                self.last_calculation = Some(packet.timestamp);

                // Update peak
                if bandwidth > self.stats.peak_bandwidth_mbps {
                    self.stats.peak_bandwidth_mbps = bandwidth;
                }
            }
        } else {
            self.last_calculation = Some(packet.timestamp);
        }

        // Update average bandwidth
        if !self.bandwidth_samples.is_empty() {
            self.stats.avg_bandwidth_mbps =
                self.bandwidth_samples.iter().sum::<f64>() / self.bandwidth_samples.len() as f64;
        }

        // Update traffic class stats
        self.update_tc_stats();
    }

    fn pcp_to_tc(pcp: u8) -> u8 {
        // Standard PCP to TC mapping (can be customized)
        match pcp {
            0 => 1, // Best Effort
            1 => 0, // Background
            2 => 2, // Excellent Effort
            3 => 3, // Critical Applications
            4 => 4, // Video
            5 => 5, // Voice
            6 => 6, // Internetwork Control
            7 => 7, // Network Control
            _ => 0,
        }
    }

    fn update_tc_stats(&mut self) {
        self.stats.traffic_classes = self
            .tc_trackers
            .iter()
            .map(|(&tc, tracker)| {
                let duration = (tracker.last_time - tracker.first_time).num_milliseconds() as f64 / 1000.0;
                let bandwidth = if duration > 0.0 {
                    (tracker.bytes as f64 * 8.0) / (duration * 1_000_000.0)
                } else {
                    0.0
                };

                let avg_size = if !tracker.packet_sizes.is_empty() {
                    tracker.packet_sizes.iter().map(|&s| s as f64).sum::<f64>()
                        / tracker.packet_sizes.len() as f64
                } else {
                    0.0
                };

                CbsTrafficClass {
                    tc,
                    packets: tracker.packets,
                    bytes: tracker.bytes,
                    bandwidth_mbps: bandwidth,
                    avg_packet_size: avg_size,
                    max_burst_size: tracker.max_burst,
                    idle_slope: None,
                    send_slope: None,
                }
            })
            .collect();

        // Sort by TC
        self.stats.traffic_classes.sort_by_key(|t| t.tc);
    }

    pub fn get_stats(&self) -> &CbsStats {
        &self.stats
    }

    pub fn get_tc_bandwidth(&self, tc: u8) -> Option<f64> {
        self.tc_trackers.get(&tc).map(|t| {
            let duration = (t.last_time - t.first_time).num_milliseconds() as f64 / 1000.0;
            if duration > 0.0 {
                (t.bytes as f64 * 8.0) / (duration * 1_000_000.0)
            } else {
                0.0
            }
        })
    }
}

impl Default for CbsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
