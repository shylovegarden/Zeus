extern fn c_transform(x: f64, y: f64) -> f64;


pub fn run_ffi_test() {
    let px = 1.5;
    let py = 2.5;
    let result = c_transform(px, py);
}
