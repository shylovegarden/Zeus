#include "dummy_error.h"
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
#line 2 "dummy_error.zs"
    var;
#line 6 "dummy_error.zs"
#line 6 "dummy_error.zs"
    x;
#line 9 "dummy_error.zs"
#line 9 "dummy_error.zs"
    Assign;
#line 12 "dummy_error.zs"
#line 12 "dummy_error.zs"
    10;
#line 16 "dummy_error.zs"
#line 16 "dummy_error.zs"
    var;
#line 20 "dummy_error.zs"
#line 20 "dummy_error.zs"
    y;
#line 23 "dummy_error.zs"
#line 23 "dummy_error.zs"
    Assign;
#line 26 "dummy_error.zs"
#line 26 "dummy_error.zs"
    20;
#line 30 "dummy_error.zs"
#line 30 "dummy_error.zs"
    var;
#line 34 "dummy_error.zs"
#line 34 "dummy_error.zs"
    z;
#line 37 "dummy_error.zs"
#line 37 "dummy_error.zs"
    Assign;
#line 40 "dummy_error.zs"
#line 40 "dummy_error.zs"
    x;
#line 44 "dummy_error.zs"
#line 44 "dummy_error.zs"
    print(z);
#line 46 "dummy_error.zs"
    return 0;
}
