#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#define NUM_PARTICLES 50000000

// Legacy Array-of-Structures (AoS) Layout
// This naive layout forces the CPU to pull 'mass' and 'density' into the L1 cache
// even though we only need to update position using velocity.
typedef struct {
    float x, y;
    float vx, vy;
    float mass;
    float density;
} Particle;

int main() {
    Particle* particles = (Particle*)malloc(NUM_PARTICLES * sizeof(Particle));
    
    // Initialize data
    for (int i = 0; i < NUM_PARTICLES; i++) {
        particles[i].x = 0.0f;
        particles[i].y = 0.0f;
        particles[i].vx = 1.0f;
        particles[i].vy = 1.0f;
        particles[i].mass = 100.0f;
        particles[i].density = 1.0f;
    }

    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);

    // The Cache-Busting Hot Loop
    for (int i = 0; i < NUM_PARTICLES; i++) {
        particles[i].x += particles[i].vx;
        particles[i].y += particles[i].vy;
    }

    clock_gettime(CLOCK_MONOTONIC, &end);
    
    double elapsed = (end.tv_sec - start.tv_sec) + 
                     (end.tv_nsec - start.tv_nsec) / 1e9;
    
    printf("Sample output: %.2f\n", particles[NUM_PARTICLES-1].x);
    printf("Execution Time (AoS): %.4f seconds\n", elapsed);
    
    free(particles);
    return 0;
}
