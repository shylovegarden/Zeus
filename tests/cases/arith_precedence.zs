// Arithmetic with correct operator precedence.
// 2*3+4 == 10, 2+3*4 == 14, (10-3)-4 == 3 (left-assoc).
pub fn main() {
    let a = 2 * 3 + 4;
    let b = 2 + 3 * 4;
    let c = 10 - 3 - 4;
    let d = a + b + c;
}
