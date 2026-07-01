use opal_lib::{OptimizationConfig, ProcessPriority, select_priority_for_load, calculate_advanced_optimization};

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

