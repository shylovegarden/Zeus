#include "main.h"
#include <stdio.h>
#include <stdlib.h>

void zeus_free_tensor(zeus_tensor* t) {
    if (t && t->data) {
        free(t->data);
        t->data = NULL;
    }
}

// [ZEUS: Struct 'CanBusFrame' registered for SoA Flattening]
double zeus_hw_read_sensor() {
    return 42;
}

double decode_can_speed(double payload) {
    double speed = ((int)((int)payload >> (int)8) & (int)255);
    return speed;
}

double is_critical_fault(double payload) {
    double fault = ((int)((int)payload >> (int)31) & (int)1);
    return fault;
}

void process_telemetry() {
    // [ZEUS: INVISIBLE SoA TRANSFORMATION for 'frames']
    double frames_id[10];
    double frames_dlc[10];
    double frames_payload[10];
    (frames_id[(int)0] = 1);
    (frames_dlc[(int)0] = 8);
    double raw_payload = ((int)((int)1 << (int)31) | (int)((int)65 << (int)8));
    (frames_payload[(int)0] = raw_payload);
    for (int i = 0; i < 1; ++i) {
        double speed = decode_can_speed(frames_payload[(int)i]);
        double fault = is_critical_fault(frames_payload[(int)i]);
    }
}

