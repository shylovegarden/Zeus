// 06 -- Constant time: @constant_time proves no secret-dependent timing.
// The trick is to AVOID branching on secret data. This function does
// pure arithmetic (no `if` on the secret), so it is constant-time and
// the proof PASSES.
@constant_time
fn mix(secret_in: i32) -> i32 {
    let scaled: i32 = secret_in * 3;
    return scaled + 7;
}

pub fn main() {
    let secret s: i32 = 99;
    println(mix(s));  // 99*3 + 7 = 304
}
