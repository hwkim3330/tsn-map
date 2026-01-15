use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::capture::CapturedPacket;

/// TAS (Time-Aware Shaper) Gate Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateEntry {
    pub gate_states: u8,  // Bitmask of open gates
    pub time_interval_ns: u32,
}

/// TAS Schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasSchedule {
    pub cycle_time_ns: u64,
    pub cycle_time_extension_ns: u32,
    pub base_time: Option<DateTime<Utc>>,
    pub entries: Vec<GateEntry>,
}

/// Per-queue TAS statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasQueueStats {
    pub queue: u8,
    pub packets: u64,
    pub bytes: u64,
    pub avg_latency_us: Option<f64>,
    pub max_latency_us: Option<f64>,
    pub gate_open_time_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TasStats {
    pub total_packets: u64,
    pub detected_cycle_time_ms: Option<f64>,
    pub queue_stats: Vec<TasQueueStats>,
    pub schedule: Option<TasSchedule>,
    pub timing_violations: u64,
}

struct QueueTracker {
    packets: u64,
    bytes: u64,
    timestamps: Vec<DateTime<Utc>>,
    inter_arrival_times: Vec<f64>,
}

pub struct TasAnalyzer {
    stats: TasStats,
    queue_trackers: HashMap<u8, QueueTracker>,
    cycle_detector: CycleDetector,
}

struct CycleDetector {
    samples: Vec<f64>,
    detected_cycle: Option<f64>,
}

impl CycleDetector {
    fn new() -> Self {
        Self {
            samples: Vec::with_capacity(1000),
            detected_cycle: None,
        }
    }

    fn add_sample(&mut self, interval_ms: f64) {
        self.samples.push(interval_ms);
        if self.samples.len() > 1000 {
            self.samples.remove(0);
        }

        // Try to detect cycle time using autocorrelation
        if self.samples.len() >= 100 {
            self.detect_cycle();
        }
    }

    fn detect_cycle(&mut self) {
        // Simple cycle detection: find most common interval
        let mut interval_counts: HashMap<u32, u32> = HashMap::new();

        for &interval in &self.samples {
            // Quantize to 1ms bins
            let bin = (interval * 10.0).round() as u32; // 0.1ms resolution
            *interval_counts.entry(bin).or_insert(0) += 1;
        }

        // Find the most common interval
        if let Some((&bin, &count)) = interval_counts.iter().max_by_key(|(_, &c)| c) {
            if count > self.samples.len() as u32 / 10 {
                self.detected_cycle = Some(bin as f64 / 10.0);
            }
        }
    }
}

impl TasAnalyzer {
    pub fn new() -> Self {
        Self {
            stats: TasStats::default(),
            queue_trackers: HashMap::new(),
            cycle_detector: CycleDetector::new(),
        }
    }

    pub fn process_packet(&mut self, packet: &CapturedPacket) {
        let queue = packet.info.vlan_pcp.unwrap_or(0);

        self.stats.total_packets += 1;

        // Update queue tracker
        let tracker = self.queue_trackers.entry(queue).or_insert_with(|| QueueTracker {
            packets: 0,
            bytes: 0,
            timestamps: Vec::with_capacity(1000),
            inter_arrival_times: Vec::with_capacity(1000),
        });

        // Calculate inter-arrival time
        if let Some(last_time) = tracker.timestamps.last() {
            let interval = (packet.timestamp - *last_time).num_microseconds().unwrap_or(0) as f64 / 1000.0;
            if interval > 0.0 && interval < 10000.0 {
                tracker.inter_arrival_times.push(interval);
                if tracker.inter_arrival_times.len() > 1000 {
                    tracker.inter_arrival_times.remove(0);
                }

                // Feed to cycle detector
                self.cycle_detector.add_sample(interval);
            }
        }

        tracker.packets += 1;
        tracker.bytes += packet.length as u64;
        tracker.timestamps.push(packet.timestamp);
        if tracker.timestamps.len() > 1000 {
            tracker.timestamps.remove(0);
        }

        // Update stats
        self.update_stats();
    }

    fn update_stats(&mut self) {
        // Update detected cycle time
        self.stats.detected_cycle_time_ms = self.cycle_detector.detected_cycle;

        // Update queue stats
        self.stats.queue_stats = self
            .queue_trackers
            .iter()
            .map(|(&queue, tracker)| {
                let (avg_latency, max_latency) = if !tracker.inter_arrival_times.is_empty() {
                    let avg = tracker.inter_arrival_times.iter().sum::<f64>()
                        / tracker.inter_arrival_times.len() as f64;
                    let max = tracker
                        .inter_arrival_times
                        .iter()
                        .cloned()
                        .fold(f64::NEG_INFINITY, f64::max);
                    (Some(avg * 1000.0), Some(max * 1000.0)) // Convert to microseconds
                } else {
                    (None, None)
                };

                TasQueueStats {
                    queue,
                    packets: tracker.packets,
                    bytes: tracker.bytes,
                    avg_latency_us: avg_latency,
                    max_latency_us: max_latency,
                    gate_open_time_pct: 0.0, // Would need schedule info
                }
            })
            .collect();

        self.stats.queue_stats.sort_by_key(|q| q.queue);
    }

    pub fn set_schedule(&mut self, schedule: TasSchedule) {
        self.stats.schedule = Some(schedule);
    }

    pub fn get_stats(&self) -> &TasStats {
        &self.stats
    }
}

impl Default for TasAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
