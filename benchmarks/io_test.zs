pub fn main() {
    let secret alice_key = 10;
    let bob_key = 5;

    // This operation involves a secret, so it will compile into an iO Garbled Circuit.
    let shared_secret = alice_key * bob_key;
    
    // Normal operation.
    let normal_op = bob_key + 2;

    println("iO Simulation complete.");
}
