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
    State(state): State<Arc<AppState>>,
    Json(request): Json<ScanRequest>,
) -> Json<ApiResponse<ScanResponse>> {
    use crate::topology::scanner::TopologyScanner;

    let interface = request.interface.unwrap_or_else(|| state.interface.clone());
    let network = request.network.unwrap_or_else(|| "192.168.1.0/24".to_string());
    let quick = request.quick.unwrap_or(false);

    let result = tokio::task::spawn_blocking(move || {
        let scanner = TopologyScanner::new(&interface);
        if quick {
            scanner.quick_scan()
        } else {
            scanner.arp_scan(&network)
        }
    }).await;

    match result {
        Ok(Ok(scan_result)) => {
            Json(ApiResponse::success(ScanResponse {
                hosts_found: scan_result.hosts_found,
                hosts: scan_result.hosts.into_iter().map(|h| DiscoveredHostResponse {
                    ip: h.ip,
                    mac: h.mac,
                    hostname: h.hostname,
                    vendor: h.vendor,
                    response_time_ms: h.response_time_ms,
                }).collect(),
                scan_duration_ms: scan_result.scan_duration_ms,
                network: scan_result.network,
            }))
        }
        Ok(Err(e)) => Json(ApiResponse::error(&format!("Scan failed: {}", e))),
        Err(e) => Json(ApiResponse::error(&format!("Task error: {}", e))),
    }
}

#[derive(Deserialize)]
pub struct ScanRequest {
    pub interface: Option<String>,
    pub network: Option<String>,
    pub quick: Option<bool>,
}

#[derive(Serialize)]
pub struct ScanResponse {
    pub hosts_found: u32,
    pub hosts: Vec<DiscoveredHostResponse>,
    pub scan_duration_ms: u64,
    pub network: String,
}

#[derive(Serialize)]
pub struct DiscoveredHostResponse {
    pub ip: String,
    pub mac: String,
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub response_time_ms: f64,
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

#[derive(Clone, Serialize)]
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
    use crate::tester::latency::icmp;
    use std::net::Ipv4Addr;

    let count = request.count.unwrap_or(10).min(100);
    let interval = request.interval.unwrap_or(1000);

    // Parse target IP
    let target_ip: Ipv4Addr = match request.target.parse() {
        Ok(ip) => ip,
        Err(_) => {
            // Try DNS resolution
            match tokio::net::lookup_host(format!("{}:0", request.target)).await {
                Ok(mut addrs) => {
                    match addrs.next() {
                        Some(addr) => match addr.ip() {
                            std::net::IpAddr::V4(ip) => ip,
                            _ => return Json(ApiResponse::error("IPv6 not supported yet")),
                        },
                        None => return Json(ApiResponse::error("Could not resolve hostname")),
                    }
                }
                Err(_) => return Json(ApiResponse::error("Could not resolve hostname")),
            }
        }
    };

    // Run ICMP ping test in blocking task (requires raw socket)
    let result = tokio::task::spawn_blocking(move || {
        icmp::run_icmp_test(target_ip, count, interval)
    }).await;

    match result {
        Ok((results, stats)) => {
            let ping_results: Vec<PingResult> = results.iter().map(|r| PingResult {
                success: r.success,
                rtt_ms: r.rtt_us / 1000.0,
                ttl: None,
            }).collect();

            let ping_stats = PingStats {
                min_ms: stats.min_us / 1000.0,
                avg_ms: stats.avg_us / 1000.0,
                max_ms: stats.max_us / 1000.0,
                loss_percent: stats.loss_percent,
            };

            Json(ApiResponse::success(PingResponse {
                results: ping_results,
                stats: ping_stats,
            }))
        }
        Err(e) => Json(ApiResponse::error(&format!("Ping test failed: {}", e))),
    }
}

// GET /api/test/ping/stream - SSE streaming ping test
#[derive(Deserialize)]
pub struct PingStreamParams {
    pub target: String,
    pub count: Option<u32>,
    pub interval: Option<u32>,
}

pub async fn ping_stream(
    Query(params): Query<PingStreamParams>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    use crate::tester::latency::icmp::ping_icmp;
    use std::net::Ipv4Addr;
    use async_stream::stream;
    use tokio::time::{sleep, Duration};

    let count = params.count.unwrap_or(10).min(100);
    let interval = params.interval.unwrap_or(1000);

    // Parse target IP
    let target_ip: Option<Ipv4Addr> = params.target.parse().ok().or_else(|| {
        // Sync DNS lookup for simplicity (in stream context)
        std::net::ToSocketAddrs::to_socket_addrs(&format!("{}:0", params.target))
            .ok()
            .and_then(|mut addrs| addrs.next())
            .and_then(|addr| match addr.ip() {
                std::net::IpAddr::V4(ip) => Some(ip),
                _ => None,
            })
    });

    let stream = stream! {
        let Some(target) = target_ip else {
            yield Ok(Event::default().event("error").data("Could not resolve hostname"));
            return;
        };

        let mut results: Vec<PingResult> = Vec::new();

        for seq in 0..count {
            // Run ping in blocking task
            let t = target;
            let result = tokio::task::spawn_blocking(move || {
                ping_icmp(t, seq as u16, 2000)
            }).await;

            let ping_result = match result {
                Ok(Some(rtt_ms)) => PingResult {
                    success: true,
                    rtt_ms,
                    ttl: None,
                },
                _ => PingResult {
                    success: false,
                    rtt_ms: 0.0,
                    ttl: None,
                },
            };

            results.push(ping_result.clone());

            // Send individual result
            let data = serde_json::json!({
                "seq": seq,
                "success": ping_result.success,
                "rtt_ms": ping_result.rtt_ms,
            });
            yield Ok(Event::default().event("ping").data(data.to_string()));

            if seq < count - 1 {
                sleep(Duration::from_millis(interval as u64)).await;
            }
        }

        // Send final stats
        let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
        let stats = if successful.is_empty() {
            PingStats {
                min_ms: 0.0,
                avg_ms: 0.0,
                max_ms: 0.0,
                loss_percent: 100.0,
            }
        } else {
            let rtts: Vec<f64> = successful.iter().map(|r| r.rtt_ms).collect();
            let min = rtts.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = rtts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let avg = rtts.iter().sum::<f64>() / rtts.len() as f64;
            let loss = ((results.len() - successful.len()) as f64 / results.len() as f64) * 100.0;
            PingStats { min_ms: min, avg_ms: avg, max_ms: max, loss_percent: loss }
        };

        let final_data = serde_json::json!({
            "stats": stats,
            "total": results.len(),
        });
        yield Ok(Event::default().event("complete").data(final_data.to_string()));
    };

    Sse::new(stream)
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
    use crate::tester::throughput::{ThroughputTester, ThroughputServer};
    use std::net::{IpAddr, SocketAddr};

    let duration = request.duration.unwrap_or(10).min(60);
    let mode = request.mode.clone().unwrap_or_else(|| "client".to_string());
    let bandwidth = request.bandwidth.map(|b| b as u64 * 1_000_000); // Mbps to bps
    let packet_size = 1400usize; // MTU-safe default

    if mode == "server" {
        // Run as server
        let result = tokio::task::spawn_blocking(move || {
            let server = ThroughputServer::new("0.0.0.0", Some(7879))?;
            // Run for duration + some buffer
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs((duration + 5) as u64));
            });
            server.run()
        }).await;

        match result {
            Ok(Ok(r)) => Json(ApiResponse::success(ThroughputResponse {
                bandwidth_bps: r.bandwidth_bps as u64,
                packets_sent: r.packets_sent,
                packets_received: r.packets_received,
                loss_percent: r.packet_loss_percent,
                jitter_ms: None,
            })),
            Ok(Err(e)) => Json(ApiResponse::error(&format!("Server error: {}", e))),
            Err(e) => Json(ApiResponse::error(&format!("Task error: {}", e))),
        }
    } else {
        // Run as client
        let target = match &request.target {
            Some(t) => t.clone(),
            None => return Json(ApiResponse::error("Target required for client mode")),
        };

        // Parse target (ip:port or just ip)
        let target_addr: SocketAddr = if target.contains(':') {
            match target.parse() {
                Ok(addr) => addr,
                Err(_) => return Json(ApiResponse::error("Invalid target address")),
            }
        } else {
            // Try to parse as IP, default port 7879
            match target.parse::<IpAddr>() {
                Ok(ip) => SocketAddr::new(ip, 7879),
                Err(_) => {
                    // Try DNS
                    match tokio::net::lookup_host(format!("{}:7879", target)).await {
                        Ok(mut addrs) => match addrs.next() {
                            Some(addr) => addr,
                            None => return Json(ApiResponse::error("Could not resolve hostname")),
                        },
                        Err(_) => return Json(ApiResponse::error("Could not resolve hostname")),
                    }
                }
            }
        };

        let result = tokio::task::spawn_blocking(move || {
            let mut tester = ThroughputTester::new(target_addr.ip(), Some(target_addr.port()))
                .with_packet_size(packet_size);

            if let Some(bw) = bandwidth {
                tester = tester.with_bandwidth_limit(bw);
            }

            tester.run_client(duration)
        }).await;

        match result {
            Ok(Ok(r)) => Json(ApiResponse::success(ThroughputResponse {
                bandwidth_bps: r.bandwidth_bps as u64,
                packets_sent: r.packets_sent,
                packets_received: r.packets_received,
                loss_percent: r.packet_loss_percent,
                jitter_ms: None,
            })),
            Ok(Err(e)) => Json(ApiResponse::error(&format!("Test error: {}", e))),
            Err(e) => Json(ApiResponse::error(&format!("Task error: {}", e))),
        }
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
