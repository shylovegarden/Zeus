struct Box { v: i32 }
@constant_time
fn lookup(secret key: i32, a: i32, b: i32) -> i32 {
    let bx = Box { v: key };
    let k: i32 = bx.v;
    if k > 0 { return b; }
    return a;
}
pub fn main() { println(lookup(1, 10, 20)); }
