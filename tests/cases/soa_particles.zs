// Structure-of-Arrays particle buffer with field access.
struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}
pub fn main() {
    let particles = Particle[32];
    let i = 0;
    particles[i].x = particles[i].vx;
    particles[i].y = particles[i].vy;
}
