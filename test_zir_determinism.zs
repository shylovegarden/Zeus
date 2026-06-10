extern fn rand() -> i32;

fn pure_add(a: i32, b: i32) -> i32 {
    return a + b;
}
fn impure(a: i32) -> i32 {
    let r: i32 = rand();
    return a + r;
}
pub fn main() {
    println(pure_add(2, 3));
}
