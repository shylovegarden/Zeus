// User function with typed param and return value.
pub fn square(n: f64) -> f64 {
    return n * n;
}
pub fn main() {
    let r = square(7.0);
    let s = square(r);
}
