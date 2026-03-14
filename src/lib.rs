use winapi::um::processthreadsapi::{GetCurrentProcess, SetPriorityClass};
use winapi::um::winbase::{HIGH_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS};

#[no_mangle]
pub extern "C" fn opal_set_priority(mode: i32) -> i32 {
    let result = unsafe {
        match mode {
            2 => SetPriorityClass(GetCurrentProcess(), REALTIME_PRIORITY_CLASS),
            1 => SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS),
            _ => SetPriorityClass(GetCurrentProcess(), NORMAL_PRIORITY_CLASS),
        }
    };
    if result != 0 { 200 } else { 500 }
}

#[no_mangle]
pub extern "C" fn opal_auto_optimize(cpu_load: f32) -> i32 {
    if cpu_load >= 95.0 {
        opal_set_priority(2)
    } else if cpu_load >= 85.0 {
        opal_set_priority(1)
    } else {
        opal_set_priority(0)
    }
}