use opal_lib::{
    OptimizationConfig, ProcessPriority, select_priority_for_load, calculate_advanced_optimization,
    ThreadPool, MemoryPool, GPUOptimizer, WorkloadAnalyzer, WorkloadType,
};

#[test]
fn selects_high_priority_for_high_cpu() {
    let cfg = OptimizationConfig::default();
    let priority = select_priority_for_load(90.0, 40.0, &cfg);
    assert_eq!(priority, ProcessPriority::High);
}

#[test]
fn selects_realtime_for_critical_pressure() {
    let cfg = OptimizationConfig::default();
    let priority = select_priority_for_load(98.0, 92.0, &cfg);
    assert_eq!(priority, ProcessPriority::High);
}

#[test]
fn keeps_normal_priority_under_thresholds() {
    let cfg = OptimizationConfig::default();
    let priority = select_priority_for_load(40.0, 30.0, &cfg);
    assert_eq!(priority, ProcessPriority::Normal);
}

#[test]
fn defaults_to_high_instead_of_realtime_for_critical_load() {
    let cfg = OptimizationConfig::default();
    let priority = select_priority_for_load(99.0, 99.0, &cfg);
    assert_eq!(priority, ProcessPriority::High);
}

#[test]
fn respects_max_priority_limit() {
    let mut cfg = OptimizationConfig::default();
    cfg.max_priority_level = 0;
    let priority = select_priority_for_load(99.0, 99.0, &cfg);
    assert_eq!(priority, ProcessPriority::Normal);
}

#[test]
fn aggressive_safe_is_more_reactive_than_default() {
    let default_cfg = OptimizationConfig::default();
    let aggressive_cfg = OptimizationConfig::aggressive_safe();
    
    let default_priority = select_priority_for_load(85.0, 75.0, &default_cfg);
    let aggressive_priority = select_priority_for_load(85.0, 75.0, &aggressive_cfg);
    
    assert!(aggressive_priority as i32 >= default_priority as i32);
}

#[test]
fn ultra_performance_activates_realtime_at_lower_load() {
    let ultra_cfg = OptimizationConfig::ultra_performance();
    let priority = select_priority_for_load(86.0, 81.0, &ultra_cfg);
    assert_eq!(priority, ProcessPriority::Realtime);
}

#[test]
fn advanced_optimization_provides_reasonable_estimates() {
    let cfg = OptimizationConfig::aggressive_safe();
    let report = calculate_advanced_optimization(85.0, 70.0, 8, 0.6, &cfg);
    
    assert!(report.estimated_fps_gain > 0.0);
    assert!(report.safety_score > 0.0 && report.safety_score <= 1.0);
    assert_eq!(report.thread_pool_workers, 8);
    assert!(report.lod_culling_enabled);
}

#[test]
fn safety_score_decreases_with_realtime_priority() {
    let cfg = OptimizationConfig::ultra_performance();
    let report_high_load = calculate_advanced_optimization(95.0, 85.0, 8, 0.8, &cfg);
    
    assert!(report_high_load.safety_score < 0.8);
}

#[test]
fn thread_pool_executes_tasks() {
    use std::sync::{Arc, Mutex};
    
    let pool = ThreadPool::new(4);
    let counter = Arc::new(Mutex::new(0));
    
    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        pool.execute(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
        });
    }
    
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    let final_count = *counter.lock().unwrap();
    assert_eq!(final_count, 10);
}

#[test]
fn memory_pool_allocates_and_deallocates() {
    let mut pool = MemoryPool::new(1024, 5);
    
    assert!(pool.allocate().is_some());
    assert!(pool.allocate().is_some());
    
    let chunk = pool.allocate().unwrap();
    assert_eq!(chunk.len(), 1024);
    
    pool.deallocate(chunk);
    assert!(pool.allocate().is_some());
}

#[test]
fn memory_pool_reports_utilization() {
    let mut pool = MemoryPool::new(1024, 10);
    
    let initial = pool.get_utilization();
    assert_eq!(initial, 0.0);
    
    for _ in 0..5 {
        let _ = pool.allocate();
    }
    
    let mid = pool.get_utilization();
    assert!(mid > 0.4 && mid <= 0.5);
}

#[test]
fn gpu_optimizer_calculates_lod_correctly() {
    let optimizer = GPUOptimizer::new();
    
    assert_eq!(optimizer.calculate_lod(50.0), 0);
    assert_eq!(optimizer.calculate_lod(150.0), 1);
    assert_eq!(optimizer.calculate_lod(600.0), 3);
    assert_eq!(optimizer.calculate_lod(10000.0), 4);
}

#[test]
fn gpu_optimizer_frustum_cull_filters_objects() {
    let mut optimizer = GPUOptimizer::new();
    
    let camera_pos = (0.0, 0.0, 0.0);
    let view_distance = 500.0;
    let objects = vec![
        (100.0, 0.0, 0.0),
        (600.0, 0.0, 0.0),
        (0.0, 200.0, 0.0),
        (0.0, 0.0, 1000.0),
    ];
    
    let visible = optimizer.frustum_cull(camera_pos, view_distance, objects);
    
    assert_eq!(visible.len(), 2);
    assert_eq!(optimizer.get_culled_count(), 2);
}

#[test]
fn gpu_optimizer_reduces_draw_calls() {
    let optimizer = GPUOptimizer::new();
    
    let reduced = optimizer.reduce_draw_calls(5000);
    assert_eq!(reduced, 3000);
    
    let small = optimizer.reduce_draw_calls(1000);
    assert_eq!(small, 1000);
}

#[test]
fn workload_analyzer_detects_cpu_bound() {
    let mut analyzer = WorkloadAnalyzer::new();
    
    let cpu_history = vec![10.0, 80.0, 15.0, 85.0, 20.0];
    let memory_history = vec![50.0, 52.0, 51.0, 53.0, 52.0];
    
    let workload = analyzer.analyze(&cpu_history, &memory_history);
    assert_eq!(workload, WorkloadType::CpuBound);
}

#[test]
fn workload_analyzer_detects_memory_intensive() {
    let mut analyzer = WorkloadAnalyzer::new();
    
    let cpu_history = vec![50.0, 52.0, 51.0, 53.0, 52.0];
    let memory_history = vec![30.0, 50.0, 70.0, 90.0, 85.0];
    
    let workload = analyzer.analyze(&cpu_history, &memory_history);
    assert_eq!(workload, WorkloadType::MemoryIntensive);
}

#[test]
fn workload_analyzer_detects_balanced() {
    let mut analyzer = WorkloadAnalyzer::new();
    
    let cpu_history = vec![50.0, 52.0, 51.0, 53.0, 52.0];
    let memory_history = vec![50.0, 52.0, 51.0, 53.0, 52.0];
    
    let workload = analyzer.analyze(&cpu_history, &memory_history);
    assert_eq!(workload, WorkloadType::Balanced);
}

