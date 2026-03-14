mod engine;
mod modules;
use sysinfo::System;
use std::{thread, time::Duration};
use chrono::Local;

fn main() {
    let mut sys = System::new_all();
    println!("[{}] Opal Core: INITIALIZED", Local::now().format("%H:%M:%S"));

    loop {
        sys.refresh_cpu();
        let cpu_usage = sys.global_cpu_info().cpu_usage();
        
        let status = opal_lib::opal_auto_optimize(cpu_usage);
        if status == 200 {
            if cpu_usage >= 95.0 {
                println!("[{}] CRITICAL LOAD: {:.2}% -> REALTIME", Local::now().format("%H:%M:%S"), cpu_usage);
            } else if cpu_usage >= 85.0 {
                println!("[{}] HIGH LOAD: {:.2}% -> HIGH_PRIORITY", Local::now().format("%H:%M:%S"), cpu_usage);
            }
        }
        
        thread::sleep(Duration::from_millis(500));
    }
}