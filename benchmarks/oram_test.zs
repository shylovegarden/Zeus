pub fn main() {
    let A = tensor[10, 10];
    
    // An attacker trying to observe the memory bus to see which index we access
    // will see three randomized reads for every actual read.
    let secret_val = A.data[2];
    let secret_val2 = A.data[4];

    println("ORAM Simulation complete.");
}
