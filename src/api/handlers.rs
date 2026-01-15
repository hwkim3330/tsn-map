use std::sync::Arc;
use std::path::PathBuf;
use axum::{
    extract::{State, Query, Multipart},
    response::sse::{Event, Sse},
    response::IntoResponse,
    http::{header, StatusCode},
    body::Body,
    Json,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::capture::{self, CapturedPacket, PcapHandler};
use crate::protocols::{TsnStream, TsnFlow};
use crate::topology::NetworkTopology;

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub interface: String,
    pub is_capturing: bool,
    pub packets_captured: u64,
    pub uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct CaptureStatsResponse {
    pub packets_captured: u64,
    pub bytes_captured: u64,
    pub packets_dropped: u64,
    pub tsn_packets: u64,
    pub ptp_packets: u64,
    pub capture_rate: f64,
    pub is_capturing: bool,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct PacketsResponse {
    pub packets: Vec<CapturedPacket>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Deserialize)]
pub struct SavePcapRequest {
    pub filename: String,
}

#[derive(Serialize)]
pub struct SavePcapResponse {
    pub success: bool,
    pub filename: String,
    pub packets_saved: usize,
}

#[derive(Deserialize)]
pub struct LoadPcapRequest {
    pub filename: String,
}

#[derive(Serialize)]
pub struct LoadPcapResponse {
    pub success: bool,
    pub filename: String,
    pub packets_loaded: usize,
}

#[derive(Deserialize)]
pub struct SetInterfaceRequest {
    pub interface: String,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }
}

// GET /api/status
pub async fn get_status(
    State(state): State<Arc<AppState>>,
) -> Json<StatusResponse> {
    let capture = state.capture_manager.read().await;

    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        interface: state.interface.clone(),
        is_capturing: capture.is_capturing(),
        packets_captured: capture.get_stats().packets_captured,
        uptime_seconds: 0,
    })
}

// POST /api/capture/start
pub async fn start_capture(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<String>> {
    use std::sync::atomic::Ordering;

    // Set the capture flag
    state.is_capturing.store(true, Ordering::SeqCst);

    // Also update capture manager state
    let mut capture = state.capture_manager.write().await;
    match capture.start_capture() {
        Ok(_) => Json(ApiResponse::success("Capture started".to_string())),
        Err(e) => {
            state.is_capturing.store(false, Ordering::SeqCst);
            Json(ApiResponse::error(&format!("Failed to start capture: {}", e)))
        }
    }
}

// POST /api/capture/stop
pub async fn stop_capture(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<String>> {
    use std::sync::atomic::Ordering;

    // Clear the capture flag
    state.is_capturing.store(false, Ordering::SeqCst);

    let mut capture = state.capture_manager.write().await;
    capture.stop_capture();
    Json(ApiResponse::success("Capture stopped".to_string()))
}

// GET /api/capture/stats
pub async fn get_capture_stats(
    State(state): State<Arc<AppState>>,
) -> Json<CaptureStatsResponse> {
    let capture = state.capture_manager.read().await;
    let stats = capture.get_stats();

    Json(CaptureStatsResponse {
        packets_captured: stats.packets_captured,
        bytes_captured: stats.bytes_captured,
        packets_dropped: stats.packets_dropped,
        tsn_packets: stats.tsn_packets,
        ptp_packets: stats.ptp_packets,
        capture_rate: stats.capture_rate,
        is_capturing: capture.is_capturing(),
    })
}

// GET /api/packets
pub async fn get_packets(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Json<PacketsResponse> {
    let capture = state.capture_manager.read().await;
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(100).min(1000);

    let packets = capture.get_packets(offset, limit);

    Json(PacketsResponse {
        total: capture.get_packet_count(),
        packets,
        offset,
        limit,
    })
}

// GET /api/packets/stream (SSE endpoint for real-time packets)
pub async fn packet_stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let capture = state.capture_manager.read().await;
    let mut rx = capture.subscribe();
    drop(capture);

    let stream = async_stream::stream! {
        while let Ok(packet) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&packet) {
                yield Ok(Event::default().data(json));
            }
        }
    };

    Sse::new(stream)
}

// GET /api/topology
pub async fn get_topology(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<NetworkTopology>> {
    let topology = state.topology_manager.read().await;
    Json(ApiResponse::success(topology.get_topology()))
}

// POST /api/topology/scan
pub async fn scan_topology(
    State(_state): State<Arc<AppState>>,
) -> Json<ApiResponse<String>> {
    // Topology is updated automatically from packet capture
    // This endpoint can trigger active scanning in the future
    Json(ApiResponse::success("Topology scan initiated".to_string()))
}

// GET /api/tsn/flows
pub async fn get_tsn_flows(
    State(_state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<TsnFlow>>> {
    // TODO: Integrate with protocol analyzer
    Json(ApiResponse::success(vec![]))
}

// GET /api/tsn/streams
pub async fn get_tsn_streams(
    State(_state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<TsnStream>>> {
    // TODO: Integrate with protocol analyzer
    Json(ApiResponse::success(vec![]))
}

// POST /api/pcap/save
pub async fn save_pcap(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SavePcapRequest>,
) -> Json<ApiResponse<SavePcapResponse>> {
    let capture = state.capture_manager.read().await;
    let packets = capture.get_packets(0, capture.get_packet_count());
    drop(capture);

    let path = PathBuf::from(&request.filename);

    match PcapHandler::save_pcap(&packets, &path) {
        Ok(count) => Json(ApiResponse::success(SavePcapResponse {
            success: true,
            filename: request.filename,
            packets_saved: count,
        })),
        Err(e) => Json(ApiResponse::error(&format!("Failed to save pcap: {}", e))),
    }
}

// POST /api/pcap/load
pub async fn load_pcap(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoadPcapRequest>,
) -> Json<ApiResponse<LoadPcapResponse>> {
    let path = PathBuf::from(&request.filename);

    match PcapHandler::load_pcap(&path) {
        Ok(packets) => {
            let count = packets.len();
            let mut capture = state.capture_manager.write().await;
            capture.clear_packets();
            for packet in packets {
                capture.add_packet(packet);
            }

            Json(ApiResponse::success(LoadPcapResponse {
                success: true,
                filename: request.filename,
                packets_loaded: count,
            }))
        }
        Err(e) => Json(ApiResponse::error(&format!("Failed to load pcap: {}", e))),
    }
}

// GET /api/interfaces
pub async fn list_interfaces() -> Json<ApiResponse<Vec<capture::InterfaceInfo>>> {
    match capture::list_interfaces() {
        Ok(interfaces) => Json(ApiResponse::success(interfaces)),
        Err(e) => Json(ApiResponse::error(&format!("Failed to list interfaces: {}", e))),
    }
}

// POST /api/interface/set
pub async fn set_interface(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SetInterfaceRequest>,
) -> Json<ApiResponse<String>> {
    let mut capture = state.capture_manager.write().await;

    match capture.set_interface(&request.interface) {
        Ok(_) => Json(ApiResponse::success(format!("Interface set to {}", request.interface))),
        Err(e) => Json(ApiResponse::error(&format!("Failed to set interface: {}", e))),
    }
}

// POST /api/pcap/download - Download captured packets as PCAP file
pub async fn download_pcap(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let capture = state.capture_manager.read().await;
    let packets = capture.get_packets(0, capture.get_packet_count());
    drop(capture);

    if packets.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            Body::from(r#"{"error": "No packets to download"}"#),
        ).into_response();
    }

    // Create PCAP in memory
    match PcapHandler::save_pcap_to_bytes(&packets) {
        Ok(bytes) => {
            let headers = [
                (header::CONTENT_TYPE, "application/vnd.tcpdump.pcap"),
                (header::CONTENT_DISPOSITION, "attachment; filename=\"capture.pcap\""),
            ];
            (StatusCode::OK, headers, Body::from(bytes)).into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(format!(r#"{{"error": "{}"}}"#, e)),
            ).into_response()
        }
    }
}

// POST /api/pcap/upload - Upload and load PCAP file
pub async fn upload_pcap(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<ApiResponse<LoadPcapResponse>> {
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("upload.pcap").to_string();

            match field.bytes().await {
                Ok(data) => {
                    match PcapHandler::load_pcap_from_bytes(&data) {
                        Ok(packets) => {
                            let count = packets.len();
                            let mut capture = state.capture_manager.write().await;
                            capture.clear_packets();

                            // Also update topology from loaded packets
                            let mut topology = state.topology_manager.write().await;
                            topology.clear();

                            for packet in packets {
                                topology.process_packet(&packet);
                                capture.add_packet(packet);
                            }

                            return Json(ApiResponse::success(LoadPcapResponse {
                                success: true,
                                filename,
                                packets_loaded: count,
                            }));
                        }
                        Err(e) => {
                            return Json(ApiResponse::error(&format!("Failed to parse pcap: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    return Json(ApiResponse::error(&format!("Failed to read file: {}", e)));
                }
            }
        }
    }

    Json(ApiResponse::error("No file provided"))
}

// ============================================
// Test Endpoints
// ============================================

#[derive(Deserialize)]
pub struct PingRequest {
    pub target: String,
    pub count: Option<u32>,
    pub interval: Option<u32>,
}

#[derive(Serialize)]
pub struct PingResult {
    pub success: bool,
    pub rtt_ms: f64,
    pub ttl: Option<u8>,
}

#[derive(Serialize)]
pub struct PingStats {
    pub min_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
    pub loss_percent: f64,
}

#[derive(Serialize)]
pub struct PingResponse {
    pub results: Vec<PingResult>,
    pub stats: PingStats,
}

// POST /api/test/ping
pub async fn ping_test(
    Json(request): Json<PingRequest>,
) -> Json<ApiResponse<PingResponse>> {
    use std::process::Command;
    use std::time::Instant;

    let count = request.count.unwrap_or(10).min(100);
    let interval = request.interval.unwrap_or(1000);
    let interval_sec = (interval as f64 / 1000.0).max(0.2);

    let mut results = Vec::new();
    let mut rtts = Vec::new();

    for _ in 0..count {
        let start = Instant::now();

        // Use system ping command
        let output = Command::new("ping")
            .args(["-c", "1", "-W", "2", &request.target])
            .output();

        match output {
            Ok(out) => {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                let stdout = String::from_utf8_lossy(&out.stdout);

                if out.status.success() {
                    // Parse RTT from ping output
                    let rtt = parse_ping_rtt(&stdout).unwrap_or(elapsed);
                    let ttl = parse_ping_ttl(&stdout);

                    results.push(PingResult {
                        success: true,
                        rtt_ms: rtt,
                        ttl,
                    });
                    rtts.push(rtt);
                } else {
                    results.push(PingResult {
                        success: false,
                        rtt_ms: 0.0,
                        ttl: None,
                    });
                }
            }
            Err(_) => {
                results.push(PingResult {
                    success: false,
                    rtt_ms: 0.0,
                    ttl: None,
                });
            }
        }

        // Wait for interval
        tokio::time::sleep(tokio::time::Duration::from_secs_f64(interval_sec)).await;
    }

    // Calculate stats
    let stats = if !rtts.is_empty() {
        let min = rtts.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = rtts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg = rtts.iter().sum::<f64>() / rtts.len() as f64;
        let loss = ((count as usize - rtts.len()) as f64 / count as f64) * 100.0;

        PingStats {
            min_ms: min,
            avg_ms: avg,
            max_ms: max,
            loss_percent: loss,
        }
    } else {
        PingStats {
            min_ms: 0.0,
            avg_ms: 0.0,
            max_ms: 0.0,
            loss_percent: 100.0,
        }
    };

    Json(ApiResponse::success(PingResponse { results, stats }))
}

fn parse_ping_rtt(output: &str) -> Option<f64> {
    // Parse "time=X.XX ms" from ping output
    for line in output.lines() {
        if let Some(idx) = line.find("time=") {
            let rest = &line[idx + 5..];
            if let Some(end) = rest.find(" ms") {
                if let Ok(rtt) = rest[..end].parse::<f64>() {
                    return Some(rtt);
                }
            }
        }
    }
    None
}

fn parse_ping_ttl(output: &str) -> Option<u8> {
    // Parse "ttl=XX" from ping output
    for line in output.lines() {
        if let Some(idx) = line.find("ttl=") {
            let rest = &line[idx + 4..];
            if let Some(end) = rest.find(char::is_whitespace) {
                if let Ok(ttl) = rest[..end].parse::<u8>() {
                    return Some(ttl);
                }
            } else if let Ok(ttl) = rest.parse::<u8>() {
                return Some(ttl);
            }
        }
    }
    None
}

#[derive(Deserialize)]
pub struct ThroughputRequest {
    pub target: Option<String>,
    pub duration: Option<u32>,
    pub protocol: Option<String>,
    pub bandwidth: Option<u32>,
    pub mode: Option<String>,
}

#[derive(Serialize)]
pub struct ThroughputResponse {
    pub bandwidth_bps: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub loss_percent: f64,
    pub jitter_ms: Option<f64>,
}

// POST /api/test/throughput
pub async fn throughput_test(
    Json(request): Json<ThroughputRequest>,
) -> Json<ApiResponse<ThroughputResponse>> {
    use std::process::Command;

    let duration = request.duration.unwrap_or(10).min(60);
    let protocol = request.protocol.clone().unwrap_or_else(|| "tcp".to_string());
    let mode = request.mode.clone().unwrap_or_else(|| "client".to_string());

    // Use iperf3 for throughput testing
    let mut args = vec!["-J".to_string()]; // JSON output

    if mode == "server" {
        args.push("-s".to_string());
        args.push("-1".to_string()); // One-off mode
    } else {
        if let Some(ref target) = request.target {
            args.push("-c".to_string());
            args.push(target.clone());
        } else {
            return Json(ApiResponse::error("Target required for client mode"));
        }
    }

    args.push("-t".to_string());
    args.push(duration.to_string());

    if protocol == "udp" {
        args.push("-u".to_string());
        if let Some(bw) = request.bandwidth {
            args.push("-b".to_string());
            args.push(format!("{}M", bw));
        }
    }

    let output = Command::new("iperf3")
        .args(&args)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);

            // Parse iperf3 JSON output
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                let end = json.get("end").and_then(|e| e.get("sum"));

                if let Some(sum) = end {
                    let bits_per_second = sum.get("bits_per_second")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as u64;

                    let packets = sum.get("packets")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    let lost = sum.get("lost_packets")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    let loss_percent = if packets > 0 {
                        (lost as f64 / packets as f64) * 100.0
                    } else {
                        0.0
                    };

                    let jitter = sum.get("jitter_ms")
                        .and_then(|v| v.as_f64());

                    return Json(ApiResponse::success(ThroughputResponse {
                        bandwidth_bps: bits_per_second,
                        packets_sent: packets,
                        packets_received: packets.saturating_sub(lost),
                        loss_percent,
                        jitter_ms: jitter,
                    }));
                }
            }

            Json(ApiResponse::error("Failed to parse iperf3 output"))
        }
        Err(e) => Json(ApiResponse::error(&format!("Failed to run iperf3: {}. Make sure iperf3 is installed.", e))),
    }
}

// ============================================
// TSN Configuration Endpoints
// ============================================

#[derive(Deserialize)]
pub struct CbsConfig {
    pub interface: String,
    pub traffic_class: u8,
    pub idle_slope: i64,
    pub send_slope: i64,
}

// POST /api/tsn/cbs
pub async fn configure_cbs(
    Json(config): Json<CbsConfig>,
) -> Json<ApiResponse<String>> {
    use std::process::Command;

    // Use tc command to configure CBS
    // tc qdisc replace dev <iface> parent <handle> cbs idleslope <val> sendslope <val> hicredit <val> locredit <val>
    let parent = format!("100:{}", config.traffic_class + 1);

    let output = Command::new("tc")
        .args([
            "qdisc", "replace", "dev", &config.interface,
            "parent", &parent,
            "cbs",
            "idleslope", &config.idle_slope.to_string(),
            "sendslope", &config.send_slope.to_string(),
            "hicredit", "0",
            "locredit", "0",
        ])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                Json(ApiResponse::success("CBS configured successfully".to_string()))
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                Json(ApiResponse::error(&format!("Failed to configure CBS: {}", stderr)))
            }
        }
        Err(e) => Json(ApiResponse::error(&format!("Failed to run tc: {}", e))),
    }
}

#[derive(Deserialize)]
pub struct GateControlEntry {
    pub tc: u8,
    pub gate_state: u32,
    pub interval: u64,
}

#[derive(Deserialize)]
pub struct TasConfig {
    pub cycle_time: u64,
    pub base_time: String,
    pub gate_control_list: Vec<GateControlEntry>,
}

// POST /api/tsn/tas
pub async fn configure_tas(
    Json(_config): Json<TasConfig>,
) -> Json<ApiResponse<String>> {
    // TAS configuration requires specific hardware support and complex tc setup
    // This is a placeholder - actual implementation depends on the NIC capabilities

    Json(ApiResponse::error("TAS configuration requires specific hardware support. Use tc-taprio or vendor-specific tools."))
}
