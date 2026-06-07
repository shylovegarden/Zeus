#include "firmware_io_test.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void zeus_free_tensor(zeus_tensor* t) {
    if (t && t->data) {
        free(t->data);
        t->data = NULL;
    }
}

const char* ZEUS_CRT0 = ".global _reset\n_reset:\n  ldr sp, =0x20000000\n  bl main\n  b .\n";

void __zeus_safestate_handler() {
}

#line 1 "firmware_io_test.zs"
// [ZEUS: SAFESTATE BLOCK (Emitted globally)]
#line 5 "firmware_io_test.zs"
void main() {
    0;
    fprintf(stderr, "[ZEUS PANIC (SAFE STATE HW RESET)]: %s\n", "Simulated Hardware Fault");
    __zeus_safestate_handler();
    exit(1);
}

