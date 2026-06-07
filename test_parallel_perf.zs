pub fn main() {
    // Each iteration does independent work (no shared mutable state)
    // On systems WITH OpenMP: this runs in parallel
    // On macOS WITHOUT OpenMP: this runs sequentially
    parallel (i in 0..1000) {
        // Simulate some work
        let x = i * i * i
        let y = x + 1
    }
}
