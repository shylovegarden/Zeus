import sys

with open('src/codegen.rs', 'r') as f:
    content = f.read()

# Fix __zeus_rand()
if "unsigned int __zeus_rand()" not in content:
    content = content.replace("static volatile atomic_flag __zeus_ledger_lock = ATOMIC_FLAG_INIT;\\n", "static volatile atomic_flag __zeus_ledger_lock = ATOMIC_FLAG_INIT;\\nstatic inline unsigned int __zeus_rand() { return rand(); }\\n")

# Fix #endif without #if. In my previous replace, I might have messed up the #endif.
# Let's just find and print the includes block around line 68.
print(content[content.find("#include <stdatomic.h>"):content.find("// Zeus Runtime Security FFI Stubs")])

with open('src/codegen.rs', 'w') as f:
    f.write(content)
