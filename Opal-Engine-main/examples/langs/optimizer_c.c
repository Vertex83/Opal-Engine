#include "optimizer_ffi.h"

int opal_c_optimize(float cpu_load, float memory_load, int* priority_out) {
    if (cpu_load >= 95.0f || memory_load >= 90.0f) {
        *priority_out = 2;
        return 1;
    }
    if (cpu_load >= 85.0f || memory_load >= 75.0f) {
        *priority_out = 1;
        return 1;
    }
    *priority_out = 0;
    return 0;
}
