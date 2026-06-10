// Combines a secret value, a parallel block, and a proof assertion.
pub fn compute(n: f64) -> f64 {
    return n * 1;
}
pub fn main() {
    let secret token = 42;
    parallel {
        let a = compute(3);
        let b = compute(3);
    }
    proof {
        assert(token >= 0);
    }
}
