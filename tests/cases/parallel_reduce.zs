// Parallel loop accumulating a reduction into a mutable var.
pub fn main() {
    let mut sum = 0
    parallel (i in 0..10) {
        sum = sum + i
    }
}
