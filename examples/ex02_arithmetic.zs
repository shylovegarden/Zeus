// 02 -- Arithmetic: integer types and operator precedence.
// Explicit integer types (i32) are required for integer behavior;
// bare number literals are floats. Multiplication binds tighter than
// addition, and parentheses override the default precedence.
pub fn main() {
    let a: i32 = 2 * 3 + 4;     // 10  (2*3 first)
    let b: i32 = 2 + 3 * 4;     // 14  (3*4 first)
    let c: i32 = (2 + 3) * 4;   // 20  (parens first)
    let d: i32 = 10 - 3 - 4;    // 3   (left-associative)
    println(a + b + c + d);     // 47
}
