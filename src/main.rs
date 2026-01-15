mod capture;
mod protocols;
mod topology;
mod api;

use std::sync::Arc;
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

use crate::capture::CaptureManager;
use crate::topology::TopologyManager;

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "tsn_map=debug,tower_http=debug".into()),
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
        .route("/api/interfaces", get(api::handlers::list_interfaces))
        .route("/api/interface/set", post(api::handlers::set_interface))
        // Static files (web frontend)
        .nest_service("/", ServeDir::new("web"))
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
