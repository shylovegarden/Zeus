// FLAGSHIP: AI supply-chain -- AI-written task that PROVES it is bounded.
@wcet(3000)
fn task(n: i32) -> i32 {
    let mut s: i32 = 0;
    for i in 0..64 { s = s + i; }
    return s + n;
}
pub fn main() { println(task(1)); }
