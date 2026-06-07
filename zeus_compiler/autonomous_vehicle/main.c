#include "main.h"
#include <stdio.h>
#include <stdlib.h>

void zeus_free_tensor(zeus_tensor* t) {
    if (t && t->data) {
        free(t->data);
        t->data = NULL;
    }
}

double zeus_hw_read_sensor() {
    return 42;
}

void main() {
    double sensor_data = zeus_hw_read_sensor();
    double result = sensor_data * 2;
}

