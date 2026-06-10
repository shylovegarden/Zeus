@constant_time
@wcet(2000)
fn score(n: i32) -> i32 {
    let mut s: i32 = 0;
    for i in 0..50 { s = s + i; }
    return s + n;
}
pub fn main() { println(score(7)); }
