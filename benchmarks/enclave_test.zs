pub fn main() {
    let mut x = 10;
    
    enclave {
        // This executes within the Hardware Enclave boundaries.
        // It should emit barriers preventing the C compiler from reordering this.
        let sec_val = 42;
        x = x + sec_val;
    }
    
    println("Enclave processing done.");
}
