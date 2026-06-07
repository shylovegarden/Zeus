// Test parallel execution
pub fn main() {
    let mut sum = 0
    parallel (i in 0..10) {
        sum = sum + i
    }
}
