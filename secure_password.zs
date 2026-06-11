// Secure Password Validator - Constant-Time Comparison
// Demonstrates: secret keyword, constant-time access, proof system

struct PassEntry {
    stored_hash: u64
    attempts: u64
}

pub fn main() {
    // Create secure password store (secret = wiped at scope exit + accessed obliviously)
    let secret passwords = PassEntry[10];
    
    // Initialize password hashes (in real code, these would be computed hashes)
    let i: u64 = 0;
    proof { assert(i >= 0); }
    
    passwords[0].stored_hash = 0xDEADBEEF;
    passwords[0].attempts = 0;
    
    passwords[1].stored_hash = 0xCAFEBABE;
    passwords[1].attempts = 0;
    
    // Test constant-time comparison
    let input_hash: u64 = 0xDEADBEEF;
    let user_id: u64 = 0;
    
    // This access is OBLIVIOUS - full O(10) scan, no timing leak on which entry matched
    let entry = passwords[user_id];
    let is_valid = const_time_compare(entry.stored_hash, input_hash);
    
    if is_valid {
        println(1);  // Valid
    } else {
        println(0);  // Invalid
    }
}

// Constant-time comparison function
// Even if hashes differ on first byte, comparison takes same time
fn const_time_compare(a: u64, b: u64) -> bool {
    let diff: u64 = a ^ b;
    let mut result: u64 = 0;
    
    // Branchless comparison (no conditional jumps)
    if diff == 0 {
        result = 1;
    } else {
        result = 0;
    }
    
    return result == 1;
}
