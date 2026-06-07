@verify(throttle <= 100.0)
@adaptive(hot_path=true)
pub fn process_data(throttle: f64) {
    cluster {
        let x = 10.0
    }
}

pub fn main() {
    // Test parallel execution
    let mut sum = 0
    parallel (i in 0..10) {
        sum = sum + i
    }

    process_data(50.0)
}
