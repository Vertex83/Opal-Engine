export fn opal_zig_optimize(cpu_load: f32, memory_load: f32, priority_out: *i32) i32 {
    if (cpu_load >= 95.0 or memory_load >= 90.0) {
        priority_out.* = 2;
        return 1;
    }
    if (cpu_load >= 85.0 or memory_load >= 75.0) {
        priority_out.* = 1;
        return 1;
    }
    priority_out.* = 0;
    return 0;
}
