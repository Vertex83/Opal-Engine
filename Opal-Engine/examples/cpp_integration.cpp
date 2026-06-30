#include <cstdint>
#include <iostream>

extern "C" {
struct OptimizationConfig {
    float cpu_high_threshold;
    float cpu_realtime_threshold;
    float memory_high_threshold;
    float memory_realtime_threshold;
    bool enable_thread_boost;
    unsigned int sample_interval_ms;
    bool aggressive_mode;
    int max_priority_level;
};

struct OptimizationReport {
    int selected_priority;
    bool applied;
    float cpu_load;
    float memory_load;
    int reason_code;
};

void* opal_create_optimizer(const OptimizationConfig* config);
void opal_destroy_optimizer(void* handle);
OptimizationReport opal_update_optimizer(void* handle, float cpu_load, float memory_load, std::uint64_t timestamp_ms);
int opal_set_priority(int mode);
int opal_auto_optimize(float cpu_load);
OptimizationReport opal_apply_optimization(float cpu_load, float memory_load, const OptimizationConfig* config);
}

int main() {
    OptimizationConfig config{};
    config.cpu_high_threshold = 85.0f;
    config.cpu_realtime_threshold = 95.0f;
    config.memory_high_threshold = 75.0f;
    config.memory_realtime_threshold = 90.0f;
    config.enable_thread_boost = true;
    config.sample_interval_ms = 120;
    config.aggressive_mode = true;
    config.max_priority_level = 2;

    void* handle = opal_create_optimizer(&config);
    auto report = opal_update_optimizer(handle, 91.0f, 80.0f, 1000);
    std::cout << "priority=" << report.selected_priority
              << " applied=" << report.applied
              << " reason=" << report.reason_code << '\n';
    opal_destroy_optimizer(handle);
    return 0;
}
