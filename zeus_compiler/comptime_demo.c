#include "comptime_demo.h"
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

int main() {
#line 4 "comptime_demo.zs"
    double obd_diagnostic_offset = 550;
#line 4 "comptime_demo.zs"
#line 6 "comptime_demo.zs"
#line 6 "comptime_demo.zs"
#line 6 "comptime_demo.zs"
    "x86_64";
#line 6 "comptime_demo.zs"
#line 11 "comptime_demo.zs"
#line 11 "comptime_demo.zs"
    double raw_rpm = 1000;
#line 14 "comptime_demo.zs"
#line 14 "comptime_demo.zs"
    double true_rpm = (raw_rpm + obd_diagnostic_offset);
#line 15 "comptime_demo.zs"
#line 16 "comptime_demo.zs"
    return 0;
}
