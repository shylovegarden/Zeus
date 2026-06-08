extern fn legacy_c_process_frame() -> void;

@ffi_export
pub fn run_phoenix_test() -> void {
    // 100 iterations * 1 Megabyte leak = 100 Megabytes leaked.
    // The Zeus Static Arena is only 64 Megabytes. 
    // If the Phoenix Fiber fails to assassinate the memory, the system will panic with an OOM!
    let count = 100;
    
    // We simulate a parallel processing pipeline
    parallel (i in 0..count) {
        legacy_c_process_frame();
    }
}
