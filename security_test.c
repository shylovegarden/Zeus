#include "security_test.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void zeus_free_tensor(zeus_tensor* t) {
    if (t && t->data) {
        free(t->data);
        t->data = NULL;
    }
}

#line 1 "error_test.zs"
void encrypt_data() {
    double key = 12345.678;
    double data = (key * 2);
    memset(&key, 0, sizeof(key));
}

#line 6 "error_test.zs"
void main() {
    encrypt_data();
}

