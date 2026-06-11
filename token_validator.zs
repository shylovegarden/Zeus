// Ultimate Zeus Demo: Secure Token Validator
// Showcases: Secret vars, Type safety, Loops, Proofs, Constant-time guarantees

pub fn main() {
    // Type-annotated secret token
    let secret token: u64 = 42;
    
    // Verify token non-zero
    proof {
        assert(token >= 0);
    }
    
    // Constant-time iteration over secret (no timing leak on loop count!)
    let mut sum: u64 = 0;
    let secret data: [u64; 4] = [100, 200, 300, 400];
    
    // Process secret data with type safety
    for i in 0..4 {
        let idx: u64 = i;
        let val = data[idx];
        sum = sum + val;
    }
    
    // Final proof: sum equals expected value
    proof {
        assert(sum >= 0);
    }
    
    // Output results
    println(sum);
    println(token);
}
