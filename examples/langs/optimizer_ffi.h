#ifndef OPAL_LANG_FFI_H
#define OPAL_LANG_FFI_H

#ifdef __cplusplus
extern "C" {
#endif

int opal_c_optimize(float cpu_load, float memory_load, int* priority_out);
int opal_zig_optimize(float cpu_load, float memory_load, int* priority_out);
int opal_d_optimize(float cpu_load, float memory_load, int* priority_out);
int opal_asm_optimize(float cpu_load, float memory_load, int* priority_out);

#ifdef __cplusplus
}
#endif

#endif
