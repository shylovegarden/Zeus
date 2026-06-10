// Zeus SoA (Structure-of-Arrays) test
// Demonstrates: aligned field arrays, FatPtr FFI bridge, vectorizer hints
// The struct gets decomposed into Particle_x[], Particle_y[], etc.
// each __attribute__((aligned(32))) so AVX2 can load 4 doubles per instruction.

struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

pub fn main() {
    // SoA instantiation -- no AoS copy overhead, cache-line perfect
    let particles = Particle[32];

    // Scalar walk to seed values (demonstrates field array access pattern)
    let i = 0;
    particles[i].x = particles[i].vx;
    particles[i].y = particles[i].vy;
}
