import sys

with open('src/codegen.rs', 'r') as f:
    content = f.read()

old_push = """        // Push (owner only, no CAS needed for bottom)
        source.push_str("static inline void zeus_wsdeque_push(zeus_wsdeque_t* q, void* task) {\\n");
        source.push_str("    size_t b = __atomic_load_n(&q->bottom, __ATOMIC_RELAXED);\\n");
        source.push_str("    q->tasks[b % ZEUS_WSQ_CAPACITY] = task;\\n");
        source.push_str("    __atomic_thread_fence(__ATOMIC_RELEASE);\\n");
        source.push_str("    __atomic_store_n(&q->bottom, b + 1, __ATOMIC_RELAXED);\\n");
        source.push_str("}\\n\\n");"""

new_push = """        // Push (owner only, no CAS needed for bottom)
        source.push_str("static inline void zeus_wsdeque_push(zeus_wsdeque_t* q, void* task) {\\n");
        source.push_str("    size_t b = __atomic_load_n(&q->bottom, __ATOMIC_RELAXED);\\n");
        source.push_str("    __atomic_store_n(&q->tasks[b % ZEUS_WSQ_CAPACITY], task, __ATOMIC_RELEASE);\\n");
        source.push_str("    __atomic_store_n(&q->bottom, b + 1, __ATOMIC_RELEASE);\\n");
        source.push_str("}\\n\\n");"""

old_pop = """        source.push_str("    if (t <= b) {\\n");
        source.push_str("        void* task = q->tasks[b % ZEUS_WSQ_CAPACITY];\\n");
        source.push_str("        if (t == b) {\\n");"""

new_pop = """        source.push_str("    if (t <= b) {\\n");
        source.push_str("        void* task = __atomic_load_n(&q->tasks[b % ZEUS_WSQ_CAPACITY], __ATOMIC_ACQUIRE);\\n");
        source.push_str("        if (t == b) {\\n");"""

old_steal = """        source.push_str("    if (t < b) {\\n");
        source.push_str("        void* task = q->tasks[t % ZEUS_WSQ_CAPACITY];\\n");
        source.push_str("        if (!__atomic_compare_exchange_n(&q->top, &t, t + 1, 0, __ATOMIC_SEQ_CST, __ATOMIC_RELAXED)) {\\n");"""

new_steal = """        source.push_str("    if (t < b) {\\n");
        source.push_str("        void* task = __atomic_load_n(&q->tasks[t % ZEUS_WSQ_CAPACITY], __ATOMIC_ACQUIRE);\\n");
        source.push_str("        if (!__atomic_compare_exchange_n(&q->top, &t, t + 1, 0, __ATOMIC_SEQ_CST, __ATOMIC_RELAXED)) {\\n");"""

content = content.replace(old_push, new_push)
content = content.replace(old_pop, new_pop)
content = content.replace(old_steal, new_steal)

with open('src/codegen.rs', 'w') as f:
    f.write(content)

print('Success')
