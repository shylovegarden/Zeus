#include "test_soa.h"
#include <stdio.h>
#include <stdlib.h>

void zeus_free_tensor(zeus_tensor* t) {
    if (t && t->data) {
        free(t->data);
        t->data = NULL;
    }
}

// [ZEUS: Struct 'Point' registered for SoA Flattening]
void main() {
    // [ZEUS: INVISIBLE SoA TRANSFORMATION for 'points']
    double points_x[100];
    double points_y[100];
    double points_z[100];
    (points_x[(int)10] = 5);
    (points_y[(int)10] = 10);
    double flags = ((int)1 << (int)2);
    double mask = ((int)flags & (int)4);
}

