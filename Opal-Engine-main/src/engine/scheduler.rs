use opal_lib::{apply_optimization, OptimizationConfig, OptimizationReport};

pub struct OpalScheduler {
    pub active: bool,
    config: OptimizationConfig,
}

impl OpalScheduler {
    pub fn new() -> Self {
        Self {
            active: true,
            config: OptimizationConfig::default(),
        }
    }

    pub fn with_config(config: OptimizationConfig) -> Self {
        Self {
            active: true,
            config,
        }
    }

    pub fn optimize(&self, cpu_load: f32, memory_load: f32) -> OptimizationReport {
        apply_optimization(cpu_load, memory_load, &self.config)
    }
}
