import zeus.io;

pub fn main() {
    println(0.0); // Debug: Started Phoenix Test

    // Launch M:N Parallel Fiber Dispatcher
    // We will launch 4 tasks. One of them will intentionally DDoS itself.
    parallel (i in 0..4) {
        if (i == 2.0) {
            // Malicious payload simulating a lock-up (e.g., 50M cycles)
            for d in 0.0..100000000.0 {
                let dummy = 1.0;
            }
        } else {
            // Normal workload
            let dummy = 2.0;
        }
    }

    // If the Sentinel correctly assassinates fiber 2, the loop will exit and we will reach here.
    println(1.0); // Debug: Survived the DDoS
}
