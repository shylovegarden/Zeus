#include "dummy_test.h"
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

#line 1 "dummy_test.zs"
int main() {
    double x = 10;
    double y = 20;
    double z = (x + y);
    return 0;
}

