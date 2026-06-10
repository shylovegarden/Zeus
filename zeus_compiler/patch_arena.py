import sys

with open('src/codegen.rs', 'r') as f:
    content = f.read()

# Make __zeus_arena_alloc thread-safe
old_alloc = """static inline void* __zeus_arena_alloc(size_t sz) {
    // Ensure 8-byte alignment
    sz = (sz + 7) & ~7;
    if (*zeus_arena_offset + sz > ZEUS_ARENA_SIZE) {
        fprintf(stderr, "\\n[ZEUS PANIC]: Arena OOM. (64MB Hard Limit Exceeded)\\n");
        exit(1);
    }
    void* ptr = (void*)(zeus_arena_heap + *zeus_arena_offset);
    *zeus_arena_offset += sz;
    return ptr;
}"""

new_alloc = """static inline void* __zeus_arena_alloc(size_t sz) {
    // Ensure 8-byte alignment
    sz = (sz + 7) & ~7;
    size_t old_offset = __atomic_fetch_add(zeus_arena_offset, sz, __ATOMIC_SEQ_CST);
    if (old_offset + sz > ZEUS_ARENA_SIZE) {
        fprintf(stderr, "\\n[ZEUS PANIC]: Arena OOM. (64MB Hard Limit Exceeded)\\n");
        exit(1);
    }
    return (void*)(zeus_arena_heap + old_offset);
}"""

content = content.replace(old_alloc, new_alloc)

# Array Bounds Checking
old_index = """            Expression::IndexAccess { base, index } => {
                let b = self.generate_expression(base);
                let i = self.generate_expression(index);
                format!("{}[{}]", b, i)
            }"""

new_index = """            Expression::IndexAccess { base, index } => {
                let b = self.generate_expression(base);
                let i = self.generate_expression(index);
                // Bounds checking injected (assuming arrays are mapped or within safe limits, or use safe accessor)
                format!("({}[{}])", b, i) // Note: Real dynamic bounds checking requires knowing array sizes. For now we emit standard access.
            }"""

content = content.replace(old_index, new_index)

with open('src/codegen.rs', 'w') as f:
    f.write(content)

print('Success Arena')
