// An AI agent submitted this module with under-budget resource contracts.
// Zeus rejects it; the agent reads the JSON diagnostics, repairs the source,
// and converges on a signed certificate -- with no human review.
@wcet(5)
@stack(8)
fn accumulate(n: i32) -> i32 {
    let mut s: i32 = 0;
    let base: i32 = n;
    for i in 0..256 { s = s + base; }
    return s;
}
pub fn main() { println(accumulate(7)); }
