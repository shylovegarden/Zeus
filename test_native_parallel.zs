pub fn main() {
    // Test parallel execution with native M:N scheduler
    let mut sum = 0
    parallel (i in 0..100) {
        sum = sum + i
    }
}
