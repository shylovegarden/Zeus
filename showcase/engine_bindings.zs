// Zeus FFI bindings auto-generated from /tmp/engine.h
// Pragmatic import: review before use. Opaque pointers map to u64; char* -> str.

extern fn engine_init(config_path: str) -> i32;
extern fn engine_step(dt: f64, substeps: i32) -> f64;
extern fn engine_apply_force(fx: f32, fy: f32, fz: f32);
extern fn engine_entity_count() -> u64;
extern fn engine_version() -> str;
extern fn engine_shutdown();
