// FEATURE TEST FIXTURE (audit v2 structured JSON: secret_branch).
// A @constant_time function branches on a `secret` parameter -- an
// unmitigated control-flow timing channel that cannot be auto-fixed.
// `zeus audit --json` must emit a structured finding with
// kind=="secret_branch" and fixable==false.
@constant_time
fn pick(secret k: i32, a: i32, b: i32) -> i32 {
    if k > 0 { return a; }
    return b;
}
pub fn main() { println(pick(1, 7, 9)); }
