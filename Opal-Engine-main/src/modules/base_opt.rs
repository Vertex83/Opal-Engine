use crate::engine::scheduler::OpalScheduler;
use opal_lib::{OptimizationConfig, OptimizationReport};

pub fn apply_ultra_optimization(
    cpu_load: f32,
    memory_load: f32,
    config: Option<OptimizationConfig>,
) -> OptimizationReport {
    let scheduler = match config {
        Some(cfg) => OpalScheduler::with_config(cfg),
        None => OpalScheduler::new(),
    };

    scheduler.optimize(cpu_load, memory_load)
}