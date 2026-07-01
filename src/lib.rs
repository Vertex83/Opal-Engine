use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::collections::VecDeque;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkloadType {
    CpuBound,
    IoBound,
    MemoryIntensive,
    Balanced,
}

pub struct WorkloadAnalyzer {
    cpu_variance: f32,
    memory_trend: f32,
    io_pressure: f32,
}

impl WorkloadAnalyzer {
    pub fn new() -> Self {
        Self {
            cpu_variance: 0.0,
            memory_trend: 0.0,
            io_pressure: 0.0,
        }
    }
    
    pub fn analyze(&mut self, cpu_history: &[f32], memory_history: &[f32]) -> WorkloadType {
        if cpu_history.len() > 1 {
            let mean_cpu = cpu_history.iter().sum::<f32>() / cpu_history.len() as f32;
            self.cpu_variance = cpu_history.iter()
                .map(|x| (x - mean_cpu).powi(2))
                .sum::<f32>() / cpu_history.len() as f32;
        }
        
        if memory_history.len() > 1 {
            self.memory_trend = (memory_history.last().unwrap_or(&0.0) 
                - memory_history.first().unwrap_or(&0.0)).abs();
        }
        
        match (self.cpu_variance > 15.0, self.memory_trend > 20.0) {
            (true, false) => WorkloadType::CpuBound,
            (false, true) => WorkloadType::MemoryIntensive,
            (true, true) => WorkloadType::IoBound,
            _ => WorkloadType::Balanced,
        }
    }
}

pub struct ThreadPool {
    sender: mpsc::Sender<Box<dyn FnOnce() + Send>>,
    _workers: Vec<thread::JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new(num_threads: usize) -> Self {
        let (sender, receiver) = mpsc::channel::<Box<dyn FnOnce() + Send>>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::new();
        
        for _ in 0..num_threads {
            let receiver = Arc::clone(&receiver);
            let worker = thread::spawn(move || {
                loop {
                    if let Ok(task) = receiver.lock().unwrap().recv() {
                        task();
                    }
                }
            });
            workers.push(worker);
        }
        
        Self {
            sender,
            _workers: workers,
        }
    }
    
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let _ = self.sender.send(Box::new(f));
    }
    
    pub fn parallel_for<F>(&self, start: usize, end: usize, chunk_size: usize, f: F)
    where
        F: Fn(usize) + Send + Copy + 'static,
    {
        for chunk_start in (start..end).step_by(chunk_size) {
            let chunk_end = (chunk_start + chunk_size).min(end);
            self.execute(move || {
                for i in chunk_start..chunk_end {
                    f(i);
                }
            });
        }
    }
}

pub struct MemoryPool {
    pool: VecDeque<Vec<u8>>,
    chunk_size: usize,
}

impl MemoryPool {
    pub fn new(chunk_size: usize, pool_size: usize) -> Self {
        let mut pool = VecDeque::with_capacity(pool_size);
        for _ in 0..pool_size {
            pool.push_back(vec![0u8; chunk_size]);
        }
        
        Self { pool, chunk_size }
    }
    
    pub fn allocate(&mut self) -> Option<Vec<u8>> {
        self.pool.pop_front()
    }
    
    pub fn deallocate(&mut self, chunk: Vec<u8>) {
        if self.pool.len() < self.pool.capacity() {
            self.pool.push_back(chunk);
        }
    }
    
    pub fn defragment(&mut self) {
        while self.pool.len() < self.pool.capacity() / 2 {
            self.pool.push_back(vec![0u8; self.chunk_size]);
        }
    }
    
    pub fn get_utilization(&self) -> f32 {
        (1.0 - (self.pool.len() as f32 / self.pool.capacity() as f32)).clamp(0.0, 1.0)
    }
}

pub struct GPUOptimizer {
    lod_levels: Vec<f32>,
    max_draw_calls: u32,
    culled_objects: u32,
}

impl GPUOptimizer {
    pub fn new() -> Self {
        Self {
            lod_levels: vec![100.0, 200.0, 500.0, 1000.0, 5000.0],
            max_draw_calls: 3000,
            culled_objects: 0,
        }
    }
    
    pub fn calculate_lod(&self, distance: f32) -> u32 {
        for (lod, &threshold) in self.lod_levels.iter().enumerate() {
            if distance <= threshold {
                return lod as u32;
            }
        }
        (self.lod_levels.len() - 1) as u32
    }
    
    pub fn frustum_cull(&mut self, camera_pos: (f32, f32, f32), view_distance: f32, objects: Vec<(f32, f32, f32)>) -> Vec<(f32, f32, f32)> {
        let mut visible = Vec::new();
        self.culled_objects = 0;
        
        for obj_pos in objects {
            let dx = obj_pos.0 - camera_pos.0;
            let dy = obj_pos.1 - camera_pos.1;
            let dz = obj_pos.2 - camera_pos.2;
            let distance = (dx*dx + dy*dy + dz*dz).sqrt();
            
            if distance <= view_distance {
                visible.push(obj_pos);
            } else {
                self.culled_objects += 1;
            }
        }
        
        visible
    }
    
    pub fn reduce_draw_calls(&self, current_calls: u32) -> u32 {
        current_calls.min(self.max_draw_calls)
    }
    
    pub fn get_culled_count(&self) -> u32 {
        self.culled_objects
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
