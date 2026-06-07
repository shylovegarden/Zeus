// Test all major Zeus features

@verify(x <= 100)
pub fn process_data(x: f64) {
    let y = x * 2
}

pub fn main() {
    // Test 1: Basic variables
    let x = 42
    let secret password = 1234
    
    // Test 2: Parallel block
    let mut sum = 0
    parallel (i in 0..10) {
        sum = sum + i
    }
    
    // Test 3: Function call
    process_data(50)
}
