#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <time.h>

typedef struct {
    uint32_t id;
    uint64_t timestamp;
    uint8_t data_buffer[8];
} DiagnosticPayload;

uint32_t legacy_process_stream(DiagnosticPayload* payloads, size_t count) {
    uint32_t high_priority_anomalies = 0;
    for (size_t i = 0; i < count; i++) {
        if (payloads[i].data_buffer[0] == 0xFF) {
            high_priority_anomalies++;
        }
    }
    return high_priority_anomalies;
}

int main() {
    size_t count = 50000000; // 50 Million Frames
    DiagnosticPayload* stream = (DiagnosticPayload*)calloc(count, sizeof(DiagnosticPayload));
    
    // Seed some anomalies
    stream[count / 2].data_buffer[0] = 0xFF;
    stream[count - 1].data_buffer[0] = 0xFF;

    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);
    
    uint32_t alerts = legacy_process_stream(stream, count);
    
    clock_gettime(CLOCK_MONOTONIC, &end);
    double time_taken = (end.tv_sec - start.tv_sec) * 1e6 + (end.tv_nsec - start.tv_nsec) / 1e3;
    
    printf("Legacy C processed 50M frames. Alerts found: %u\n", alerts);
    printf("Time taken: %.2f microseconds\n", time_taken);

    free(stream);
    return 0;
}
