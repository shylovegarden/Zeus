@verify(x)
pub fn test_verify_var_final(x: f64) {
    let y = x * 2.0
}

pub fn main() {
    test_verify_var_final(50.0)
}