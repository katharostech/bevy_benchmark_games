use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Metrics {
    pub iterations: Vec<IterationMetrics>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IterationMetrics {
    pub cpu_cycles: u64,
    pub cpu_instructions: u64,
    pub avg_frame_time_us: f64,
}
