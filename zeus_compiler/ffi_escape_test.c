#include "ffi_escape_test.h"
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

#line 1 "ffi_escape_test.zs"
// [ZEUS: Struct 'Point' registered for SoA Flattening]
#line 6 "ffi_escape_test.zs"
extern void c_transform(Point* p);
#line 8 "ffi_escape_test.zs"
void main() {
    // [ZEUS: INVISIBLE SoA TRANSFORMATION for 'points']
    double points_x[10];
    double points_y[10];
    (points_x[0] = 5);
    ({ Point _zeus_tmp_0; _zeus_tmp_0.x = points_x[0]; _zeus_tmp_0.y = points_y[0]; c_transform(&_zeus_tmp_0); points_x[0] = _zeus_tmp_0.x; points_y[0] = _zeus_tmp_0.y; });
}

