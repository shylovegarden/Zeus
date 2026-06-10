@requires(b > 0)
@ensures(result > 0)
fn add_pos(a: i32, b: i32) -> i32 {
    return a + b;
}
pub fn main() {
    println(add_pos(3, 4));
}
