// Zeus SoA throughput benchmark

struct Particle {
    x: f64,
    vx: f64,
    y: f64,
    vy: f64,
}

pub fn main() {
    let particles = Particle[16384];

    let i = 0;
    particles[i].x  = 0.0;
    particles[i].vx = 1.0;
    particles[i].y  = 0.0;
    particles[i].vy = 0.5;

    // integration: x += vx
    particles[i].x = particles[i].x;

    @verify(i >= 0);
}
