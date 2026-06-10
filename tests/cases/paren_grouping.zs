// Explicit grouping overrides default precedence.
pub fn main() {
    let a = (2 + 3) * 4;
    let b = 2 * (3 + 4);
    let c = ((1 + 2) * (3 + 4));
}
