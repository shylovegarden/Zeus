// 05 -- Secret values: the `secret` keyword tags data the compiler must
// protect (tracked through derived expressions, zeroized when dead).
// Here a secret key is combined into a public-looking result.
pub fn main() {
    let secret key: i32 = 1234;
    let masked: i32 = key + 1;
    println(masked);  // 1235
}
