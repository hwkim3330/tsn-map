use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::capture::CapturedPacket;

/// FRER (Frame Replication and Elimination for Reliability) Stream Stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrerStreamStats {
    pub stream_id: String,
    pub packets_received: u64,
    pub duplicates_eliminated: u64,
    pub sequence_errors: u64,
    pub last_sequence: Option<u32>,
    pub replication_factor: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrerStats {
    pub total_packets: u64,
    pub total_duplicates: u64,
    pub total_sequence_errors: u64,
    pub streams: Vec<FrerStreamStats>,
    pub elimination_rate_pct: f64,
}

struct StreamTracker {
    packets: u64,
    duplicates: u64,
    seq_errors: u64,
    last_seq: Option<u32>,
    seen_sequences: Vec<u32>,
    paths_seen: HashMap<String, u64>,
}

pub struct FrerAnalyzer {
    stats: FrerStats,
    stream_trackers: HashMap<String, StreamTracker>,
}

impl FrerAnalyzer {
    pub fn new() -> Self {
        Self {
            stats: FrerStats::default(),
            stream_trackers: HashMap::new(),
        }
    }

    pub fn process_packet(&mut self, packet: &CapturedPacket) {
        self.stats.total_packets += 1;

        // Get stream ID and sequence number
        let (stream_id, seq_num) = if let Some(ref tsn_info) = packet.tsn_info {
            (
                tsn_info.stream_id.clone().unwrap_or_else(|| {
                    format!("{}:{}", packet.info.src_mac, packet.info.vlan_id.unwrap_or(0))
                }),
                tsn_info.sequence_number,
            )
        } else {
            return;
        };

        // Track this stream
        let tracker = self.stream_trackers.entry(stream_id.clone()).or_insert_with(|| StreamTracker {
            packets: 0,
            duplicates: 0,
            seq_errors: 0,
            last_seq: None,
            seen_sequences: Vec::with_capacity(1000),
            paths_seen: HashMap::new(),
        });

        tracker.packets += 1;

        // Track path (using dst MAC as path identifier)
        let path = packet.info.dst_mac.clone();
        *tracker.paths_seen.entry(path).or_insert(0) += 1;

        // Check sequence number if available
        if let Some(seq) = seq_num {
            // Check for duplicate
            if tracker.seen_sequences.contains(&seq) {
                tracker.duplicates += 1;
                self.stats.total_duplicates += 1;
            } else {
                // Check for sequence error
                if let Some(last) = tracker.last_seq {
                    let expected = last.wrapping_add(1);
                    if seq != expected && seq != last {
                        tracker.seq_errors += 1;
                        self.stats.total_sequence_errors += 1;
                    }
                }

                tracker.seen_sequences.push(seq);
                if tracker.seen_sequences.len() > 1000 {
                    tracker.seen_sequences.remove(0);
                }
                tracker.last_seq = Some(seq);
            }
        }

        // Update stats
        self.update_stats();
    }

    fn update_stats(&mut self) {
        // Update stream stats
        self.stats.streams = self
            .stream_trackers
            .iter()
            .map(|(id, tracker)| {
                let replication_factor = tracker.paths_seen.len() as u8;

                FrerStreamStats {
                    stream_id: id.clone(),
                    packets_received: tracker.packets,
                    duplicates_eliminated: tracker.duplicates,
                    sequence_errors: tracker.seq_errors,
                    last_sequence: tracker.last_seq,
                    replication_factor,
                }
            })
            .collect();

        // Calculate elimination rate
        if self.stats.total_packets > 0 {
            self.stats.elimination_rate_pct =
                (self.stats.total_duplicates as f64 / self.stats.total_packets as f64) * 100.0;
        }
    }

    pub fn get_stats(&self) -> &FrerStats {
        &self.stats
    }

    pub fn get_stream_stats(&self, stream_id: &str) -> Option<FrerStreamStats> {
        self.stream_trackers.get(stream_id).map(|tracker| {
            FrerStreamStats {
                stream_id: stream_id.to_string(),
                packets_received: tracker.packets,
                duplicates_eliminated: tracker.duplicates,
                sequence_errors: tracker.seq_errors,
                last_sequence: tracker.last_seq,
                replication_factor: tracker.paths_seen.len() as u8,
            }
        })
    }
}

impl Default for FrerAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
