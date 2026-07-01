use std::sync::atomic::{AtomicU32, Ordering};

#[cfg(target_os = "windows")]
use winapi::um::errhandlingapi::GetLastError;
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::{GetCurrentProcess, GetCurrentThread, SetPriorityClass, SetThreadPriority};
#[cfg(target_os = "windows")]
use winapi::um::winbase::{
    HIGH_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    THREAD_PRIORITY_ABOVE_NORMAL, THREAD_PRIORITY_NORMAL, THREAD_PRIORITY_TIME_CRITICAL,
};

static LAST_ERROR_CODE: AtomicU32 = AtomicU32::new(0);

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
            aggressive_mode: false,
            max_priority_level: 1,
        }
    }
}

impl OptimizationConfig {
    pub fn aggressive_safe() -> Self {
        Self {
            cpu_high_threshold: 75.0,
            cpu_realtime_threshold: 90.0,
            memory_high_threshold: 70.0,
            memory_realtime_threshold: 85.0,
            enable_thread_boost: true,
            sample_interval_ms: 100,
            aggressive_mode: true,
            max_priority_level: 1,
        }
    }
    
    pub fn ultra_performance() -> Self {
        Self {
            cpu_high_threshold: 70.0,
            cpu_realtime_threshold: 85.0,
            memory_high_threshold: 65.0,
            memory_realtime_threshold: 80.0,
            enable_thread_boost: true,
            sample_interval_ms: 80,
            aggressive_mode: true,
            max_priority_level: 2,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptimizationReport {
    pub selected_priority: ProcessPriority,
    pub applied: bool,
    pub cpu_load: f32,
    pub memory_load: f32,
    pub reason_code: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AdvancedOptimizationReport {
    pub base_report: OptimizationReport,
    pub thread_pool_workers: u32,
    pub memory_pool_pressure: f32,
    pub lod_culling_enabled: bool,
    pub estimated_fps_gain: f32,
    pub safety_score: f32,
}

fn normalize_config(config: &OptimizationConfig) -> OptimizationConfig {
    let mut normalized = *config;
    normalized.cpu_high_threshold = normalized.cpu_high_threshold.clamp(0.0, 100.0);
    normalized.cpu_realtime_threshold = normalized.cpu_realtime_threshold.clamp(0.0, 100.0);
    normalized.memory_high_threshold = normalized.memory_high_threshold.clamp(0.0, 100.0);
    normalized.memory_realtime_threshold = normalized.memory_realtime_threshold.clamp(0.0, 100.0);
    normalized.sample_interval_ms = normalized.sample_interval_ms.max(1);
    normalized.max_priority_level = normalized.max_priority_level.clamp(0, 2);
    if normalized.cpu_realtime_threshold < normalized.cpu_high_threshold {
        normalized.cpu_realtime_threshold = normalized.cpu_high_threshold;
    }
    if normalized.memory_realtime_threshold < normalized.memory_high_threshold {
        normalized.memory_realtime_threshold = normalized.memory_high_threshold;
    }
    normalized
}

pub fn select_priority_for_load(cpu_load: f32, memory_load: f32, config: &OptimizationConfig) -> ProcessPriority {
    let config = normalize_config(config);
    let high_threshold = if config.aggressive_mode { config.cpu_high_threshold - 4.0 } else { config.cpu_high_threshold };
    let realtime_threshold = if config.aggressive_mode { config.cpu_realtime_threshold - 2.0 } else { config.cpu_realtime_threshold };
    let mem_high_threshold = if config.aggressive_mode { config.memory_high_threshold - 4.0 } else { config.memory_high_threshold };
    let mem_realtime_threshold = if config.aggressive_mode { config.memory_realtime_threshold - 2.0 } else { config.memory_realtime_threshold };

    if cpu_load >= realtime_threshold || memory_load >= mem_realtime_threshold {
        match config.max_priority_level {
            2 => ProcessPriority::Realtime,
            1 => ProcessPriority::High,
            _ => ProcessPriority::Normal,
        }
    } else if cpu_load >= high_threshold || memory_load >= mem_high_threshold {
        match config.max_priority_level {
            1 | 2 => ProcessPriority::High,
            _ => ProcessPriority::Normal,
        }
    } else {
        ProcessPriority::Normal
    }
}

fn apply_priority_with_error(priority: ProcessPriority, boost_threads: bool) -> (bool, Option<u32>) {
    #[cfg(target_os = "windows")]
    {
        let process_handle = unsafe { GetCurrentProcess() };
        let class_result = unsafe {
            match priority {
                ProcessPriority::Realtime => SetPriorityClass(process_handle, REALTIME_PRIORITY_CLASS),
                ProcessPriority::High => SetPriorityClass(process_handle, HIGH_PRIORITY_CLASS),
                ProcessPriority::Normal => SetPriorityClass(process_handle, NORMAL_PRIORITY_CLASS),
            }
        };

        if class_result == 0 {
            let error = unsafe { GetLastError() };
            LAST_ERROR_CODE.store(error, Ordering::SeqCst);
            return (false, Some(error));
        }

        if boost_threads {
            unsafe {
                let thread_handle = GetCurrentThread();
                let result = match priority {
                    ProcessPriority::Realtime => SetThreadPriority(thread_handle, THREAD_PRIORITY_TIME_CRITICAL as i32),
                    ProcessPriority::High => SetThreadPriority(thread_handle, THREAD_PRIORITY_ABOVE_NORMAL as i32),
                    ProcessPriority::Normal => SetThreadPriority(thread_handle, THREAD_PRIORITY_NORMAL as i32),
                };
                if result == 0 {
                    let error = GetLastError();
                    LAST_ERROR_CODE.store(error, Ordering::SeqCst);
                    return (false, Some(error));
                }
            }
        }

        LAST_ERROR_CODE.store(0, Ordering::SeqCst);
        (true, None)
    }

    #[cfg(not(target_os = "windows"))]
    {
        LAST_ERROR_CODE.store(0, Ordering::SeqCst);
        (true, None)
    }
}

fn apply_priority(priority: ProcessPriority, boost_threads: bool) -> bool {
    apply_priority_with_error(priority, boost_threads).0
}

pub fn apply_optimization(cpu_load: f32, memory_load: f32, config: &OptimizationConfig) -> OptimizationReport {
    let config = normalize_config(config);
    let priority = select_priority_for_load(cpu_load, memory_load, &config);
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
        reason_code: if applied { reason_code } else { -reason_code },
    }
}

#[no_mangle]
pub extern "C" fn opal_set_priority(mode: i32) -> i32 {
    let priority = match mode {
        2 => ProcessPriority::Realtime,
        1 => ProcessPriority::High,
        _ => ProcessPriority::Normal,
    };
    let (applied, error_code) = apply_priority_with_error(priority, true);
    if applied {
        200
    } else {
        match error_code {
            Some(5) => 403,
            Some(87) => 400,
            Some(_) => 500,
            None => 500,
        }
    }
}

#[no_mangle]
pub extern "C" fn opal_auto_optimize(cpu_load: f32) -> i32 {
    let config = OptimizationConfig::default();
    let priority = select_priority_for_load(cpu_load, 0.0, &config);
    let (applied, error_code) = apply_priority_with_error(priority, config.enable_thread_boost);
    if applied {
        match priority {
            ProcessPriority::Realtime => 200,
            ProcessPriority::High => 200,
            ProcessPriority::Normal => 100,
        }
    } else {
        match error_code {
            Some(5) => 403,
            Some(87) => 400,
            Some(_) => 500,
            None => 500,
        }
    }
}

#[no_mangle]
pub extern "C" fn opal_last_error() -> u32 {
    LAST_ERROR_CODE.load(Ordering::SeqCst)
}

#[no_mangle]
pub extern "C" fn opal_apply_optimization(
    cpu_load: f32,
    memory_load: f32,
    config_ptr: *const OptimizationConfig,
) -> OptimizationReport {
    let local_config = if config_ptr.is_null() {
        OptimizationConfig::default()
    } else {
        unsafe { std::ptr::read(config_ptr) }
    };
    apply_optimization(cpu_load, memory_load, &local_config)
}

#[no_mangle]
pub extern "C" fn opal_apply_optimization_ex(
    cpu_load: f32,
    memory_load: f32,
    config_ptr: *const OptimizationConfig,
    out: *mut OptimizationReport,
) -> i32 {
    if out.is_null() {
        return -1;
    }

    let local_config = if config_ptr.is_null() {
        OptimizationConfig::default()
    } else {
        unsafe { std::ptr::read(config_ptr) }
    };

    let report = apply_optimization(cpu_load, memory_load, &local_config);
    unsafe {
        std::ptr::write(out, report);
    }
    0
}

#[no_mangle]
pub extern "C" fn opal_aggressive_optimize(
    cpu_load: f32,
    memory_load: f32,
    enable_realtime: bool,
) -> i32 {
    let config = if enable_realtime {
        OptimizationConfig::ultra_performance()
    } else {
        OptimizationConfig::aggressive_safe()
    };
    
    let priority = select_priority_for_load(cpu_load, memory_load, &config);
    let (applied, error_code) = apply_priority_with_error(priority, config.enable_thread_boost);
    
    if applied {
        match priority {
            ProcessPriority::Realtime => 300,
            ProcessPriority::High => 200,
            ProcessPriority::Normal => 100,
        }
    } else {
        match error_code {
            Some(5) => 403,
            Some(87) => 400,
            Some(_) => 500,
            None => 500,
        }
    }
}

pub fn calculate_advanced_optimization(
    cpu_load: f32,
    memory_load: f32,
    num_workers: u32,
    memory_pressure: f32,
    config: &OptimizationConfig,
) -> AdvancedOptimizationReport {
    let base_report = apply_optimization(cpu_load, memory_load, config);
    
    let lod_culling_enabled = cpu_load > 70.0;
    
    let estimated_fps_gain = if config.aggressive_mode {
        match base_report.selected_priority {
            ProcessPriority::Realtime => 30.0,
            ProcessPriority::High => 20.0,
            ProcessPriority::Normal => 5.0,
        }
    } else {
        match base_report.selected_priority {
            ProcessPriority::Realtime => 15.0,
            ProcessPriority::High => 8.0,
            ProcessPriority::Normal => 2.0,
        }
    };
    
    let safety_score = {
        let priority_score = match base_report.selected_priority {
            ProcessPriority::Normal => 1.0,
            ProcessPriority::High => 0.8,
            ProcessPriority::Realtime => 0.6,
        };
        
        let load_score = 1.0 - ((cpu_load + memory_load) / 200.0).min(1.0);
        let aggressive_penalty = if config.aggressive_mode { 0.85 } else { 1.0 };
        
        ((priority_score * 0.4 + load_score * 0.6) * aggressive_penalty).max(0.1)
    };
    
    AdvancedOptimizationReport {
        base_report,
        thread_pool_workers: num_workers,
        memory_pool_pressure: memory_pressure.clamp(0.0, 1.0),
        lod_culling_enabled,
        estimated_fps_gain,
        safety_score,
    }
}

#[no_mangle]
pub extern "C" fn opal_advanced_optimize(
    cpu_load: f32,
    memory_load: f32,
    num_workers: u32,
    memory_pressure: f32,
    config_ptr: *const OptimizationConfig,
    out: *mut AdvancedOptimizationReport,
) -> i32 {
    if out.is_null() {
        return -1;
    }
    
    let config = if config_ptr.is_null() {
        OptimizationConfig::aggressive_safe()
    } else {
        unsafe { std::ptr::read(config_ptr) }
    };
    
    let report = calculate_advanced_optimization(cpu_load, memory_load, num_workers, memory_pressure, &config);
    unsafe {
        std::ptr::write(out, report);
    }
    0
}
