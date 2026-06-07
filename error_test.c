#include "error_test.h"
#include <stdio.h>
#include <stdlib.h>

void zeus_free_tensor(zeus_tensor* t) {
    if (t && t->data) {
        free(t->data);
        t->data = NULL;
    }
}

#line 2 "error_test.zs"
zeus_result_t read_sensor() {
    fprintf(stderr, "[ZEUS PANIC (SAFE STATE HW RESET)]: %s\n", "Sensor unplugged!");
    exit(1);
}

#line 9 "error_test.zs"
void process_data() {
    double data = 0;
    read_sensor();
}

#line 15 "error_test.zs"
void main() {
    process_data();
}

