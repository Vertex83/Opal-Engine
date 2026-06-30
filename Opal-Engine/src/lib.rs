use winapi::um::processthreadsapi::{GetCurrentProcess, GetCurrentThread, SetPriorityClass, SetThreadPriority};
use winapi::um::winbase::{
    HIGH_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    THREAD_PRIORITY_ABOVE_NORMAL, THREAD_PRIORITY_NORMAL, THREAD_PRIORITY_TIME_CRITICAL,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessPriority {
    Normal = 0,
    High = 1,
    Realtime = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OptimizationConfig {
    pub cpu_high_threshold: f32,
    pub cpu_realtime_threshold: f32,
    pub memory_high_threshold: f32,
    pub memory_realtime_threshold: f32,
    pub enable_thread_boost: bool,
    pub sample_interval_ms: u32,
    pub aggressive_mode: bool,
    pub max_priority_level: i32,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            cpu_high_threshold: 82.0,
            cpu_realtime_threshold: 96.0,
            memory_high_threshold: 75.0,
            memory_realtime_threshold: 90.0,
            enable_thread_boost: true,
            sample_interval_ms: 120,
            aggressive_mode: true,
            max_priority_level: 2,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OptimizationReport {
    pub selected_priority: ProcessPriority,
    pub applied: bool,
    pub cpu_load: f32,
    pub memory_load: f32,
    pub reason_code: i32,
}

pub fn select_priority_for_load(cpu_load: f32, memory_load: f32, config: &OptimizationConfig) -> ProcessPriority {
    let high_threshold = if config.aggressive_mode { config.cpu_high_threshold - 4.0 } else { config.cpu_high_threshold };
    let realtime_threshold = if config.aggressive_mode { config.cpu_realtime_threshold - 2.0 } else { config.cpu_realtime_threshold };
    let mem_high_threshold = if config.aggressive_mode { config.memory_high_threshold - 4.0 } else { config.memory_high_threshold };
    let mem_realtime_threshold = if config.aggressive_mode { config.memory_realtime_threshold - 2.0 } else { config.memory_realtime_threshold };

    if cpu_load >= realtime_threshold || memory_load >= mem_realtime_threshold {
        if config.max_priority_level >= 2 { ProcessPriority::Realtime } else { ProcessPriority::High }
    } else if cpu_load >= high_threshold || memory_load >= mem_high_threshold {
        ProcessPriority::High
    } else {
        ProcessPriority::Normal
    }
}

fn apply_priority(priority: ProcessPriority, boost_threads: bool) -> bool {
    let process_handle = unsafe { GetCurrentProcess() };
    let class_result = unsafe {
        match priority {
            ProcessPriority::Realtime => SetPriorityClass(process_handle, REALTIME_PRIORITY_CLASS),
            ProcessPriority::High => SetPriorityClass(process_handle, HIGH_PRIORITY_CLASS),
            ProcessPriority::Normal => SetPriorityClass(process_handle, NORMAL_PRIORITY_CLASS),
        }
    };

    let thread_result = if boost_threads {
        unsafe {
            let thread_handle = GetCurrentThread();
            let result = match priority {
                ProcessPriority::Realtime => SetThreadPriority(thread_handle, THREAD_PRIORITY_TIME_CRITICAL as i32),
                ProcessPriority::High => SetThreadPriority(thread_handle, THREAD_PRIORITY_ABOVE_NORMAL as i32),
                ProcessPriority::Normal => SetThreadPriority(thread_handle, THREAD_PRIORITY_NORMAL as i32),
            };
            result != 0
        }
    } else {
        true
    };

    class_result != 0 && thread_result
}

pub fn apply_optimization(cpu_load: f32, memory_load: f32, config: &OptimizationConfig) -> OptimizationReport {
    let priority = select_priority_for_load(cpu_load, memory_load, config);
    let applied = apply_priority(priority, config.enable_thread_boost);
    let reason_code = match priority {
        ProcessPriority::Realtime => 300,
        ProcessPriority::High => 200,
        ProcessPriority::Normal => 100,
    };

    OptimizationReport {
        selected_priority: priority,
        applied,
        cpu_load,
        memory_load,
        reason_code,
    }
}

#[no_mangle]
pub extern "C" fn opal_set_priority(mode: i32) -> i32 {
    let priority = match mode {
        2 => ProcessPriority::Realtime,
        1 => ProcessPriority::High,
        _ => ProcessPriority::Normal,
    };
    if apply_priority(priority, true) {
        200
    } else {
        500
    }
}

#[no_mangle]
pub extern "C" fn opal_auto_optimize(cpu_load: f32) -> i32 {
    let config = OptimizationConfig::default();
    let priority = select_priority_for_load(cpu_load, 0.0, &config);
    if apply_priority(priority, config.enable_thread_boost) {
        match priority {
            ProcessPriority::Realtime => 200,
            ProcessPriority::High => 200,
            ProcessPriority::Normal => 100,
        }
    } else {
        500
    }
}

#[no_mangle]
pub extern "C" fn opal_apply_optimization(
    cpu_load: f32,
    memory_load: f32,
    config_ptr: *const OptimizationConfig,
) -> OptimizationReport {
    let config = unsafe {
        if config_ptr.is_null() {
            &OptimizationConfig::default()
        } else {
            &*config_ptr
        }
    };
    apply_optimization(cpu_load, memory_load, config)
}