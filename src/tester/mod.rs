pub mod latency;
pub mod throughput;

pub use latency::{LatencyTester, LatencyResult, LatencyStats};
pub use throughput::{ThroughputTester, ThroughputResult};
