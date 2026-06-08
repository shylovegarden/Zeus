import zeus.crypto;
import zeus.io;

pub fn main() {
    println(1.0); // Debug: Started Crypto Test

    // Run SHA-256 Initialization
    let state = sha256_init();
    
    // Verify first constant
    if (state.h0 == 1779033703.0) {
        println(1.0); // Success
    } else {
        println(0.0); // Failure
    }

    // Run ChaCha20 simulated Quarter Round
    let secure_val = chacha20_quarter_round(10.0, 20.0, 30.0, 40.0);
    
    if (secure_val == 30.0) {
        println(1.0); // Success
    } else {
        println(0.0); // Failure
    }
}
