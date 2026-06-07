#include "god_tier.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void zeus_free_tensor(zeus_tensor* t) {
    if (t && t->data) {
        free(t->data);
        t->data = NULL;
    }
}

void __zeus_safestate_handler() {
}

#line 1 "god_tier.zs"
// [ZEUS: Struct 'TelemetryFrame' registered for SoA Flattening]
typedef struct {
    double speed;
    double temp;
} TelemetryFrame;

#line 6 "god_tier.zs"
#line 7 "god_tier.zs"
#line 7 "god_tier.zs"
void process_telemetry(TelemetryFrame frame) {
    double speed = frame.speed;
    double temp = frame.temp;
    // [ZEUS CLUSTER DISTRIBUTION: RDMA INJECTED]
    (speed = (speed * 1.5));
    (temp = (temp + 10));
}

