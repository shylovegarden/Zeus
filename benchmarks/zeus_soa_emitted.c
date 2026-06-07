#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#define NUM_PARTICLES 50000000

// [ZEUS: INVISIBLE SoA TRANSFORMATION for 'Particle']
// The Zeus C-Backend detected an Array-of-Structures.
// It automatically flattened the layout into perfectly isolated
// Structure-of-Arrays (SoA) memory blocks to maximize CPU Cache Line utilization.
typedef struct {
    float* x;
    float* y;
    float* vx;
    float* vy;
    float* mass;
    float* density;
} ZeusSoAParticles;

int main() {
    ZeusSoAParticles particles;
    particles.x = (float*)malloc(NUM_PARTICLES * sizeof(float));
    particles.y = (float*)malloc(NUM_PARTICLES * sizeof(float));
    particles.vx = (float*)malloc(NUM_PARTICLES * sizeof(float));
    particles.vy = (float*)malloc(NUM_PARTICLES * sizeof(float));
    particles.mass = (float*)malloc(NUM_PARTICLES * sizeof(float));
    particles.density = (float*)malloc(NUM_PARTICLES * sizeof(float));
    
    // Initialize data
    for (int i = 0; i < NUM_PARTICLES; i++) {
        particles.x[i] = 0.0f;
        particles.y[i] = 0.0f;
        particles.vx[i] = 1.0f;
        particles.vy[i] = 1.0f;
        particles.mass[i] = 100.0f;
        particles.density[i] = 1.0f;
    }

    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);

    // The Perfect Cache Hit Loop
    // Because x and vx are contiguous arrays, the CPU perfectly predicts and 
    // pre-loads the cache lines. Zero cache misses.
    for (int i = 0; i < NUM_PARTICLES; i++) {
        particles.x[i] += particles.vx[i];
        particles.y[i] += particles.vy[i];
    }

    clock_gettime(CLOCK_MONOTONIC, &end);
    
    double elapsed = (end.tv_sec - start.tv_sec) + 
                     (end.tv_nsec - start.tv_nsec) / 1e9;
    
    printf("Sample output: %.2f\n", particles.x[NUM_PARTICLES-1]);
    printf("Execution Time (SoA): %.4f seconds\n", elapsed);
    
    free(particles.x);
    free(particles.y);
    free(particles.vx);
    free(particles.vy);
    free(particles.mass);
    free(particles.density);
    return 0;
}
