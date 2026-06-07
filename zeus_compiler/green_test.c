#include "green_test.h"
#include <stdio.h>
#include <stdlib.h>

void zeus_free_tensor(zeus_tensor* t) {
    if (t && t->data) {
        free(t->data);
        t->data = NULL;
    }
}

double process_data() {
    double result = 0;
    for (int i = 0; i < 5000; ++i) {
        result = result + 2;
    }
    // [ZEUS: PARALLEL BLOCK START (OpenMP/Threads)]
    result = result * 2;
    // [ZEUS: PARALLEL BLOCK END]
    return result;
}

