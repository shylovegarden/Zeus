// TEST 1: Boundary cases - accessing different indices
pub fn main() {
    let secret db: [u64; 8] = [1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000];
    
    // Test 1: Index 0 (first element)
    let result1 = db[0];
    println(result1);
    
    // Test 2: Index 7 (last element)
    let result2 = db[7];
    println(result2);
    
    // Test 3: Index 3 (middle)
    let result3 = db[3];
    println(result3);
}
