mod capture;
mod protocols;
mod topology;
mod api;
mod tester;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use clap::Parser;
use pcap::{Capture, Device};
use chrono::Utc;

use crate::capture::{CaptureManager, CapturedPacket};
use crate::topology::TopologyManager;

fn get_web_dir() -> std::path::PathBuf {
    // Try multiple locations for the web directory
    let candidates = vec![
        // Current directory
        std::path::PathBuf::from("web"),
        // Relative to executable
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("../web")))
            .unwrap_or_default(),
        // Project directory (for development)
        std::path::PathBuf::from("/home/kim/tsn-map/web"),
    ];

    for path in candidates {
        if path.exists() && path.join("index.html").exists() {
            tracing::info!("Serving web files from: {:?}", path);
            return path;
        }
    }

    tracing::warn!("Web directory not found, using 'web' as fallback");
    std::path::PathBuf::from("web")
}

#[derive(Parser, Debug)]
#[command(name = "tsn-map")]
#[command(about = "TSN Network Visualization and Analysis Tool")]
struct Args {
    /// Network interface to capture on
    #[arg(short, long, default_value = "enp11s0")]
    interface: String,

    /// Web server port
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Enable promiscuous mode
    #[arg(long, default_value_t = true)]
    promiscuous: bool,

    /// Capture buffer size in MB
    #[arg(long, default_value_t = 64)]
    buffer_size: usize,
}

pub struct AppState {
    pub capture_manager: RwLock<CaptureManager>,
    pub topology_manager: RwLock<TopologyManager>,
    pub interface: String,
    pub is_capturing: Arc<AtomicBool>,
    pub buffer_size: usize,
}

async fn capture_worker(
    state: Arc<AppState>,
) {
    let mut packet_id: u64 = 0;

    loop {
        // Wait until capturing is enabled
        while !state.is_capturing.load(Ordering::SeqCst) {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Get current interface
        let interface = {
            let cm = state.capture_manager.read().await;
            cm.get_interface().to_string()
        };

        tracing::info!("Starting packet capture on {}", interface);

        // Open capture
        let device = match Device::list() {
            Ok(devices) => devices.into_iter().find(|d| d.name == interface),
            Err(e) => {
                tracing::error!("Failed to list devices: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        let device = match device {
            Some(d) => d,
            None => {
                tracing::error!("Interface {} not found", interface);
                state.is_capturing.store(false, Ordering::SeqCst);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        let mut cap = match Capture::from_device(device)
            .map(|c| c.promisc(true))
            .map(|c| c.snaplen(65535))
            .map(|c| c.buffer_size(state.buffer_size as i32))
            .map(|c| c.timeout(100))
            .and_then(|c| c.open())
        {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to open capture: {}", e);
                state.is_capturing.store(false, Ordering::SeqCst);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        tracing::info!("Capture started on {}", interface);

        // Capture loop
        while state.is_capturing.load(Ordering::SeqCst) {
            match cap.next_packet() {
                Ok(packet) => {
                    let captured = CapturedPacket::from_raw(
                        packet_id,
                        packet.data,
                        Utc::now(),
                    );
                    packet_id += 1;

                    // Update topology
                    {
                        let mut tm = state.topology_manager.write().await;
                        tm.process_packet(&captured);
                    }

                    // Add to capture manager
                    {
                        let mut cm = state.capture_manager.write().await;
                        cm.add_packet(captured);
                    }
                }
                Err(pcap::Error::TimeoutExpired) => {
                    // Normal timeout, continue
                    continue;
                }
                Err(e) => {
                    tracing::warn!("Capture error: {}", e);
                    break;
                }
            }
        }

        tracing::info!("Capture stopped");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "tsn_map=info,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    tracing::info!("Starting TSN-Map on interface: {}", args.interface);
    tracing::info!("Web server will run on port: {}", args.port);

    // Initialize application state
    let state = Arc::new(AppState {
        capture_manager: RwLock::new(CaptureManager::new(&args.interface, args.buffer_size)?),
        topology_manager: RwLock::new(TopologyManager::new()),
        interface: args.interface.clone(),
        is_capturing: Arc::new(AtomicBool::new(false)),
        buffer_size: args.buffer_size * 1024 * 1024,
    });

    // Spawn capture worker
    let capture_state = state.clone();
    tokio::spawn(async move {
        capture_worker(capture_state).await;
    });

    // Build router
    let app = Router::new()
        // API routes
        .route("/api/status", get(api::handlers::get_status))
        .route("/api/capture/start", post(api::handlers::start_capture))
        .route("/api/capture/stop", post(api::handlers::stop_capture))
        .route("/api/capture/stats", get(api::handlers::get_capture_stats))
        .route("/api/packets", get(api::handlers::get_packets))
        .route("/api/packets/stream", get(api::handlers::packet_stream))
        .route("/api/topology", get(api::handlers::get_topology))
        .route("/api/topology/scan", post(api::handlers::scan_topology))
        .route("/api/tsn/flows", get(api::handlers::get_tsn_flows))
        .route("/api/tsn/streams", get(api::handlers::get_tsn_streams))
        .route("/api/pcap/save", post(api::handlers::save_pcap))
        .route("/api/pcap/load", post(api::handlers::load_pcap))
        .route("/api/pcap/download", post(api::handlers::download_pcap))
        .route("/api/pcap/upload", post(api::handlers::upload_pcap))
        .route("/api/interfaces", get(api::handlers::list_interfaces))
        .route("/api/interface/set", post(api::handlers::set_interface))
        // Test endpoints
        .route("/api/test/ping", post(api::handlers::ping_test))
        .route("/api/test/ping/stream", get(api::handlers::ping_stream))
        .route("/api/test/throughput", post(api::handlers::throughput_test))
        .route("/api/test/throughput/stream", get(api::handlers::throughput_stream))
        // TSN configuration
        .route("/api/tsn/cbs", post(api::handlers::configure_cbs))
        .route("/api/tsn/tas", post(api::handlers::configure_tas))
        // Hardware timestamp APIs
        .route("/api/timestamp/capability", get(api::handlers::get_timestamp_capability))
        .route("/api/test/hwping/stream", get(api::handlers::hwping_stream))
        // Static files (web frontend)
        .fallback_service(ServeDir::new(get_web_dir()))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port)).await?;
    tracing::info!("TSN-Map server listening on http://0.0.0.0:{}", args.port);

    axum::serve(listener, app).await?;

    Ok(())
}
