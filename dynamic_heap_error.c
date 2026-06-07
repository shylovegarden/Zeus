#include "dynamic_heap_error.h"
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

#line 2 "dynamic_heap_error.zs"
void main() {
    double n = 10;
    n;
}

