mod engine;
mod modules;
use chrono::Local;
use opal_lib::{apply_optimization, OptimizationConfig};
use std::{thread, time::Duration};
use sysinfo::System;

fn main() {
    let mut sys = System::new_all();
    let config = OptimizationConfig::default();
    println!("[{}] Opal Core: INITIALIZED", Local::now().format("%H:%M:%S"));

    loop {
        sys.refresh_cpu();
        sys.refresh_memory();

        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let memory_total = sys.total_memory();
        let memory_used = sys.used_memory();
        let memory_usage = if memory_total > 0 {
            (memory_used as f64 / memory_total as f64 * 100.0) as f32
        } else {
            0.0
        };

        let report = apply_optimization(cpu_usage, memory_usage, &config);
        if report.applied {
            match report.selected_priority {
                opal_lib::ProcessPriority::Realtime => {
                    println!("[{}] CRITICAL LOAD: CPU {:.2}% | MEM {:.2}% -> REALTIME", Local::now().format("%H:%M:%S"), cpu_usage, memory_usage);
                }
                opal_lib::ProcessPriority::High => {
                    println!("[{}] HIGH LOAD: CPU {:.2}% | MEM {:.2}% -> HIGH", Local::now().format("%H:%M:%S"), cpu_usage, memory_usage);
                }
                opal_lib::ProcessPriority::Normal => {
                    println!("[{}] STABLE LOAD: CPU {:.2}% | MEM {:.2}% -> NORMAL", Local::now().format("%H:%M:%S"), cpu_usage, memory_usage);
                }
            }
        }

        thread::sleep(Duration::from_millis(config.sample_interval_ms as u64));
    }
}