use std::ffi::c_void;
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
    pub smoothing_factor: f32,
    pub cooldown_ms: u32,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            cpu_high_threshold: 85.0,
            cpu_realtime_threshold: 95.0,
            memory_high_threshold: 75.0,
            memory_realtime_threshold: 90.0,
            enable_thread_boost: true,
            sample_interval_ms: 250,
            smoothing_factor: 0.35,
            cooldown_ms: 1500,
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

pub struct OptimizerController {
    config: OptimizationConfig,
    last_priority: ProcessPriority,
    last_change_ms: u64,
    last_cpu: f32,
    last_memory: f32,
    last_timestamp_ms: u64,
}

impl OptimizerController {
    pub fn new(config: OptimizationConfig) -> Self {
        Self {
            config,
            last_priority: ProcessPriority::Normal,
            last_change_ms: 0,
            last_cpu: 0.0,
            last_memory: 0.0,
            last_timestamp_ms: 0,
        }
    }

    pub fn update(&mut self, cpu_load: f32, memory_load: f32, timestamp_ms: u64) -> OptimizationReport {
        let smoothed_cpu = if self.last_timestamp_ms == 0 {
            cpu_load
        } else {
            clamp(self.config.smoothing_factor * cpu_load + (1.0 - self.config.smoothing_factor) * self.last_cpu, 0.0, 100.0)
        };
        let smoothed_memory = if self.last_timestamp_ms == 0 {
            memory_load
        } else {
            clamp(self.config.smoothing_factor * memory_load + (1.0 - self.config.smoothing_factor) * self.last_memory, 0.0, 100.0)
        };
        let target = select_priority_for_load(smoothed_cpu, smoothed_memory, &self.config);
        let can_change = self.last_priority != target && (self.last_change_ms == 0 || timestamp_ms.saturating_sub(self.last_change_ms) >= self.config.cooldown_ms as u64);
        let applied = if can_change {
            let result = apply_priority(target, self.config.enable_thread_boost);
            if result {
                self.last_priority = target;
                self.last_change_ms = timestamp_ms;
            }
            result
        } else {
            false
        };

        self.last_cpu = smoothed_cpu;
        self.last_memory = smoothed_memory;
        self.last_timestamp_ms = timestamp_ms;

        OptimizationReport {
            selected_priority: target,
            applied,
            cpu_load: smoothed_cpu,
            memory_load: smoothed_memory,
            reason_code: match target {
                ProcessPriority::Realtime => 300,
                ProcessPriority::High => 200,
                ProcessPriority::Normal => 100,
            },
        }
    }
}

fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

pub fn select_priority_for_load(cpu_load: f32, memory_load: f32, config: &OptimizationConfig) -> ProcessPriority {
    if cpu_load >= config.cpu_realtime_threshold || memory_load >= config.memory_realtime_threshold {
        ProcessPriority::Realtime
    } else if cpu_load >= config.cpu_high_threshold || memory_load >= config.memory_high_threshold {
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
pub extern "C" fn opal_apply_optimization(cpu_load: f32, memory_load: f32, config_ptr: *const OptimizationConfig) -> OptimizationReport {
    let config = unsafe {
        if config_ptr.is_null() {
            &OptimizationConfig::default()
        } else {
            &*config_ptr
        }
    };
    apply_optimization(cpu_load, memory_load, config)
}

#[no_mangle]
pub extern "C" fn opal_create_optimizer(config_ptr: *const OptimizationConfig) -> *mut c_void {
    let config = if config_ptr.is_null() {
        OptimizationConfig::default()
    } else {
        unsafe { *config_ptr }
    };
    let controller = Box::new(OptimizerController::new(config));
    Box::into_raw(controller) as *mut c_void
}

#[no_mangle]
pub extern "C" fn opal_destroy_optimizer(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle as *mut OptimizerController));
        }
    }
}

#[no_mangle]
pub extern "C" fn opal_update_optimizer(handle: *mut c_void, cpu_load: f32, memory_load: f32, timestamp_ms: u64) -> OptimizationReport {
    if handle.is_null() {
        return OptimizationReport { selected_priority: ProcessPriority::Normal, applied: false, cpu_load, memory_load, reason_code: 0 };
    }

    let controller = unsafe { &mut *(handle as *mut OptimizerController) };
    controller.update(cpu_load, memory_load, timestamp_ms)
}
