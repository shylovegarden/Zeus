@constant_time
fn f(secret k: i32, m: i32) -> i32 { let mut x: i32 = m; x /= k; return x; }
pub fn main() { println(f(7, 3)); }
