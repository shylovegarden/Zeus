#include "domain_capture_test.h"
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

#line 2 "domain_capture_test.zs"
extern zeus_tensor matmul(zeus_tensor A, zeus_tensor B);
#line 4 "domain_capture_test.zs"
// [ZEUS: NATIVE ECS ECS_BUFFER for 'Transform']
#line 11 "domain_capture_test.zs"
void process_secure_data() {
    // [ZEUS: INTEL SGX / AMD SEV ENCLAVE ENTRY]
    #pragma zeus_enclave_enter
    { 
        double key = 1.23;
        double result = key;
        memset(&key, 0, sizeof(key));
    }
    #pragma zeus_enclave_exit
    // [ZEUS: ENCLAVE EXIT]
}

#line 19 "domain_capture_test.zs"
void ai_matmul(zeus_tensor A, zeus_tensor B) {
    matmul(A, B);
}

#line 25 "domain_capture_test.zs"
void main() {
    process_secure_data();
}

