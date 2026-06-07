#include "advanced_interop.h"
#include <stdio.h>
#include <stdlib.h>

void zeus_free_tensor(zeus_tensor* t) {
    if (t && t->data) {
        free(t->data);
        t->data = NULL;
    }
}

double calculate_ai_load(zeus_tensor data, double iterations) {
    double result = 0;
    // [ZEUS: PARALLEL BLOCK START (OpenMP/Threads)]
    result = iterations * 2;
    // [ZEUS: PARALLEL BLOCK END]
    return result;
}

