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
