#include "test.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
    double power_level = 100;
    double weights = /* Tensor[1024, 1024] */ 0.0;
    // [ZEUS: PARALLEL BLOCK START (OpenMP/Threads)]
    weights = weights * 1.618;
    printf("Execution complete.\n");
    // [ZEUS: PARALLEL BLOCK END]
    return 0;
}
