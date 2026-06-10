// FLAGSHIP: Safety-critical embedded -- proven WCET + bounded stack + zero-heap.
@wcet(5000)
@stack(4096)
fn control_step(setpoint: i32, measured: i32) -> i32 {
    let mut accum: i32 = 0;
    let err: i32 = setpoint - measured;
    for i in 0..16 { accum = accum + err; }
    return accum;
}
pub fn main() { println(control_step(100, 80)); }
