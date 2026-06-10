// FEATURE TEST FIXTURE (audit v2 structured JSON: wcet_exceeded).
// A loop of 256 iterations carries a proven WCET far above the declared
// @wcet(5) budget. `zeus audit --json` must emit a structured finding with
// kind=="wcet_exceeded", a positive integer `gap`, and fixable==true.
// NOTE: the @wcet attribute MUST sit immediately above `fn` (no other
// attribute line in between) for the parser to attach it to the function.
@wcet(5)
fn sum_to(n: i32) -> i32 {
    let mut s: i32 = 0;
    let base: i32 = n;
    for i in 0..256 { s = s + base; }
    return s;
}
pub fn main() { println(sum_to(7)); }
