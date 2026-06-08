pub fn main() {
    let a = 10.0;
    let b = 20.0;
    let c = a + b;

    // This should pass the SMT solver (30.0 < 50.0)
    assert(c < 50.0);

    // This should FAIL the SMT solver (30.0 > 100.0 is provably impossible)
    assert(c > 100.0);
}
