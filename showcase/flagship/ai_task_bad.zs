// FLAGSHIP: AI supply-chain -- an UNBOUNDED task the gate must refuse.
fn task(n: i32) -> i32 {
    let mut i: i32 = 0;
    let mut s: i32 = 0;
    while (i < n) { s = s + i; i = i + 1; }
    return s;
}
pub fn main() { println(task(10)); }
