// TEST 3: Stress test - larger array + multiple accesses
pub fn main() {
    let secret crypto_keys: [u64; 16] = [
        100, 200, 300, 400, 500, 600, 700, 800,
        900, 1000, 1100, 1200, 1300, 1400, 1500, 1600
    ];
    
    let mut total: u64 = 0;
    
    // Access multiple indices (should all be constant-time)
    for i in 0..16 {
        total = total + crypto_keys[i];
    }
    
    println(total);
}
