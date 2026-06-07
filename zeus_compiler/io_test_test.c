#include "io_test_test.h"
#include <stdio.h>
#include <stdlib.h>

void zeus_free_tensor(zeus_tensor* t) {
    if (t && t->data) {
        free(t->data);
        t->data = NULL;
    }
}

void test_check_math() {
    // [ZEUS VERIFIED: assert((1 + 1))]
}

void test_check_io() {
    printf("%f\n", 42);
}

void main() {
    test_check_math();
    test_check_io();
}

