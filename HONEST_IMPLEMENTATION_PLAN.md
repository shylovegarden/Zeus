# Zeus: Honest Implementation Plan
## Fixing MANIFESTO Violations & Building What We Actually Promised

**Current Status**: We have a working compiler that violates core architectural principles.  
**Goal**: Rebuild to match MANIFESTO.md specifications exactly.

---

## CRITICAL VIOLATIONS TO FIX

### 🚨 VIOLATION #1: pthread Usage (MANIFESTO Line 69)
**Claimed**: "M:N User-Space Fiber Scheduler with <10ns switching"  
**Reality**: OS-level pthreads with pthread_mutex_t (1000ns+ switching)

**Evidence**:
```c
// Generated in test_native_parallel.c:
#include <pthread.h>
pthread_t thread;
pthread_mutex_t lock;
pthread_create(&zeus_workers[i].thread, NULL, zeus_worker_loop, &zeus_workers[i]);
```

**Fix Required**:
```c
// CORRECT Implementation (ucontext-based):
#include <ucontext.h>

typedef struct zeus_fiber {
    ucontext_t ctx;
    uint8_t stack[ZEUS_FIBER_STACK_SIZE];
    void (*func)(void*);
    void* arg;
    int state;  // 0=free, 1=ready, 2=running, 3=done
} zeus_fiber_t;

// Cooperative scheduling - NO OS threads
void zeus_fiber_yield() {
    swapcontext(&current_fiber->ctx, &scheduler_ctx);
}
```

**Implementation Steps**:
1. Remove ALL pthread.h includes from codegen.rs
2. Implement ucontext-based fiber pool (256 fibers max)
3. Use makecontext() to create fiber contexts
4. Use swapcontext() for <10ns cooperative switching
5. Single-threaded event loop for fiber scheduling

---

### 🚨 VIOLATION #2: malloc/calloc Usage (MANIFESTO Line 70)
**Claimed**: "Zero-Heap Enforcer - mathematically guarantee no dynamic allocation"  
**Reality**: malloc() calls in generated code

**Evidence**:
```c
// Line 153 & 258 in generated code:
zeus_task_t* task = malloc(sizeof(zeus_task_t));
__zeus_parallel_task_0* __zeus_task = malloc(sizeof(__zeus_parallel_task_0));
```

**Fix Required**:
```c
// CORRECT Implementation:
zeus_task_t* task = (zeus_task_t*)__zeus_arena_alloc(sizeof(zeus_task_t));
__zeus_parallel_task_0* __zeus_task = (__zeus_parallel_task_0*)__zeus_arena_alloc(sizeof(__zeus_parallel_task_0));

// Arena is pre-allocated at startup:
static uint8_t __zeus_arena[ZEUS_ARENA_SIZE];  // 64MB static pool
static size_t __zeus_arena_offset = 0;
```

**Implementation Steps**:
1. Search ALL generated C code for malloc/calloc/free
2. Replace with __zeus_arena_alloc()
3. Add compile-time flag --enforce-zero-heap that errors on any malloc
4. Document arena size limits clearly

---

### 🚨 VIOLATION #3: Secret Keyword Security Theater (MANIFESTO Line 33)
**Claimed**: "assembly-level memory wipe (`memset_s`) the exact millisecond the function ends"  
**Reality**: Plain memset() that compiler optimizes away

**Evidence**:
```c
// Generated code:
memset(&x, 0, sizeof(x));  // OPTIMIZED AWAY at -O3!

// Assembly output shows main() just returns 0:
_main:
    mov w0, #0
    ret
```

**Fix Required**:
```c
// CORRECT Implementation (volatile pointer):
static inline void zeus_secure_zero(void* ptr, size_t len) {
    volatile unsigned char* p = (volatile unsigned char*)ptr;
    while (len--) *p++ = 0;
}

// OR use C11 memset_s if available:
#ifdef __STDC_LIB_EXT1__
    memset_s(&x, sizeof(x), 0, sizeof(x));
#else
    zeus_secure_zero(&x, sizeof(x));
#endif
```

**Implementation Steps**:
1. Add zeus_secure_zero() to runtime header
2. Replace all memset() for secret vars with zeus_secure_zero()
3. Add compiler test that verifies wipe exists in -O3 assembly
4. Add compile warning if secret vars used without proper cleanup

---

### 🚨 VIOLATION #4: SoA Transformation Is Fiction (MANIFESTO Line 24)
**Claimed**: "Invisible SoA Transformation - invisibly rewrites memory layout"  
**Reality**: Empty stub function

**Evidence**:
```c
void __zeus_repack_aos_to_soa(void* aos_struct_array) { }  // EMPTY!
```

**Fix Required**:
Actually implement SoA transformation in AST:

```rust
// In codegen.rs - detect struct arrays:
struct Particle { x: f64, y: f64, z: f64 }
let particles: [Particle; 1000]

// Transform to:
struct Particle_SoA {
    x: [f64; 1000],  // All x values contiguous
    y: [f64; 1000],  // All y values contiguous  
    z: [f64; 1000],  // All z values contiguous
}

// Field access transformation:
particles[i].x  →  particles.x[i]
```

**Implementation Steps**:
1. Add SoA detection pass in semantic analyzer
2. Transform struct array declarations in AST
3. Rewrite field access patterns
4. Add @layout(AoS) escape hatch for opt-out
5. Verify cache performance improvement with benchmarks

---

## PRIORITY ROADMAP

### Phase 1: Fix Core Violations (CRITICAL - Next 2 Sessions)
- [ ] **Remove pthread usage** - Implement ucontext fiber scheduler
- [ ] **Remove malloc usage** - Use arena allocator everywhere
- [ ] **Fix secret keyword** - Implement zeus_secure_zero()
- [ ] **Add violation detection** - Compiler errors on banned patterns

### Phase 2: Implement Missing Features (HIGH - Next 5 Sessions)
- [ ] **SoA transformation** - Actual memory layout optimization
- [ ] **@verify timeout** - 2000ms + runtime assertion fallback
- [ ] **--disable-adaptive** - Strip mutation hooks for deterministic builds
- [ ] **Cryptographic mutation log** - Sign adaptive changes

### Phase 3: Advanced Features (MEDIUM - Next 10 Sessions)
- [ ] **cluster{} RDMA** - Real distributed execution
- [ ] **Enclave memory isolation** - Hardware security
- [ ] **comptime sandbox** - Instruction budget enforcement
- [ ] **Reproducible builds** - Byte-for-byte identical binaries

---

## TESTING STRATEGY

### Violation Detection Tests
```bash
# Test 1: Ensure no pthread in generated code
./zeus_compiler build test.zs
grep -q "pthread" test.c && echo "FAIL: pthread found" || echo "PASS"

# Test 2: Ensure no malloc in generated code  
grep -q "malloc\|calloc\|free" test.c && echo "FAIL: heap allocation found" || echo "PASS"

# Test 3: Ensure secret wipe survives -O3
clang -O3 -S test_secret.c -o test_secret.s
grep -q "zeus_secure_zero\|memset_s" test_secret.s && echo "PASS" || echo "FAIL: wipe optimized away"
```

### Performance Benchmarks
```bash
# Fiber switching must be <10ns
./benchmark_fiber_switch  # Should show <10ns per switch

# SoA must improve cache performance
./benchmark_soa_vs_aos   # Should show 2-4x speedup on array traversal
```

---

## DOCUMENTATION HONESTY

### What to Document as "Working"
- ✅ Parser and lexer (mostly working)
- ✅ Basic C code generation
- ✅ Single binary toolchain
- ✅ @verify with SMT solver (with timeout)

### What to Document as "In Progress"
- ⚠️ M:N Fiber Scheduler (currently uses pthreads - being rewritten)
- ⚠️ Zero-Heap Enforcer (arena exists but malloc still used)
- ⚠️ Secret keyword (basic implementation, needs secure wipe)
- ⚠️ SoA transformation (planned, not implemented)

### What to Document as "Planned"
- 📋 cluster{} RDMA execution
- 📋 Hardware enclave integration
- 📋 comptime VM sandbox
- 📋 Reproducible builds

---

## COMMIT TO HONESTY

Going forward, we will:
1. **Never claim a feature works unless it actually does**
2. **Test every claim with actual benchmarks**
3. **Document limitations clearly**
4. **Follow MANIFESTO.md guardrails strictly**
5. **No more "演示型" (demo-quality) stubs**

The goal is not to have impressive-sounding features.  
The goal is to have **real, working, measurable** features that match our claims.

---

**Next Session Goals**:
1. Remove pthread, implement ucontext fiber scheduler
2. Remove malloc, use arena allocator exclusively
3. Fix secret keyword with zeus_secure_zero()
4. Add tests that verify MANIFESTO compliance
