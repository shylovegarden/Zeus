import sys

with open('src/codegen.rs', 'r') as f:
    content = f.read()

# 1. Restore Windows stub
old_stub = """        source.push_str("#if defined(__unix__) || defined(__APPLE__)\\n");
        source.push_str("#pragma GCC diagnostic push\\n");
        source.push_str("#pragma GCC diagnostic ignored \\"-Wdeprecated-declarations\\"\\n");
        source.push_str("#include <ucontext.h>\\n");
        source.push_str("#pragma GCC diagnostic pop\\n");
        source.push_str("#include <unistd.h>\\n\\n");"""

new_stub = """        source.push_str("#if defined(_WIN32) || defined(_WIN64)\\n");
        source.push_str("typedef struct ucontext_t { void* dummy; struct { void* ss_sp; size_t ss_size; } uc_stack; struct ucontext_t* uc_link; } ucontext_t;\\n");
        source.push_str("static inline int getcontext(ucontext_t *ucp) { return 0; }\\n");
        source.push_str("static inline void makecontext(ucontext_t *ucp, void (*func)(), int argc, ...) {}\\n");
        source.push_str("static inline int swapcontext(ucontext_t *oucp, const ucontext_t *ucp) { return 0; }\\n");
        source.push_str("#define _SC_NPROCESSORS_ONLN 1\\n");
        source.push_str("static inline long sysconf(int name) { return 4; }\\n");
        source.push_str("#else\\n");
        source.push_str("#pragma GCC diagnostic push\\n");
        source.push_str("#pragma GCC diagnostic ignored \\"-Wdeprecated-declarations\\"\\n");
        source.push_str("#include <ucontext.h>\\n");
        source.push_str("#pragma GCC diagnostic pop\\n");
        source.push_str("#include <unistd.h>\\n");
        source.push_str("#endif\\n\\n");"""
content = content.replace(old_stub, new_stub)

# 2. Add OramAccess to find_referenced_in_expr
old_oram_expr = """            Expression::IndexAccess { base, index } => {
                self.find_referenced_in_expr(base, iterator, local_vars, referenced);
                self.find_referenced_in_expr(index, iterator, local_vars, referenced);
            }"""
new_oram_expr = """            Expression::IndexAccess { base, index } | Expression::OramAccess { base, index } => {
                self.find_referenced_in_expr(base, iterator, local_vars, referenced);
                self.find_referenced_in_expr(index, iterator, local_vars, referenced);
            }"""
content = content.replace(old_oram_expr, new_oram_expr)

# 3. Replace __rdtsc() % 2 with __zeus_rand() % 2 in OramAccess
content = content.replace("__rdtsc() % 2", "__zeus_rand() % 2")

# 4. Add stdatomic.h and spinlock
old_includes = """        source.push_str("#include <stdint.h>\\n");
        source.push_str("#include <string.h>\\n");"""
new_includes = """        source.push_str("#include <stdint.h>\\n");
        source.push_str("#include <string.h>\\n");
        source.push_str("#include <stdatomic.h>\\n");
        source.push_str("static volatile atomic_flag __zeus_ledger_lock = ATOMIC_FLAG_INIT;\\n");"""
content = content.replace(old_includes, new_includes)

# 5. Add atomic spinlock to zeus_serialize_mutation_ledger
old_ledger = """        source.push_str("void __zeus_serialize_mutation_ledger(const char* func_name, const char* data) {\\n");
        source.push_str("    FILE* _f = fopen(\\"mutation_ledger.json\\", \\"a\\");\\n");
        source.push_str("    if (_f) {\\n");
        source.push_str("        fprintf(_f, \\"{\\\\\\"function\\\\\\": \\\\\\"%s\\\\\\", \\\\\\"timestamp\\\\\\": %llu, \\\\\\"data\\\\\\": \\\\\\"%s\\\\\\"}\\\\n\\", func_name, __rdtsc(), data);\\n");
        source.push_str("        fclose(_f);\\n");
        source.push_str("    }\\n");
        source.push_str("}\\n");"""

new_ledger = """        source.push_str("void __zeus_serialize_mutation_ledger(const char* func_name, const char* data) {\\n");
        source.push_str("    while (atomic_flag_test_and_set_explicit(&__zeus_ledger_lock, memory_order_acquire)) { }\\n");
        source.push_str("    FILE* _f = fopen(\\"mutation_ledger.json\\", \\"a\\");\\n");
        source.push_str("    if (_f) {\\n");
        source.push_str("        fprintf(_f, \\"{\\\\\\"function\\\\\\": \\\\\\"%s\\\\\\", \\\\\\"timestamp\\\\\\": %llu, \\\\\\"data\\\\\\": \\\\\\"%s\\\\\\"}\\\\n\\", func_name, __zeus_rand(), data);\\n");
        source.push_str("        fclose(_f);\\n");
        source.push_str("    }\\n");
        source.push_str("    atomic_flag_clear_explicit(&__zeus_ledger_lock, memory_order_release);\\n");
        source.push_str("}\\n");"""
content = content.replace(old_ledger, new_ledger)

with open('src/codegen.rs', 'w') as f:
    f.write(content)

print('Success')
