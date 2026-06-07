
extern fn ext_i2c_read(addr: f64) -> f64;


extern fn ext_mmio_write(addr: f64, val: f64);


extern fn ext_i2c_write(addr: f64, val: f64);


extern fn ext_mmio_read(addr: f64) -> f64;


pub fn i2c_read(addr: f64) -> f64 {
    return ext_i2c_read(addr);
}


pub fn i2c_write(addr: f64, val: f64) {
    ext_i2c_write(addr, val);
}


pub fn mmio_read(addr: f64) -> f64 {
    return ext_mmio_read(addr);
}


pub fn mmio_write(addr: f64, val: f64) {
    ext_mmio_write(addr, val);
}


pub fn wasm_js_call(addr: f64) -> f64 {
    return 0;
}


pub fn riscv_mmio_read(addr: f64) -> f64 {
    return 0;
}


pub fn gpu_launch_kernel(grid_x: f64, block_x: f64) {
}


pub fn can_bus_send(id: f64, data: f64) {
}
