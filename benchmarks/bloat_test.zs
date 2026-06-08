// Anti-Bloat Enforcer Benchmark
// This file intentionally attempts to use a legacy paradigm to trigger the compiler's guillotine.

extern fn malloc(size: i32) -> i32;

pub fn main() {
    // We try to bypass the Zero-Heap Enforcer.
    // The Anti-Bloat Enforcer MUST catch this before the binary is even built.
    let ptr = malloc(1024);
}
