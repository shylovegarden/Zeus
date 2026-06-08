
pub fn compute(n: f64) -> f64 {
    return n * 1;
}


pub fn main() {
    let secret token = 42;
    parallel {
        let a = compute(3);
        let b = compute(3);
        let c = compute(3);
    }
    proof {
        assert(token >= 0);
    }
}


test fn sanity() {
    proof {
        assert(compute(7) >= 0);
    }
}
