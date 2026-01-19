//! Packet interval and latency tracking
//!
//! Tracks:
//! - Inter-packet arrival time (like Wireshark's delta time)
//! - TCP RTT estimation from ACK timing
//! - Per-flow statistics

use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// TCP flow identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TcpFlowKey {
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    pub dst_port: u16,
}

impl TcpFlowKey {
    pub fn new(src_ip: &str, dst_ip: &str, src_port: u16, dst_port: u16) -> Self {
        Self {
            src_ip: src_ip.to_string(),
            dst_ip: dst_ip.to_string(),
            src_port,
            dst_port,
        }
    }

    /// Get reverse flow key (for matching ACKs)
    pub fn reverse(&self) -> Self {
        Self {
            src_ip: self.dst_ip.clone(),
            dst_ip: self.src_ip.clone(),
            src_port: self.dst_port,
            dst_port: self.src_port,
        }
    }
}

/// Pending TCP segment waiting for ACK
#[derive(Debug, Clone)]
struct PendingSegment {
    seq_end: u32,  // seq + payload_len
    timestamp: Instant,
}

/// TCP flow state for RTT tracking
#[derive(Debug)]
struct TcpFlowState {
    pending_segments: Vec<PendingSegment>,
    rtt_samples: Vec<f64>,  // RTT in microseconds
    last_activity: Instant,
}

impl Default for TcpFlowState {
    fn default() -> Self {
        Self {
            pending_segments: Vec::new(),
            rtt_samples: Vec::new(),
            last_activity: Instant::now(),
        }
    }
}

/// Single interval sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalSample {
    pub timestamp: DateTime<Utc>,
    pub delta_us: f64,      // Time since previous packet (microseconds)
    pub packet_id: u64,
    pub src: String,
    pub dst: String,
    pub protocol: String,
    pub length: u32,
}

/// RTT sample from TCP ACK
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RttSample {
    pub timestamp: DateTime<Utc>,
    pub rtt_us: f64,        // Round-trip time (microseconds)
    pub flow: String,       // "src_ip:port -> dst_ip:port"
}

/// Aggregated interval statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalStats {
    pub count: u64,
    pub min_us: f64,
    pub max_us: f64,
    pub avg_us: f64,
    pub std_dev_us: f64,
    pub percentile_50_us: f64,
    pub percentile_95_us: f64,
    pub percentile_99_us: f64,
}

/// Recent samples for graphing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalData {
    pub intervals: Vec<IntervalSample>,
    pub rtt_samples: Vec<RttSample>,
    pub interval_stats: IntervalStats,
    pub rtt_stats: Option<IntervalStats>,
}

/// Packet interval tracker
pub struct IntervalTracker {
    /// Last packet timestamp for delta calculation
    last_packet_time: Option<(Instant, DateTime<Utc>)>,

    /// Recent interval samples (ring buffer, max 1000)
    interval_samples: Vec<IntervalSample>,
    max_samples: usize,

    /// TCP flow states for RTT tracking
    tcp_flows: HashMap<TcpFlowKey, TcpFlowState>,

    /// Recent RTT samples
    rtt_samples: Vec<RttSample>,

    /// All interval values for statistics
    all_intervals: Vec<f64>,
}

impl IntervalTracker {
    pub fn new() -> Self {
        Self {
            last_packet_time: None,
            interval_samples: Vec::with_capacity(1000),
            max_samples: 1000,
            tcp_flows: HashMap::new(),
            rtt_samples: Vec::with_capacity(500),
            all_intervals: Vec::new(),
        }
    }

    /// Process a captured packet
    pub fn process_packet(
        &mut self,
        packet_id: u64,
        timestamp: DateTime<Utc>,
        capture_instant: Instant,
        length: u32,
        src: &str,
        dst: &str,
        protocol: &str,
        // TCP specific
        src_ip: Option<&str>,
        dst_ip: Option<&str>,
        src_port: Option<u16>,
        dst_port: Option<u16>,
        is_tcp: bool,
        tcp_seq: Option<u32>,
        tcp_ack: Option<u32>,
        tcp_flags_ack: bool,
        payload_len: u32,
    ) {
        // Calculate inter-packet interval
        let delta_us = if let Some((last_instant, _)) = self.last_packet_time {
            capture_instant.duration_since(last_instant).as_secs_f64() * 1_000_000.0
        } else {
            0.0
        };

        self.last_packet_time = Some((capture_instant, timestamp));

        // Store interval sample
        let sample = IntervalSample {
            timestamp,
            delta_us,
            packet_id,
            src: src.to_string(),
            dst: dst.to_string(),
            protocol: protocol.to_string(),
            length,
        };

        if self.interval_samples.len() >= self.max_samples {
            self.interval_samples.remove(0);
        }
        self.interval_samples.push(sample);

        // Store for statistics
        if delta_us > 0.0 {
            self.all_intervals.push(delta_us);
            // Keep last 10000 for statistics
            if self.all_intervals.len() > 10000 {
                self.all_intervals.remove(0);
            }
        }

        // TCP RTT tracking
        if is_tcp {
            if let (Some(sip), Some(dip), Some(sp), Some(dp)) = (src_ip, dst_ip, src_port, dst_port) {
                self.track_tcp_rtt(
                    sip, dip, sp, dp,
                    tcp_seq, tcp_ack, tcp_flags_ack,
                    payload_len, capture_instant, timestamp,
                );
            }
        }

        // Cleanup old TCP flows (every 100 packets)
        if packet_id % 100 == 0 {
            self.cleanup_old_flows();
        }
    }

    /// Track TCP RTT from ACK timing
    fn track_tcp_rtt(
        &mut self,
        src_ip: &str,
        dst_ip: &str,
        src_port: u16,
        dst_port: u16,
        seq: Option<u32>,
        ack: Option<u32>,
        is_ack: bool,
        payload_len: u32,
        instant: Instant,
        timestamp: DateTime<Utc>,
    ) {
        let flow_key = TcpFlowKey::new(src_ip, dst_ip, src_port, dst_port);

        // If this packet has payload, record it as pending for ACK
        if let Some(seq_num) = seq {
            if payload_len > 0 {
                let flow = self.tcp_flows.entry(flow_key.clone()).or_default();
                flow.pending_segments.push(PendingSegment {
                    seq_end: seq_num.wrapping_add(payload_len),
                    timestamp: instant,
                });
                flow.last_activity = instant;

                // Keep only last 20 pending segments
                if flow.pending_segments.len() > 20 {
                    flow.pending_segments.remove(0);
                }
            }
        }

        // If this is an ACK, try to match with pending segment on reverse flow
        if is_ack {
            if let Some(ack_num) = ack {
                let reverse_key = flow_key.reverse();

                if let Some(reverse_flow) = self.tcp_flows.get_mut(&reverse_key) {
                    // Find segment that this ACK acknowledges
                    let mut matched_idx = None;
                    let mut rtt_us = 0.0;

                    for (i, seg) in reverse_flow.pending_segments.iter().enumerate() {
                        // ACK acknowledges all data up to ack_num
                        if Self::seq_le(seg.seq_end, ack_num) {
                            rtt_us = instant.duration_since(seg.timestamp).as_secs_f64() * 1_000_000.0;
                            matched_idx = Some(i);
                            break;
                        }
                    }

                    if let Some(idx) = matched_idx {
                        // Remove acknowledged segment
                        reverse_flow.pending_segments.remove(idx);

                        // Record RTT sample
                        if rtt_us > 0.0 && rtt_us < 10_000_000.0 {  // Max 10 seconds
                            reverse_flow.rtt_samples.push(rtt_us);
                            if reverse_flow.rtt_samples.len() > 100 {
                                reverse_flow.rtt_samples.remove(0);
                            }

                            let rtt_sample = RttSample {
                                timestamp,
                                rtt_us,
                                flow: format!("{}:{} -> {}:{}",
                                    reverse_key.src_ip, reverse_key.src_port,
                                    reverse_key.dst_ip, reverse_key.dst_port),
                            };

                            if self.rtt_samples.len() >= 500 {
                                self.rtt_samples.remove(0);
                            }
                            self.rtt_samples.push(rtt_sample);
                        }
                    }
                }
            }
        }
    }

    /// Compare sequence numbers (handles wrap-around)
    fn seq_le(a: u32, b: u32) -> bool {
        let diff = b.wrapping_sub(a);
        diff < 0x80000000
    }

    /// Cleanup old TCP flows
    fn cleanup_old_flows(&mut self) {
        let now = Instant::now();
        let timeout = Duration::from_secs(60);

        self.tcp_flows.retain(|_, flow| {
            now.duration_since(flow.last_activity) < timeout
        });
    }

    /// Calculate statistics from samples
    fn calculate_stats(values: &[f64]) -> IntervalStats {
        if values.is_empty() {
            return IntervalStats {
                count: 0,
                min_us: 0.0,
                max_us: 0.0,
                avg_us: 0.0,
                std_dev_us: 0.0,
                percentile_50_us: 0.0,
                percentile_95_us: 0.0,
                percentile_99_us: 0.0,
            };
        }

        let count = values.len() as u64;
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = values.iter().sum();
        let avg = sum / values.len() as f64;

        // Standard deviation
        let variance: f64 = values.iter()
            .map(|&x| (x - avg).powi(2))
            .sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        // Percentiles (need sorted copy)
        let mut sorted: Vec<f64> = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p50 = Self::percentile(&sorted, 50.0);
        let p95 = Self::percentile(&sorted, 95.0);
        let p99 = Self::percentile(&sorted, 99.0);

        IntervalStats {
            count,
            min_us: min,
            max_us: max,
            avg_us: avg,
            std_dev_us: std_dev,
            percentile_50_us: p50,
            percentile_95_us: p95,
            percentile_99_us: p99,
        }
    }

    fn percentile(sorted: &[f64], p: f64) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let idx = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    /// Get current interval data for API
    pub fn get_data(&self, limit: usize) -> IntervalData {
        let intervals: Vec<IntervalSample> = self.interval_samples
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        let rtt_samples: Vec<RttSample> = self.rtt_samples
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        let interval_stats = Self::calculate_stats(&self.all_intervals);

        let rtt_values: Vec<f64> = self.rtt_samples.iter().map(|s| s.rtt_us).collect();
        let rtt_stats = if !rtt_values.is_empty() {
            Some(Self::calculate_stats(&rtt_values))
        } else {
            None
        };

        IntervalData {
            intervals,
            rtt_samples,
            interval_stats,
            rtt_stats,
        }
    }

    /// Reset all tracking data
    pub fn reset(&mut self) {
        self.last_packet_time = None;
        self.interval_samples.clear();
        self.tcp_flows.clear();
        self.rtt_samples.clear();
        self.all_intervals.clear();
    }
}

impl Default for IntervalTracker {
    fn default() -> Self {
        Self::new()
    }
}
