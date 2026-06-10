fn sum_to(n: i32) -> i32 {
    let mut s: i32 = 0;
    for i in 0..100 { s = s + i; }
    return s;
}
fn fac(n: i32) -> i32 { if (n < 2) { return 1; } return n * fac(n - 1); }
pub fn main() { println(sum_to(100)); }
