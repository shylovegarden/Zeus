#include "hello_world.h"
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

#line 3 "hello_world.zs"
double compute(double n) {
    return (n * 1);
}

#line 7 "hello_world.zs"
void main() {
    double token = 42;
    // [ZEUS: PARALLEL BLOCK START (OpenMP/Threads)]
    double a = compute(3);
    double b = compute(3);
    double c = compute(3);
    // [ZEUS: PARALLEL BLOCK END]
    // [ZEUS: COMPILE-TIME PROOF BLOCK (Elided from Runtime)]
    memset(&token, 0, sizeof(token));
}

#line 24 "hello_world.zs"
// [ZEUS: TEST BLOCK 'sanity' (Elided from Build)]
