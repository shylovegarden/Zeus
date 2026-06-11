// TEST 2: Mathematical proof verification
pub fn main() {
    let x: u64 = 100;
    let y: u64 = 200;
    
    // Prove arithmetic invariants BEFORE operations
    proof {
        assert(x > 0);
    }
    
    proof {
        assert(y > 0);
    }
    
    // After assignment, prove the result
    let sum: u64 = x + y;
    
    proof {
        assert(sum >= 0);
    }
    
    println(sum);
}
