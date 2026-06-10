// Top-level proof block: each assert is discharged by the SMT solver and
// emits a deterministic "[ZEUS VERIFIED] Mathematically proven: ..." line.
proof {
    assert(5 != 3);
    assert(2 * 3 + 4 > 9);
}
pub fn main() { let x = 1; }
