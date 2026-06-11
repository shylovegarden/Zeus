fn divide(a: f64, b: f64) -> Result<f64, f64> {
    if b == 0.0 {
        return Err(b);
    }
    return Ok(a);
}

pub fn main() {
    let r = divide(10.0, 2.0);
    let bad = divide(5.0, 0.0);
    let v = unwrap(r);
    let d = is_err(bad);
    println(v);
}
