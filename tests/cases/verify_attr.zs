// @verify attribute drives the formal-verification stage for the function.
@verify(1)
pub fn checked(x: f64) {
    let y = x * 2.0
}
pub fn main() {
    checked(50.0)
}
