use opal_lib::{OptimizationConfig, ProcessPriority, select_priority_for_load};

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
    assert_eq!(priority, ProcessPriority::Realtime);
}

#[test]
fn keeps_normal_priority_under_thresholds() {
    let cfg = OptimizationConfig::default();
    let priority = select_priority_for_load(40.0, 30.0, &cfg);
    assert_eq!(priority, ProcessPriority::Normal);
}
