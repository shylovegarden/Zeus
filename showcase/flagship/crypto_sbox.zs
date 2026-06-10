// FLAGSHIP: Cryptography -- constant-time secret table, proven leak-free.
struct Cell { v: i32 }
@constant_time
fn sbox_mix(round: i32) -> i32 {
    let secret sbox = Cell[256];
    let a = sbox[round].v;
    return a + round;
}
pub fn main() { println(sbox_mix(7)); }
