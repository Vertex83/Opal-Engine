mod engine;
mod modules;
mod profiler;
mod thread_manager;
mod memory_optimizer;
mod gpu_optimizer;

use chrono::Local;
use opal_lib::{apply_optimization, OptimizationConfig};
use std::{
    thread, time::{Duration, Instant},
    sync::{Arc, Mutex, mpsc},
    collections::VecDeque,
};
use sysinfo::System;

// ==================== MODUŁ 1: PROFILER ====================
mod profiler {
    use std::time::{Instant, Duration};
    use std::collections::VecDeque;
    
    pub struct FrameProfiler {
        frame_start: Instant,
        frame_times: VecDeque<Duration>,
        max_history: usize,
        pub physics_time: Duration,
        pub render_time: Duration,
        pub gpu_time: Duration,
    }
    
    impl FrameProfiler {
        pub fn new() -> Self {
            Self {
                frame_start: Instant::now(),
                frame_times: VecDeque::with_capacity(120),
                max_history: 120,
                physics_time: Duration::ZERO,
                render_time: Duration::ZERO,
                gpu_time: Duration::ZERO,
            }
        }
        
        pub fn frame_start(&mut self) {
            self.frame_start = Instant::now();
        }
        
        pub fn frame_end(&mut self) -> (f32, Duration) {
            let frame_time = self.frame_start.elapsed();
            self.frame_times.push_back(frame_time);
            
            if self.frame_times.len() > self.max_history {
                self.frame_times.pop_front();
            }
            
            let avg_frame_time: Duration = self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32;
            let fps = 1.0 / avg_frame_time.as_secs_f32().max(0.001);
            
            (fps, frame_time)
        }
        
        pub fn get_bottleneck(&self) -> String {
            let frame_ms = self.frame_times.back().unwrap_or(&Duration::from_millis(16)).as_millis() as u32;
            let physics_pct = (self.physics_time.as_millis() as f32 / frame_ms.max(1) as f32) * 100.0;
            let render_pct = (self.render_time.as_millis() as f32 / frame_ms.max(1) as f32) * 100.0;
            let gpu_pct = (self.gpu_time.as_millis() as f32 / frame_ms.max(1) as f32) * 100.0;
            
            let max = physics_pct.max(render_pct).max(gpu_pct);
            
            match max as u32 {
                0..=40 => format!("✅ BALANCED ({}ms)", frame_ms),
                41..=60 => {
                    if physics_pct > render_pct { 
                        format!("⚠️ PHYSICS HEAVY ({}ms) - {}%", frame_ms, physics_pct as u32)
                    } else {
                        format!("⚠️ RENDER HEAVY ({}ms) - {}%", frame_ms, render_pct as u32)
                    }
                }
                _ => format!("🔴 CRITICAL ({}ms) - Physics:{}% Render:{}% GPU:{}%", frame_ms, physics_pct as u32, render_pct as u32, gpu_pct as u32),
            }
        }
        
        pub fn start_physics(&mut self) {
            self.physics_time = Instant::now().elapsed();
        }
        
        pub fn end_physics(&mut self) {
            self.physics_time = Instant::now().elapsed();
        }
        
        pub fn start_render(&mut self) {
            self.render_time = Instant::now().elapsed();
        }
        
        pub fn end_render(&mut self) {
            self.render_time = Instant::now().elapsed();
        }
    }
}

// ==================== MODUŁ 2: THREAD MANAGER ====================
mod thread_manager {
    use std::thread;
    use std::sync::{Arc, Mutex, mpsc};
    use std::collections::VecDeque;
    
    pub struct ThreadPool {
        sender: mpsc::Sender<Box<dyn Fn() + Send>>,
        _workers: Vec<thread::JoinHandle<()>>,
    }
    
    impl ThreadPool {
        pub fn new(num_threads: usize) -> Self {
            let (sender, receiver) = mpsc::channel();
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
            F: Fn() + Send + 'static,
        {
            let _ = self.sender.send(Box::new(f));
        }
        
        pub fn parallel_for<F>(&self, start: usize, end: usize, chunk_size: usize, mut f: F)
        where
            F: FnMut(usize) + Send + Copy + 'static,
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
}

// ==================== MODUŁ 3: MEMORY OPTIMIZER ====================
mod memory_optimizer {
    use std::collections::VecDeque;
    
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
    }
    
    pub struct GarbageCollector {
        collect_interval: u32,
        frame_count: u32,
    }
    
    impl GarbageCollector {
        pub fn new(interval: u32) -> Self {
            Self {
                collect_interval: interval,
                frame_count: 0,
            }
        }
        
        pub fn should_collect(&mut self) -> bool {
            self.frame_count += 1;
            if self.frame_count >= self.collect_interval {
                self.frame_count = 0;
                return true;
            }
            false
        }
    }
}

// ==================== MODUŁ 4: GPU OPTIMIZER ====================
mod gpu_optimizer {
    pub struct GPUOptimizer {
        lod_levels: Vec<f32>,
        max_draw_calls: u32,
        culled_objects: u32,
    }
    
    impl GPUOptimizer {
        pub fn new() -> Self {
            Self {
                lod_levels: vec![100.0, 200.0, 500.0, 1000.0, 5000.0],
                max_draw_calls: 3000,  // Zmniejsz z domyślnie 10000
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
        
        pub fn reduce_draw_calls(&self, draw_calls: u32) -> u32 {
            draw_calls.min(self.max_draw_calls)
        }
        
        pub fn get_stats(&self) -> String {
            format!("Culled: {} objects", self.culled_objects)
        }
    }
}

// ==================== MAIN ====================
fn main() {
    let mut sys = System::new_all();
    let config = OptimizationConfig::default();
    
    // INICJALIZACJA MODUŁÓW
    let mut profiler = profiler::FrameProfiler::new();
    let thread_pool = thread_manager::ThreadPool::new(num_cpus::get());
    let mut memory_pool = memory_optimizer::MemoryPool::new(1024 * 1024, 32);
    let mut gc = memory_optimizer::GarbageCollector::new(60);
    let mut gpu_optimizer = gpu_optimizer::GPUOptimizer::new();
    
    // KONFIGURACJA
    let frame_time_target = Duration::from_millis(16); // 60 FPS target (16.67ms per frame)
    let mut frame_count = 0u64;
    let start_time = Instant::now();
    
    println!("[{}] Opal Core: FULL OPTIMIZATION INITIALIZED", Local::now().format("%H:%M:%S"));
    println!("[{}] CPU Cores: {} | Frame Target: 60 FPS (16.67ms)", Local::now().format("%H:%M:%S"), num_cpus::get());
    
    loop {
        profiler.frame_start();
        let frame_iteration_start = Instant::now();
        
        // ========== SEKCJA 1: SYSTEM MONITORING ==========
        if frame_count % 30 == 0 {
            sys.refresh_cpu();
            sys.refresh_memory();
        }
        
        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let memory_total = sys.total_memory();
        let memory_used = sys.used_memory();
        let memory_usage = if memory_total > 0 {
            (memory_used as f64 / memory_total as f64 * 100.0) as f32
        } else {
            0.0
        };

        // ========== SEKCJA 2: PRIORITY OPTIMIZATION ==========
        let report = apply_optimization(cpu_usage, memory_usage, &config);
        
        // ========== SEKCJA 3: PARALLEL PHYSICS ==========
        let physics_start = Instant::now();
        let num_objects = 10000;
        thread_pool.parallel_for(0, num_objects, 1000, |_i| {
            // Symulacja fizyki na wiele rdzeni
            let _x = (_i as f32).sin() * 100.0;
            let _y = (_i as f32).cos() * 100.0;
        });
        profiler.physics_time = physics_start.elapsed();
        
        // ========== SEKCJA 4: GPU OPTIMIZATION (CULLING + LOD) ==========
        let render_start = Instant::now();
        let camera_pos = (0.0, 0.0, 0.0);
        let view_distance = 500.0;
        let all_objects = vec![(100.0, 0.0, 0.0); 5000];
        
        let visible_objects = gpu_optimizer.frustum_cull(camera_pos, view_distance, all_objects);
        let mut draw_calls = visible_objects.len() as u32 * 3;
        draw_calls = gpu_optimizer.reduce_draw_calls(draw_calls);
        
        // LOD calculation
        for obj in &visible_objects {
            let dx = obj.0 - camera_pos.0;
            let dy = obj.1 - camera_pos.1;
            let dz = obj.2 - camera_pos.2;
            let distance = (dx*dx + dy*dy + dz*dz).sqrt();
            let _lod = gpu_optimizer.calculate_lod(distance);
        }
        profiler.render_time = render_start.elapsed();
        
        // ========== SEKCJA 5: MEMORY MANAGEMENT ==========
        if gc.should_collect() {
            memory_pool.defragment();
        }
        
        if memory_usage > 85.0 {
            if let Some(_chunk) = memory_pool.allocate() {
                // Recycle chunk
            }
        }
        
        // ========== SEKCJA 6: FRAME TIMING CONTROL ==========
        let (fps, frame_ms) = profiler.frame_end();
        
        // Adaptive FPS limiting
        let elapsed = frame_iteration_start.elapsed();
        if elapsed < frame_time_target {
            let sleep_time = frame_time_target - elapsed;
            thread::sleep(sleep_time);
        }
        
        // ========== SEKCJA 7: DIAGNOSTYKA ==========
        frame_count += 1;
        
        if frame_count % 60 == 0 {
            println!(
                "[{}] FPS: {:.1} | Frame: {:.2}ms | CPU: {:.1}% | MEM: {:.1}% | Draw Calls: {} | {} | {}",
                Local::now().format("%H:%M:%S"),
                fps,
                frame_ms.as_secs_f32() * 1000.0,
                cpu_usage,
                memory_usage,
                draw_calls,
                profiler.get_bottleneck(),
                gpu_optimizer.get_stats()
            );
            
            if report.applied {
                match report.selected_priority {
                    opal_lib::ProcessPriority::Realtime => {
                        println!("[{}] PRIORITY: REALTIME (max performance)", Local::now().format("%H:%M:%S"));
                    }
                    opal_lib::ProcessPriority::High => {
                        println!("[{}] PRIORITY: HIGH (optimized)", Local::now().format("%H:%M:%S"));
                    }
                    opal_lib::ProcessPriority::Normal => {
                        println!("[{}]  PRIORITY: NORMAL (stable)", Local::now().format("%H:%M:%S"));
                    }
                }
            }
            
            if frame_ms.as_millis() > 16 {
                println!("[{}] FRAME SPIKE DETECTED! {}ms > 16ms target", 
                    Local::now().format("%H:%M:%S"),
                    frame_ms.as_millis()
                );
            }
        }
        
        // Safety check - prevent infinite loop crashes
        if frame_count % 3600 == 0 {
            let uptime = start_time.elapsed();
            println!("[{}]  Uptime: {}s | Total Frames: {} | Avg FPS: {:.1}", 
                Local::now().format("%H:%M:%S"),
                uptime.as_secs(),
                frame_count,
                frame_count as f32 / uptime.as_secs_f32()
            );
        }
    }
}
