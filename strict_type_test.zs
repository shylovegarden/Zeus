// strict_type_test.zs
// Expected: FAILS under `zeus build --strict-types`
// Expected: PASSES under `zeus build` (default permissive mode)

fn check_width_safety() -> u64 {
    let x: i32 = 100
    // Assigning a known-i32 variable to a u64 slot:
    // -- permissive mode: OK (both are Num)
    // -- strict mode:     ERROR: cannot implicitly assign I32 to U64 slot
    let y: u64 = x
    return y
}

fn main() {
    let result: u64 = check_width_safety()
}
