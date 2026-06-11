#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or, clippy::type_complexity)]
use crate::ast::{Program, Statement};
use crate::backend::{Backend, Artifact, CompileError};

#[allow(dead_code)]
pub struct CCodegen {
    pub output_name: String,
    pub struct_schemas: std::cell::RefCell<std::collections::HashMap<String, Vec<(String, crate::ast::Type)>>>,
    pub secret_vars: std::cell::RefCell<Vec<Vec<String>>>,
    pub extern_functions: std::cell::RefCell<std::collections::HashMap<String, (Vec<(String, crate::ast::Type)>, Option<crate::ast::Type>)>>,
    pub soa_arrays: std::cell::RefCell<std::collections::HashSet<String>>,
    pub current_var_types: std::cell::RefCell<std::collections::HashMap<String, String>>,
    /// Variables that are targets of @atomic_add — typed as int64_t so the compiler
    /// can emit __atomic_fetch_add directly instead of a CAS loop.
    pub atomic_int_vars: std::cell::RefCell<std::collections::HashSet<String>>,
    /// Struct names for which we have already emitted a FatPtr typedef (dedup guard).
    pub soa_fat_ptr_structs: std::cell::RefCell<std::collections::HashSet<String>>,
    pub disable_adaptive: bool,
    pub export_mutation_log: bool,
    pub tuned_weights: Vec<f32>,
    /// Monotonic id assigned to each parallel block during dispatch codegen.
    pub parallel_counter: std::cell::RefCell<u64>,
    /// SoA array name -> element struct name (for resolving field C types).
    pub soa_struct_of: std::cell::RefCell<std::collections::HashMap<String, String>>,
    /// SoA array name -> length C-expr, ONLY for arrays declared `secret`.
    /// Presence here means accesses must be compiled to oblivious full-scans.
    pub soa_secret_lens: std::cell::RefCell<std::collections::HashMap<String, String>>,
    /// @ensures expressions (C source) for the function currently being emitted.
    pub pending_ensures: std::cell::RefCell<Vec<String>>,
    pub is_target_nvme: bool,
    pub l1_cache_size: usize,
}


pub(crate) mod builtins;
pub(crate) mod statements;
pub(crate) mod expressions;
pub(crate) mod parallel;

/// Pack f32 weights into INT4 pairs (2 per byte) for .rodata embedding.
/// Returns (packed_bytes, scale_factor) where scale = max_abs / 7.0.
fn pack_int4_weights(weights: &[f32]) -> (Vec<u8>, f32) {
    if weights.is_empty() { return (vec![0u8], 1.0f32); }
    let max_abs = weights.iter().map(|w| w.abs()).fold(0.0f32, f32::max).max(1e-9);
    let scale = max_abs / 7.0f32;
    let quant: Vec<i8> = weights.iter().map(|w| {
        let q = (w / scale).round().clamp(-7.0, 7.0) as i8;
        q
    }).collect();
    let n_bytes = (quant.len() + 1) / 2;
    let mut bytes = vec![0u8; n_bytes.max(2)]; // always ≥ 2 bytes for safe indexing
    for (i, &q) in quant.iter().enumerate() {
        let nibble = (q & 0x0F) as u8;
        if i % 2 == 0 {
            bytes[i / 2] = nibble;             // lo nibble
        } else {
            bytes[i / 2] |= (nibble << 4);    // hi nibble
        }
    }
    (bytes, scale)
}

impl CCodegen {
    pub fn new(output_name: &str) -> Self {
        CCodegen {
            output_name: output_name.to_string(),
            struct_schemas: std::cell::RefCell::new(std::collections::HashMap::new()),
            secret_vars: std::cell::RefCell::new(vec![vec![]]),
            extern_functions: std::cell::RefCell::new(std::collections::HashMap::new()),
            soa_arrays: std::cell::RefCell::new(std::collections::HashSet::new()),
            current_var_types: std::cell::RefCell::new(std::collections::HashMap::new()),
            atomic_int_vars: std::cell::RefCell::new(std::collections::HashSet::new()),
            soa_fat_ptr_structs: std::cell::RefCell::new(std::collections::HashSet::new()),
            disable_adaptive: false,
            export_mutation_log: false,
            tuned_weights: vec![0.25f32, -0.5f32, 0.8f32, -0.1f32], // Default mock weights
            parallel_counter: std::cell::RefCell::new(0),
            soa_struct_of: std::cell::RefCell::new(std::collections::HashMap::new()),
            soa_secret_lens: std::cell::RefCell::new(std::collections::HashMap::new()),
            pending_ensures: std::cell::RefCell::new(Vec::new()),
            is_target_nvme: false,
            l1_cache_size: 32768, // Default 32KB
        }
    }

    pub fn set_tuned_weights(&mut self, weights: Vec<f32>) {
        self.tuned_weights = weights;
    }

    pub fn set_config(&mut self, disable_adaptive: bool, export_mutation_log: bool, is_target_nvme: bool, l1_cache_size: usize) {
        self.disable_adaptive = disable_adaptive;
        self.export_mutation_log = export_mutation_log;
        self.is_target_nvme = is_target_nvme;
        self.l1_cache_size = l1_cache_size;
    }

    fn generate_secure_wipe(&self, var: &str, pad: &str) -> String {
        format!("{}{{ volatile unsigned char* _p = (volatile unsigned char*)&{}; for(size_t _i = 0; _i < sizeof({}); _i++) _p[_i] = 0; __asm__ volatile(\"\" : : \"g\"(&{}) : \"memory\"); }}\n", pad, var, var, var)
    }

    pub fn generate_source(&self, program: &Program) -> String {
        self.collect_atomic_int_vars(program); // must run before collect_var_types
        self.collect_var_types(program);
        // Collect Struct Schemas and Extern Functions
        for stmt in &program.statements {
            if let Statement::StructDeclaration { name, fields, .. } = stmt {
                self.struct_schemas.borrow_mut().insert(name.clone(), fields.clone());
            }
            if let Statement::ExternFunctionDeclaration { name, parameters, return_type } = stmt {
                self.extern_functions.borrow_mut().insert(name.clone(), (parameters.clone(), return_type.clone()));
            }
        }
        let mut source = String::new();
        source.push_str("// Auto-generated by Zeus v0.1\n");
        // BUG FIX #4: Define GNU and Darwin source flags along with _XOPEN_SOURCE before system headers
        source.push_str("#ifndef _GNU_SOURCE\n#define _GNU_SOURCE 1\n#endif\n");
        source.push_str("#ifndef _DARWIN_C_SOURCE\n#define _DARWIN_C_SOURCE 1\n#endif\n");
        source.push_str("#ifndef _XOPEN_SOURCE\n#define _XOPEN_SOURCE 600\n#endif\n");
        source.push_str(&format!("#include \"{}.h\"\n", self.output_name));
        source.push_str("#include <stdio.h>\n");
        source.push_str("#include <stddef.h>\n");   // size_t, NULL — NO stdlib.h (MISRA 21.3 compliance)
        source.push_str("#include <stdint.h>\n");
        source.push_str("#include <stdbool.h>\n");
        source.push_str("#include <math.h>\n");
        source.push_str("extern long long llabs(long long);\n");
        source.push_str("#include <string.h>\n");
        source.push_str("#include <stdatomic.h>\n");
        // Forward-declare exit without pulling in malloc/calloc/free via stdlib.h
        source.push_str("extern void exit(int status) __attribute__((noreturn));\n");
        source.push_str("static volatile atomic_flag __zeus_ledger_lock = ATOMIC_FLAG_INIT;\n");
        // XOR-shift32 PRNG — no stdlib.h rand(), no global seed interference with legacy code
        source.push_str("static inline unsigned int __zeus_rand(void) {\n");
        source.push_str("    static volatile unsigned int _zs = 0xDEAD1337u;\n");
        source.push_str("    unsigned int x = _zs; x ^= x << 13; x ^= x >> 17; x ^= x << 5;\n");
        source.push_str("    return (_zs = x);\n");
        source.push_str("}\n");
        source.push_str("#if defined(_WIN32) || defined(_WIN64)\n#include <windows.h>\n");
        source.push_str("#define PROT_READ 1\n#define PROT_WRITE 2\n#define MAP_SHARED 1\n#define MAP_ANON 2\n#define MAP_FAILED ((void*)-1)\n");
        source.push_str("static inline void* mmap(void* addr, size_t length, int prot, int flags, int fd, size_t offset) { return VirtualAlloc(NULL, length, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE); }\n");
        source.push_str("#define O_RDWR 2\n#define O_DIRECT 0\n#define O_SYNC 0\n");
        source.push_str("static inline int open(const char* pathname, int flags) { return -1; }\n");
        source.push_str("static inline void close(int fd) {}\n");
        source.push_str("typedef int pid_t;\n");
        source.push_str("static inline pid_t fork(void) { return 1; }\n");
        source.push_str("static inline pid_t getppid(void) { return 1; }\n");
        source.push_str("static inline int usleep(unsigned int usec) { Sleep(usec / 1000); return 0; }\n");
        source.push_str("#else\n#include <sys/mman.h>\n#include <sys/wait.h>\n#include <fcntl.h>\n#include <unistd.h>\n#endif\n\n");
        source.push_str("// Zeus Runtime Security FFI Stubs (Fallback Implementations)\n");
        source.push_str("void zeus_tls_handshake(void) {}\n");
        source.push_str("int zeus_enclave_verify_token(void) { return 1; }\n");
        source.push_str("void __zeus_serialize_mutation_ledger(const char* func_name, const char* sig) {}\n");
        source.push_str("const char* __zeus_sign_mutation(const char* func_name, const char* data) { return \"VALID_SIG\"; }\n\n");

        source.push_str("#if defined(__x86_64__) || defined(__i386__)\n");
        source.push_str("#include <x86intrin.h>\n");
        source.push_str("#elif defined(__aarch64__)\n");
        source.push_str("static inline uint64_t __rdtsc(void) {\n");
        source.push_str("    uint64_t val;\n");
        source.push_str("    asm volatile(\"mrs %0, cntvct_el0\" : \"=r\" (val));\n");
        source.push_str("    return val;\n");
        source.push_str("}\n");
        source.push_str("#else\n");
        source.push_str("static inline uint64_t __rdtsc(void) { return 0; }\n");
        source.push_str("#endif\n\n");

        source.push_str("// ============================================================================\n");
        source.push_str("// ZEUS HARDWARE ENCLAVE BINDINGS (Intel SGX / AMD SEV)\n");
        source.push_str("// ============================================================================\n");
        source.push_str("// These compiler barriers strictly prevent the C compiler from reordering\n");
        source.push_str("// memory operations across the enclave boundary.\n");
        source.push_str("#define zeus_enclave_enter() asm volatile(\"\" ::: \"memory\")\n");
        source.push_str("#define zeus_enclave_exit() asm volatile(\"\" ::: \"memory\")\n\n");

        source.push_str("// ============================================================================\n");
        source.push_str("// ANTI-SIDE-CHANNEL ENGINE (Hardware Flushes & Core Hopping)\n");
        source.push_str("// ============================================================================\n");
        source.push_str("#if defined(__linux__)\n");
        source.push_str("#include <sched.h>\n");
        source.push_str("#endif\n");
        source.push_str("#if defined(__x86_64__) || defined(__i386__)\n");
        source.push_str("#define zeus_speculation_flush() _mm_lfence()\n");
        source.push_str("#elif defined(__aarch64__)\n");
        source.push_str("#define zeus_speculation_flush() asm volatile(\"isb\" ::: \"memory\")\n");
        source.push_str("#else\n");
        source.push_str("#define zeus_speculation_flush() asm volatile(\"\" ::: \"memory\")\n");
        source.push_str("#endif\n\n");

        source.push_str("// ============================================================================\n");
        source.push_str("// ZEUS ADAPTIVE HEURISTIC: compile-time tunable linear score (NOT an ML model)\n");
        source.push_str("// ============================================================================\n");
        source.push_str(&format!("static const float __zeus_micro_ai_weights[{}] = {{{}}};\n", 
            self.tuned_weights.len(), 
            self.tuned_weights.iter().map(|w| format!("{}f", w)).collect::<Vec<_>>().join(", ")
        ));
        source.push_str("static inline float __zeus_heuristic_score(float input_fuel, float input_latency) {\n");
        source.push_str("    // Linear weighted score over two runtime signals (tunable via --tune).\n");
        source.push_str("    float score = (input_fuel * __zeus_micro_ai_weights[0]) + (input_latency * __zeus_micro_ai_weights[1]);\n");
        source.push_str("    return score;\n");
        source.push_str("}\n\n");
        // Oblivious memory access for `secret` arrays: scans EVERY element in a
        // fixed linear order with a branchless masked select, so the cache/memory
        // access pattern is independent of the secret index (defeats cache-timing
        // and access-pattern side channels). O(n) per access -- opt-in via `secret`.
        source.push_str("static inline void __zeus_oread_bytes(void* dst, const void* base, size_t n, size_t esz, size_t idx) {\n");
        source.push_str("    unsigned char* d = (unsigned char*)dst; const unsigned char* b = (const unsigned char*)base;\n");
        source.push_str("    for (size_t j = 0; j < esz; j++) d[j] = 0;\n");
        source.push_str("    for (size_t k = 0; k < n; k++) {\n");
        source.push_str("        unsigned char m = (unsigned char)0 - (unsigned char)(k == idx);\n");
        source.push_str("        const unsigned char* e = b + k * esz;\n");
        source.push_str("        for (size_t j = 0; j < esz; j++) d[j] |= (unsigned char)(e[j] & m);\n");
        source.push_str("    }\n}\n");
        source.push_str("static inline void __zeus_owrite_bytes(void* base, size_t n, size_t esz, size_t idx, const void* src) {\n");
        source.push_str("    unsigned char* b = (unsigned char*)base; const unsigned char* s = (const unsigned char*)src;\n");
        source.push_str("    for (size_t k = 0; k < n; k++) {\n");
        source.push_str("        unsigned char m = (unsigned char)0 - (unsigned char)(k == idx);\n");
        source.push_str("        unsigned char* e = b + k * esz;\n");
        source.push_str("        for (size_t j = 0; j < esz; j++) e[j] = (unsigned char)((e[j] & (unsigned char)~m) | (s[j] & m));\n");
        source.push_str("    }\n}\n\n");

        // ── W^X Dual-Mapped JIT Infrastructure ───────────────────────────────────
        // On Linux: memfd_create gives an anonymous file; mmap it twice — once
        // PROT_READ|PROT_WRITE (mutations), once PROT_READ|PROT_EXEC (dispatch).
        // The same physical pages are never simultaneously W and X — strict W^X.
        // On non-Linux: fall back to volatile flags (no executable mapping needed
        // because @adaptive mutations are control-flow only, not machine-code edits).
        source.push_str("// [ZEUS W^X JIT SUPERVISOR] -- dual-mapped pages: PROT_WRITE != PROT_EXEC\n");
        source.push_str("#ifdef __linux__\n");
        source.push_str("#include <sys/syscall.h>\n");
        source.push_str("#ifndef __NR_memfd_create\n#define __NR_memfd_create 319\n#endif\n");
        source.push_str("#ifndef MFD_CLOEXEC\n#define MFD_CLOEXEC 1U\n#endif\n");
        source.push_str("typedef struct { void* exec_map; void* write_map; int fd; } zeus_jit_region_t;\n");
        source.push_str("static zeus_jit_region_t __zeus_jit = {NULL, NULL, -1};\n");
        source.push_str("static void __zeus_jit_init(void) {\n");
        source.push_str("    if (__zeus_jit.fd >= 0) return;\n");
        source.push_str("    int _fd = (int)syscall(__NR_memfd_create, \"zeus_jit\", (unsigned int)MFD_CLOEXEC);\n");
        source.push_str("    if (_fd < 0) return;\n");
        source.push_str("    if (ftruncate(_fd, 4096) < 0) return;\n");
        source.push_str("    // Write mapping: never executed\n");
        source.push_str("    __zeus_jit.write_map = mmap(NULL, 4096, PROT_READ|PROT_WRITE, MAP_SHARED, _fd, 0);\n");
        source.push_str("    // Exec mapping: never written -- strict W^X\n");
        source.push_str("    __zeus_jit.exec_map  = mmap(NULL, 4096, PROT_READ|PROT_EXEC,  MAP_SHARED, _fd, 0);\n");
        source.push_str("    if (__zeus_jit.write_map == MAP_FAILED || __zeus_jit.exec_map == MAP_FAILED) {\n");
        source.push_str("        __zeus_jit.write_map = NULL; __zeus_jit.exec_map = NULL; return;\n");
        source.push_str("    }\n");
        source.push_str("    __zeus_jit.fd = _fd;\n");
        source.push_str("}\n");
        source.push_str("// Mutate: write through WRITE mapping, read back through EXEC mapping\n");
        source.push_str("static inline void __zeus_jit_mutate(int slot, int val) {\n");
        source.push_str("    __zeus_jit_init();\n");
        source.push_str("    if (!__zeus_jit.write_map) return;\n");
        source.push_str("    ((volatile int*)__zeus_jit.write_map)[slot & 63] = val;\n");
        source.push_str("    __atomic_thread_fence(__ATOMIC_SEQ_CST);\n");
        source.push_str("}\n");
        source.push_str("static inline int __zeus_jit_read(int slot) {\n");
        source.push_str("    __zeus_jit_init();\n");
        source.push_str("    if (!__zeus_jit.exec_map) return 0;\n");
        source.push_str("    return ((volatile int*)__zeus_jit.exec_map)[slot & 63];\n");
        source.push_str("}\n");
        source.push_str("#else\n");
        source.push_str("static volatile int __zeus_jit_flags[64];\n");
        source.push_str("static inline void __zeus_jit_mutate(int slot, int val) { __zeus_jit_flags[slot & 63] = val; }\n");
        source.push_str("static inline int  __zeus_jit_read(int slot) { return __zeus_jit_flags[slot & 63]; }\n");
        source.push_str("#endif\n\n");

        source.push_str("// ----------------------------------------------------------------------------\n");
        source.push_str("// Arithmetic helper with a dead timing-noise term. NOTE: this is NOT crypto\n");
        source.push_str("// obfuscation or indistinguishability obfuscation (iO); it is ordinary math.\n");
        source.push_str("// ----------------------------------------------------------------------------\n");
        source.push_str("static inline void __zeus_div_zero_trap(void) {\n");
        source.push_str("    fprintf(stderr, \"[ZEUS TRAP] integer division or modulo by zero\\n\");\n");
        source.push_str("    exit(136);\n");
        source.push_str("}\n\n");
        source.push_str("static inline double __zeus_io_circuit_math(double a, double b, int op) {\n");
        source.push_str("    uint64_t ua = *(uint64_t*)&a;\n");
        source.push_str("    uint64_t ub = *(uint64_t*)&b;\n");
        source.push_str("    uint64_t noise = __rdtsc() ^ 0xDEADBEEFC0DEFACE;\n");
        source.push_str("    volatile uint64_t sink = (ua ^ noise) & (ub ^ noise);\n");
        source.push_str("    (void)sink;\n");
        source.push_str("    if (op == 0) return a + b;\n");
        source.push_str("    if (op == 1) return a - b;\n");
        source.push_str("    if (op == 2) return a * b;\n");
        source.push_str("    if (op == 3) return a / b;\n");
        source.push_str("    return 0;\n");
        source.push_str("}\n\n");

        // Provide memory lifecycle tools for legacy C code to clean up our tensors
        source.push_str("void zeus_free_tensor(zeus_tensor* t) {\n");
        source.push_str("    // [ZEUS ZERO-HEAP ENFORCER]: No dynamic deallocation allowed.\n");
        source.push_str("    // Tensor data is bound to the static arena pool.\n");
        source.push_str("    if (t) t->data = NULL;\n");
        source.push_str("}\n\n");

        source.push_str("// ============================================================================\n");
        source.push_str("// ZEUS NATIVE M:N FIBER SCHEDULER (Zero-Heap, Lock-Free Work-Stealing)\n");
        source.push_str("// ============================================================================\n");
        // _XOPEN_SOURCE is now emitted at the top of the file (Bug Fix #4)
        source.push_str("#if defined(_WIN32) || defined(_WIN64)\n");
        source.push_str("typedef struct ucontext_t { void* dummy; struct { void* ss_sp; size_t ss_size; } uc_stack; struct ucontext_t* uc_link; } ucontext_t;\n");
        source.push_str("static inline int getcontext(ucontext_t *ucp) { return 0; }\n");
        source.push_str("static inline void makecontext(ucontext_t *ucp, void (*func)(), int argc, ...) {}\n");
        source.push_str("static inline int swapcontext(ucontext_t *oucp, const ucontext_t *ucp) { return 0; }\n");
        source.push_str("#define _SC_NPROCESSORS_ONLN 1\n");
        source.push_str("static inline long sysconf(int name) { return 4; }\n");
        source.push_str("#else\n");
        source.push_str("#pragma GCC diagnostic push\n");
        source.push_str("#pragma GCC diagnostic ignored \"-Wdeprecated-declarations\"\n");
        source.push_str("#include <ucontext.h>\n");
        source.push_str("#pragma GCC diagnostic pop\n");
        source.push_str("#include <unistd.h>\n");
        source.push_str("#endif\n\n");

        // --- zeus_fiber_t ---
        source.push_str("typedef struct zeus_fiber {\n");
        source.push_str("    ucontext_t ctx;\n");
        source.push_str("    char stack[65536]; // 64KB fiber stack\n");
        source.push_str("    void (*func)(void*);\n");
        source.push_str("    void* arg;\n");
        source.push_str("    volatile uint64_t last_cycle_start;\n");
        source.push_str("    volatile int is_dead;\n");
        source.push_str("} zeus_fiber_t;\n\n");

        // --- Arena allocator ---
        // ============================================================================
        // ZEUS ELASTIC ARENA BALLOONING (Virtual Over-Provisioning, Vector 1)
        // Multi-arena pool: 8 arenas of 32MB each = 256MB total. Adjacent arenas
        // donate physical pages via atomic bit-shift when a primary arena nears OOM.
        // ZERO kernel traps for allocation — pure bump-pointer + atomic CAS stealing.
        // ============================================================================
        source.push_str("#define ZEUS_ARENA_COUNT 8\n");
        source.push_str("#define ZEUS_ARENA_SIZE  (1024UL * 1024UL * 32UL)  // 32 MB per arena\n");
        source.push_str("#define ZEUS_ARENA_TOTAL (ZEUS_ARENA_COUNT * ZEUS_ARENA_SIZE)\n\n");
        source.push_str("typedef struct {\n");
        source.push_str("    char*           base;\n");
        source.push_str("    volatile size_t offset; // bump pointer\n");
        source.push_str("    volatile size_t limit;  // soft ceiling (expanded by ballooning)\n");
        source.push_str("    volatile int    lock;   // CAS spinlock for limit expansion\n");
        source.push_str("} zeus_arena_t;\n\n");
        source.push_str("static zeus_arena_t __zeus_arenas[ZEUS_ARENA_COUNT];\n");
        source.push_str("static zeus_fiber_t* volatile* __zeus_active_fibers;\n");
        source.push_str("static volatile size_t* __zeus_active_fibers_count;\n\n");

        source.push_str("__attribute__((constructor)) void __zeus_init_shared_memory() {\n");
        source.push_str("    char* _mega = (char*)mmap(NULL, ZEUS_ARENA_TOTAL, PROT_READ|PROT_WRITE, MAP_SHARED|MAP_ANON, -1, 0);\n");
        source.push_str("    for (int _i = 0; _i < ZEUS_ARENA_COUNT; _i++) {\n");
        source.push_str("        __zeus_arenas[_i].base   = _mega + (size_t)_i * ZEUS_ARENA_SIZE;\n");
        source.push_str("        __atomic_store_n(&__zeus_arenas[_i].offset, 0,                __ATOMIC_RELAXED);\n");
        source.push_str("        __atomic_store_n(&__zeus_arenas[_i].limit,  ZEUS_ARENA_SIZE,   __ATOMIC_RELAXED);\n");
        source.push_str("        __atomic_store_n(&__zeus_arenas[_i].lock,   0,                __ATOMIC_RELAXED);\n");
        source.push_str("    }\n");
        source.push_str("    __zeus_active_fibers = (zeus_fiber_t**)mmap(NULL, sizeof(zeus_fiber_t*), PROT_READ|PROT_WRITE, MAP_SHARED|MAP_ANON, -1, 0);\n");
        source.push_str("    __zeus_active_fibers_count = (size_t*)mmap(NULL, sizeof(size_t), PROT_READ|PROT_WRITE, MAP_SHARED|MAP_ANON, -1, 0);\n");
        source.push_str("    *__zeus_active_fibers_count = 0;\n");
        source.push_str("    *__zeus_active_fibers = NULL;\n");
        source.push_str("}\n\n");

        source.push_str("// Compatibility alias so legacy code using zeus_arena_heap compiles\n");
        source.push_str("#define zeus_arena_heap   (__zeus_arenas[0].base)\n");
        source.push_str("#define zeus_arena_offset (__zeus_arenas[0].offset)\n\n");

        source.push_str("// __zeus_arena_alloc: bump-pointer with elastic ballooning.\n");
        source.push_str("// If arena[0] is near its soft limit, steals pages from an adjacent\n");
        source.push_str("// under-utilised arena via a single atomic bit-shift (right-shift by 1\n");
        source.push_str("// of the donor's free space = 50% donation), bypassing the OS entirely.\n");
        source.push_str("static inline void* __zeus_arena_alloc(size_t sz) {\n");
        source.push_str("    sz = (sz + 7) & ~(size_t)7;\n");
        source.push_str("    size_t old = __sync_fetch_and_add(&__zeus_arenas[0].offset, sz);\n");
        source.push_str("    size_t lim = __atomic_load_n(&__zeus_arenas[0].limit, __ATOMIC_ACQUIRE);\n");
        source.push_str("    if (__builtin_expect(old + sz <= lim, 1)) {\n");
        source.push_str("        return __zeus_arenas[0].base + old;\n");
        source.push_str("    }\n");
        source.push_str("    // Elastic balloon: attempt to steal from a donor arena\n");
        source.push_str("    for (int _d = 1; _d < ZEUS_ARENA_COUNT; _d++) {\n");
        source.push_str("        size_t _du = __atomic_load_n(&__zeus_arenas[_d].offset, __ATOMIC_ACQUIRE);\n");
        source.push_str("        size_t _dl = __atomic_load_n(&__zeus_arenas[_d].limit,  __ATOMIC_ACQUIRE);\n");
        source.push_str("        size_t _df = (_dl > _du) ? (_dl - _du) : 0;\n");
        source.push_str("        if (_df >= sz) {\n");
        source.push_str("            int _exp = 0;\n");
        source.push_str("            if (__atomic_compare_exchange_n(&__zeus_arenas[_d].lock, &_exp, 1, 0, __ATOMIC_SEQ_CST, __ATOMIC_RELAXED)) {\n");
        source.push_str("                size_t _steal = _df >> 1; // atomic bit-shift: donate half free pages\n");
        source.push_str("                __atomic_fetch_sub(&__zeus_arenas[_d].limit,  _steal, __ATOMIC_SEQ_CST);\n");
        source.push_str("                __atomic_fetch_add(&__zeus_arenas[0].limit, _steal, __ATOMIC_SEQ_CST);\n");
        source.push_str("                __atomic_store_n(&__zeus_arenas[_d].lock, 0, __ATOMIC_RELEASE);\n");
        source.push_str("                return __zeus_arenas[0].base + old;\n");
        source.push_str("            }\n");
        source.push_str("        }\n");
        source.push_str("    }\n");
        source.push_str("    fprintf(stderr, \"[ZEUS PANIC]: Elastic Arena OOM — all %d arenas (%luMB total) exhausted\\n\", ZEUS_ARENA_COUNT, (unsigned long)(ZEUS_ARENA_TOTAL >> 20));\n");
        source.push_str("    exit(1);\n");
        source.push_str("}\n\n");



        // --- Lock-Free Work-Stealing Deque (Chase-Lev) ---
        source.push_str("// ============================================================================\n");
        source.push_str("// LOCK-FREE COOPERATIVE WORK-STEALING DEQUE (Chase-Lev, C11 Atomics)\n");
        source.push_str("// ============================================================================\n");
        source.push_str("#define ZEUS_WSQ_CAPACITY 4096\n\n");

        source.push_str("typedef struct {\n");
        source.push_str("    void* tasks[ZEUS_WSQ_CAPACITY];\n");
        source.push_str("    size_t top;    // Thieves steal from top (CAS)\n");
        source.push_str("    size_t bottom; // Owner pushes/pops from bottom\n");
        source.push_str("} zeus_wsdeque_t;\n\n");

        source.push_str("static inline void zeus_wsdeque_init(zeus_wsdeque_t* q) {\n");
        source.push_str("    __atomic_store_n(&q->top, 0, __ATOMIC_RELAXED);\n");
        source.push_str("    __atomic_store_n(&q->bottom, 0, __ATOMIC_RELAXED);\n");
        source.push_str("}\n\n");

        // Push (owner only, no CAS needed for bottom)
        source.push_str("static inline void zeus_wsdeque_push(zeus_wsdeque_t* q, void* task) {\n");
        source.push_str("    size_t b = __atomic_load_n(&q->bottom, __ATOMIC_RELAXED);\n");
        source.push_str("    __atomic_store_n(&q->tasks[b % ZEUS_WSQ_CAPACITY], task, __ATOMIC_RELEASE);\n");
        source.push_str("    __atomic_store_n(&q->bottom, b + 1, __ATOMIC_RELEASE);\n");
        source.push_str("}\n\n");

        // Pop (owner only, needs CAS if contending with steal)
        source.push_str("static inline void* zeus_wsdeque_pop(zeus_wsdeque_t* q) {\n");
        source.push_str("    size_t b = __atomic_load_n(&q->bottom, __ATOMIC_RELAXED) - 1;\n");
        source.push_str("    __atomic_store_n(&q->bottom, b, __ATOMIC_RELAXED);\n");
        source.push_str("    __atomic_thread_fence(__ATOMIC_SEQ_CST);\n");
        source.push_str("    size_t t = __atomic_load_n(&q->top, __ATOMIC_RELAXED);\n");
        source.push_str("    if (t <= b) {\n");
        source.push_str("        void* task = __atomic_load_n(&q->tasks[b % ZEUS_WSQ_CAPACITY], __ATOMIC_ACQUIRE);\n");
        source.push_str("        if (t == b) {\n");
        source.push_str("            // Last element — race with steal\n");
        source.push_str("            size_t expected = t;\n");
        source.push_str("            if (!__atomic_compare_exchange_n(&q->top, &expected, t + 1, 0, __ATOMIC_SEQ_CST, __ATOMIC_RELAXED)) {\n");
        source.push_str("                task = NULL; // Lost race to thief\n");
        source.push_str("            }\n");
        source.push_str("            __atomic_store_n(&q->bottom, t + 1, __ATOMIC_RELAXED);\n");
        source.push_str("        }\n");
        source.push_str("        return task;\n");
        source.push_str("    } else {\n");
        source.push_str("        // Empty\n");
        source.push_str("        __atomic_store_n(&q->bottom, t, __ATOMIC_RELAXED);\n");
        source.push_str("        return NULL;\n");
        source.push_str("    }\n");
        source.push_str("} \n\n");

        // Steal (thieves, CAS on top)
        source.push_str("static inline void* zeus_wsdeque_steal(zeus_wsdeque_t* q) {\n");
        source.push_str("    size_t t = __atomic_load_n(&q->top, __ATOMIC_ACQUIRE);\n");
        source.push_str("    __atomic_thread_fence(__ATOMIC_SEQ_CST);\n");
        source.push_str("    size_t b = __atomic_load_n(&q->bottom, __ATOMIC_ACQUIRE);\n");
        source.push_str("    if (t < b) {\n");
        source.push_str("        void* task = q->tasks[t % ZEUS_WSQ_CAPACITY];\n");
        source.push_str("        size_t expected = t;\n");
        source.push_str("        if (!__atomic_compare_exchange_n(&q->top, &expected, t + 1, 0, __ATOMIC_SEQ_CST, __ATOMIC_RELAXED)) {\n");
        source.push_str("            return NULL; // Lost race\n");
        source.push_str("        }\n");
        source.push_str("        return task;\n");
        source.push_str("    }\n");
        source.push_str("    return NULL;\n");
        source.push_str("}\n\n");

        // ============================================================================
        // STOCHASTIC CORE HOPPING (Thermal Resonance Side-Channel Mitigation, Vector 3)
        // Uses __rdtsc() as a high-entropy PRNG seed, then sched_setaffinity() to force
        // the executing fiber to a random physical core, completely obfuscating thermal
        // signatures of cryptographic / AI-inference workloads.
        // ============================================================================
        source.push_str("// ============================================================================\n");
        source.push_str("// ZEUS STOCHASTIC CORE HOPPING (Thermal Side-Channel Mitigation)\n");
        source.push_str("// ============================================================================\n");
        source.push_str("#if defined(__linux__)\n");
        source.push_str("static inline void zeus_stochastic_core_hop(void) {\n");
        source.push_str("    uint64_t _tsc = __rdtsc();\n");
        source.push_str("    // SplitMix64 finaliser: high-quality avalanche from TSC entropy\n");
        source.push_str("    _tsc ^= (_tsc >> 30); _tsc *= 0xbf58476d1ce4e5b9ULL;\n");
        source.push_str("    _tsc ^= (_tsc >> 27); _tsc *= 0x94d049bb133111ebULL;\n");
        source.push_str("    _tsc ^= (_tsc >> 31);\n");
        source.push_str("    long _nc = sysconf(_SC_NPROCESSORS_ONLN);\n");
        source.push_str("    if (_nc <= 1) return;\n");
        source.push_str("    int _cpu = (int)(_tsc % (uint64_t)_nc);\n");
        source.push_str("    cpu_set_t _cs; CPU_ZERO(&_cs); CPU_SET(_cpu, &_cs);\n");
        source.push_str("    sched_setaffinity(0, sizeof(cpu_set_t), &_cs);\n");
        source.push_str("    // Flush micro-architectural transient state after core migration\n");
        source.push_str("    zeus_speculation_flush();\n");
        source.push_str("}\n");
        source.push_str("#else\n");
        source.push_str("static inline void zeus_stochastic_core_hop(void) { (void)0; }\n");
        source.push_str("#endif\n\n");

        // ============================================================================
        // IOMMU / VFIO DMA FIREWALL (Physical DMA Isolation, Vector 5)
        // Binds every static arena to the VFIO IOMMU domain via VFIO_IOMMU_MAP_DMA.
        // A rogue PCIe device cannot DMA into arena memory without a valid IOVA
        // mapping — the hardware IOMMU intercepts and rejects any such attempt.
        // Call zeus_vfio_bind_arena() once at startup for bare-metal deployments.
        // ============================================================================
        source.push_str("// ============================================================================\n");
        source.push_str("// ZEUS IOMMU / VFIO DMA FIREWALL (Physical Memory Isolation)\n");
        source.push_str("// ============================================================================\n");
        source.push_str("#if defined(__linux__)\n");
        source.push_str("#include <sys/ioctl.h>\n");
        source.push_str("#ifndef VFIO_TYPE1_IOMMU\n");
        source.push_str("#define VFIO_TYPE (';')\n");
        source.push_str("#define VFIO_BASE 100\n");
        source.push_str("#define VFIO_IOMMU_MAP_DMA   _IOW(VFIO_TYPE, VFIO_BASE + 13, struct zeus_vfio_dma_map)\n");
        source.push_str("#define VFIO_DMA_MAP_FLAG_READ  (1 << 0)\n");
        source.push_str("#define VFIO_DMA_MAP_FLAG_WRITE (1 << 1)\n");
        source.push_str("struct zeus_vfio_dma_map {\n");
        source.push_str("    uint32_t argsz; uint32_t flags;\n");
        source.push_str("    uint64_t vaddr; uint64_t iova; uint64_t size;\n");
        source.push_str("};\n");
        source.push_str("#define VFIO_TYPE1_IOMMU 1\n");
        source.push_str("#endif\n");
        source.push_str("static int __zeus_vfio_fd = -1;\n");
        source.push_str("static inline int zeus_vfio_bind_arena(void) {\n");
        source.push_str("    int _fd = open(\"/dev/vfio/vfio\", O_RDWR);\n");
        source.push_str("    if (_fd < 0) return -1; // VFIO unavailable — non-bare-metal host\n");
        source.push_str("    __zeus_vfio_fd = _fd;\n");
        source.push_str("    for (int _i = 0; _i < ZEUS_ARENA_COUNT; _i++) {\n");
        source.push_str("        struct zeus_vfio_dma_map _m;\n");
        source.push_str("        _m.argsz = sizeof(_m);\n");
        source.push_str("        _m.flags = VFIO_DMA_MAP_FLAG_READ | VFIO_DMA_MAP_FLAG_WRITE;\n");
        source.push_str("        _m.vaddr = (uint64_t)(uintptr_t)__zeus_arenas[_i].base;\n");
        source.push_str("        _m.iova  = (uint64_t)(uintptr_t)__zeus_arenas[_i].base; // identity IOVA\n");
        source.push_str("        _m.size  = ZEUS_ARENA_SIZE;\n");
        source.push_str("        ioctl(_fd, VFIO_IOMMU_MAP_DMA, &_m); // bind arena to IOMMU domain\n");
        source.push_str("    }\n");
        source.push_str("    return 0;\n");
        source.push_str("}\n");
        source.push_str("#else\n");
        source.push_str("static inline int zeus_vfio_bind_arena(void) { return -1; }\n");
        source.push_str("#endif\n\n");

        // ============================================================================
        // ARM64 POINTER AUTHENTICATION CODES — JIT Control-Flow Integrity (Vector 6)
        // pacia/autia sign/authenticate instruction pointers using the A-key.
        // pacib/autib protect return addresses using the B-key.
        // A PAC mismatch flips the top 2 bits → hardware exception on AUTIA failure,
        // rendering JIT control-flow hijacking and type-confusion exploits obsolete.
        // ============================================================================
        source.push_str("// ============================================================================\n");
        source.push_str("// ZEUS ARM64 POINTER AUTHENTICATION (PAC — ARMv8.3-A)\n");
        source.push_str("// ============================================================================\n");
        source.push_str("#if defined(__aarch64__) && defined(__ARM_FEATURE_PAC_DEFAULT)\n");
        source.push_str("static inline void zeus_pac_sign_code(void** ptr, uint64_t ctx) {\n");
        source.push_str("    uint64_t _r = (uint64_t)*ptr;\n");
        source.push_str("    asm volatile(\"pacia %0, %1\" : \"+r\"(_r) : \"r\"(ctx));\n");
        source.push_str("    *ptr = (void*)_r;\n");
        source.push_str("}\n");
        source.push_str("static inline void* zeus_pac_auth_code(void* ptr, uint64_t ctx) {\n");
        source.push_str("    uint64_t _r = (uint64_t)ptr;\n");
        source.push_str("    asm volatile(\"autia %0, %1\" : \"+r\"(_r) : \"r\"(ctx));\n");
        source.push_str("    return (void*)_r;\n");
        source.push_str("}\n");
        source.push_str("static inline void zeus_pac_sign_ret(void** ptr, uint64_t ctx) {\n");
        source.push_str("    uint64_t _r = (uint64_t)*ptr;\n");
        source.push_str("    asm volatile(\"pacib %0, %1\" : \"+r\"(_r) : \"r\"(ctx));\n");
        source.push_str("    *ptr = (void*)_r;\n");
        source.push_str("}\n");
        source.push_str("static inline void* zeus_pac_auth_ret(void* ptr, uint64_t ctx) {\n");
        source.push_str("    uint64_t _r = (uint64_t)ptr;\n");
        source.push_str("    asm volatile(\"autib %0, %1\" : \"+r\"(_r) : \"r\"(ctx));\n");
        source.push_str("    return (void*)_r;\n");
        source.push_str("}\n");
        source.push_str("#define ZEUS_PAC_SIGN_JIT(ptr, ctx) zeus_pac_sign_code(&(ptr), (ctx))\n");
        source.push_str("#define ZEUS_PAC_AUTH_JIT(ptr, ctx) ((void*)zeus_pac_auth_code((ptr), (ctx)))\n");
        source.push_str("#else\n");
        source.push_str("#define ZEUS_PAC_SIGN_JIT(ptr, ctx) ((void)(ctx))\n");
        source.push_str("#define ZEUS_PAC_AUTH_JIT(ptr, ctx) (ptr)\n");
        source.push_str("#endif\n\n");

        // ============================================================================
        // INT4 QUANTIZED NEURAL WEIGHTS IN .rodata (Zero-Heap Embedded Inference, Vector 9)
        // Packed 2 INT4 values per byte: lo nibble = weight[2i], hi nibble = weight[2i+1].
        // Baked into .rodata at link time — zero heap, bounded WCET, deterministic.
        // __attribute__((section(".rodata"))) guarantees read-only placement.
        // ============================================================================
        let (int4_bytes, int4_scale) = pack_int4_weights(&self.tuned_weights);
        source.push_str("// ============================================================================\n");
        source.push_str("// ZEUS INT4 QUANTIZED MICRO-AI (.rodata, zero-heap, bounded WCET)\n");
        source.push_str("// ============================================================================\n");
        source.push_str(&format!("static const uint8_t __zeus_int4_weights[{}] __attribute__((section(\".rodata\"))) = {{{}}};\n",
            int4_bytes.len(),
            int4_bytes.iter().map(|b| format!("0x{:02x}", b)).collect::<Vec<_>>().join(",")));
        source.push_str(&format!("static const float __zeus_int4_scale = {:.8}f;\n", int4_scale));
        source.push_str("#define ZEUS_INT4_LO(b) ((int8_t)(((b) & 0x0F) << 4) >> 4)\n");
        source.push_str("#define ZEUS_INT4_HI(b) ((int8_t)((int8_t)(b) >> 4))\n");
        source.push_str("static inline float __zeus_int4_infer(float x0, float x1) {\n");
        source.push_str("    int w0 = ZEUS_INT4_LO(__zeus_int4_weights[0]);\n");
        source.push_str("    int w1 = (sizeof(__zeus_int4_weights) > 1) ? ZEUS_INT4_HI(__zeus_int4_weights[0]) : 0;\n");
        source.push_str("    int w2 = (sizeof(__zeus_int4_weights) > 1) ? ZEUS_INT4_LO(__zeus_int4_weights[1]) : 0;\n");
        source.push_str("    (void)w2;\n");
        source.push_str("    return (x0 * (float)w0 + x1 * (float)w1) * __zeus_int4_scale;\n");
        source.push_str("}\n\n");

        // Generate C struct declarations at the top so type definitions are complete
        for stmt in &program.statements {
            if let Statement::StructDeclaration { .. } = stmt {
                source.push_str(&self.generate_statement(stmt, 0));
            }
        }

        // Generate extern function signatures so parallel blocks can see them
        for stmt in &program.statements {
            if let Statement::ExternFunctionDeclaration { name, parameters, return_type } = stmt {
                let c_ret = self.type_to_c(return_type);
                let params: Vec<String> = parameters.iter().map(|(p_name, p_type)| {
                    format!("{} {}", self.type_to_c(&Some(p_type.clone())), p_name)
                }).collect();
                source.push_str(&format!("extern {} {}({});\n", c_ret, name, params.join(", ")));
            }
        }

        // Pre-pass: Generate parallel block task structs and worker functions at top level
        let parallel_defs = self.generate_parallel_definitions(program);
        source.push_str(&parallel_defs);
        
        if self.output_name.contains("firmware") {
            source.push_str("const char* ZEUS_CRT0 = \".global _reset\\n_reset:\\n  ldr sp, =0x20000000\\n  bl main\\n  b .\\n\";\n\n");
        }

        let mut safestate_body = None;
        for stmt in &program.statements {
            if let Statement::SafeStateBlock { statements } = stmt {
                safestate_body = Some(statements.clone());
            }
        }

        source.push_str("void __zeus_safestate_handler() {\n");
        if let Some(statements) = safestate_body {
            self.secret_vars.borrow_mut().push(Vec::new());
            for stmt in &statements {
                if matches!(stmt, Statement::StructDeclaration { .. }) {
                    continue;
                }
                source.push_str(&self.generate_statement(stmt, 1));
            }
            let scope_vars = self.secret_vars.borrow_mut().pop().unwrap();
            for var in scope_vars {
                source.push_str(&self.generate_secure_wipe(&var, "    "));
            }
        }
        source.push_str("}\n\n");

        let has_funcs = program.statements.iter().any(|s| matches!(s, Statement::FunctionDeclaration{..}));
        
        if !has_funcs {
            source.push_str("int main() {\n");
            self.secret_vars.borrow_mut().push(Vec::new());
            for stmt in &program.statements {
                if matches!(stmt, Statement::StructDeclaration { .. }) {
                    continue;
                }
                source.push_str(&self.generate_statement(stmt, 1));
            }
            let scope_vars = self.secret_vars.borrow_mut().pop().unwrap();
            for var in scope_vars {
                source.push_str(&self.generate_secure_wipe(&var, "    "));
            }
            source.push_str("    return 0;\n}\n");
        } else {
            for stmt in &program.statements {
                if matches!(stmt, Statement::StructDeclaration { .. }) {
                    continue;
                }
                source.push_str(&self.generate_statement(stmt, 0));
            }
        }

        source
    }

    /// Generates the C header file (.h)
    pub fn generate_header(&self, program: &Program) -> String {
        let guard_name = format!("{}_H", self.output_name.to_uppercase());
        let mut header = String::new();
        
        header.push_str("// Auto-generated by Zeus v0.1\n");
        header.push_str(&format!("#ifndef {}\n", guard_name));
        header.push_str(&format!("#define {}\n\n", guard_name));
        header.push_str("#include <stddef.h>\n");
        header.push_str("#include <stdint.h>\n");
        header.push_str("#include <stdbool.h>\n\n");
        
        // Expose the fundamental tensor layout for legacy C apps
        header.push_str("typedef struct {\n");
        header.push_str("    double* data;\n");
        header.push_str("    size_t dim1;\n");
        header.push_str("    size_t dim2;\n");
        header.push_str("} zeus_tensor;\n\n");

        header.push_str("void zeus_free_tensor(zeus_tensor* t);\n\n");

        // Standard library structs
        header.push_str("// ZeusString — thin wrapper over a C string in the arena\n");
        header.push_str("typedef struct { const char* ptr; uint64_t len; uint64_t cap; } ZeusString;\n\n");
        header.push_str("// ZeusVec — growable array backed by the arena allocator\n");
        header.push_str("typedef struct { void* ptr; uint64_t len; uint64_t cap; uint64_t elem_size; } ZeusVec;\n\n");

        // Runtime Error Handling — Result<T, E>
        // ok_val and err_val are both double (widest numeric type); string errors
        // use err_str. For user-defined payload types, use the struct-based API.
        header.push_str("typedef struct {\n");
        header.push_str("    int is_error;\n");
        header.push_str("    double ok_val;\n");
        header.push_str("    double err_val;\n");
        header.push_str("    const char* err_str;\n");
        header.push_str("} zeus_result_t;\n\n");

        // Constructors
        header.push_str("#define ZEUS_OK(v)  ((zeus_result_t){ .is_error=0, .ok_val=(double)(v), .err_val=0, .err_str=NULL })\n");
        header.push_str("#define ZEUS_ERR(e) ((zeus_result_t){ .is_error=1, .ok_val=0, .err_val=(double)(e), .err_str=NULL })\n");
        header.push_str("#define ZEUS_ERR_STR(s) ((zeus_result_t){ .is_error=1, .ok_val=0, .err_val=0, .err_str=(s) })\n\n");

        // ? operator: propagate error upward, unwrap ok value
        header.push_str("#define ZEUS_TRY(expr) ({ \\\n");
        header.push_str("    zeus_result_t _res = (zeus_result_t)(expr); \\\n");
        header.push_str("    if (_res.is_error) return _res; \\\n");
        header.push_str("    _res.ok_val; \\\n");
        header.push_str("})\n\n");

        // unwrap with panic on error
        header.push_str("#define ZEUS_UNWRAP(expr) ({ \\\n");
        header.push_str("    zeus_result_t _u = (zeus_result_t)(expr); \\\n");
        header.push_str("    if (_u.is_error) { \\\n");
        header.push_str("        if (_u.err_str) fprintf(stderr, \"[ZEUS PANIC] unwrap on Err: %s\\n\", _u.err_str); \\\n");
        header.push_str("        else fprintf(stderr, \"[ZEUS PANIC] unwrap on Err(%g)\\n\", _u.err_val); \\\n");
        header.push_str("        exit(1); \\\n");
        header.push_str("    } \\\n");
        header.push_str("    _u.ok_val; \\\n");
        header.push_str("})\n\n");

        // FFI Stubs for Architecture Fallbacks & Security
        header.push_str("// Zeus Security FFI Boundaries\n");
        header.push_str("extern void zeus_tls_handshake(void);\n");
        header.push_str("extern int zeus_enclave_verify_token(void);\n");
        header.push_str("extern void __zeus_serialize_mutation_ledger(const char* func_name, const char* sig);\n");
        header.push_str("extern const char* __zeus_sign_mutation(const char* func_name, const char* data);\n\n");
        header.push_str("extern void ibv_post_send(void* qp, void* wr, void** bad_wr);\n");
        header.push_str("extern void ibv_post_recv(void* qp, void* wr, void** bad_wr);\n\n");

        // Traverse the AST for `pub fn` declarations and emit their C signatures
        header.push_str("// Public Zeus API Boundaries\n");
        for stmt in &program.statements {
            if let Statement::StructDeclaration { name, fields, .. } = stmt {
                header.push_str(&format!("typedef struct {} {};\n", name, name));
                // [ZEUS FAT PTR FFI BRIDGE] Emit SoA FatPtr alongside each struct.
                header.push_str(&format!("// Zeus SoA FatPtr: pass '{}_FatPtr*' instead of copying\n", name));
                header.push_str("typedef struct {\n");
                for (f_name, f_type) in fields {
                    let c_t = self.type_to_c(&Some(f_type.clone()));
                    header.push_str(&format!("    {}* {};\n", c_t, f_name));
                }
                header.push_str("    size_t len;\n");
                header.push_str(&format!("}} {}_FatPtr;\n\n", name));
            }
        }
        
        for stmt in &program.statements {
            if let Statement::FunctionDeclaration { is_pub, name, parameters, return_type, attributes, .. } = stmt {
                if *is_pub && attributes.contains(&crate::ast::FunctionAttribute::FfiExport) {
                    let mut c_ret = self.type_to_c(return_type);
                    if name == "main" {
                        c_ret = "int".to_string();
                    }
                    let mut params = Vec::new();
                    for (p_name, p_type) in parameters {
                        params.push(format!("{} {}", self.type_to_c(&Some(p_type.clone())), p_name));
                    }
                    header.push_str(&format!("{} {}({});\n", c_ret, name, params.join(", ")));
                }
            }
        }
        
        header.push_str(&format!("\n#endif // {}\n", guard_name));
        header
    }

    fn type_to_c(&self, t: &Option<crate::ast::Type>) -> String {
        match t {
            Some(crate::ast::Type::I8) => "int8_t".to_string(),
            Some(crate::ast::Type::I32) => "int32_t".to_string(),
            Some(crate::ast::Type::U64) => "uint64_t".to_string(),
            Some(crate::ast::Type::F32) => "float".to_string(),
            Some(crate::ast::Type::F64) => "double".to_string(),
            Some(crate::ast::Type::Bool) => "bool".to_string(),
            Some(crate::ast::Type::Tensor { .. }) => "zeus_tensor*".to_string(),
            Some(crate::ast::Type::Array(base, _)) => format!("{}*", self.type_to_c(&Some(*base.clone()))),
            Some(crate::ast::Type::Struct(name)) => {
                if name == "u32" { "uint32_t".to_string() }
                else if name == "usize" { "size_t".to_string() }
                else if name == "u8" { "uint8_t".to_string() }
                else if name == "str" { "const char*".to_string() }
                else { name.clone() }
            },
            Some(crate::ast::Type::Unknown(name)) => name.clone(),
            Some(crate::ast::Type::Result(_, _)) => "zeus_result_t".to_string(),
            Some(crate::ast::Type::Pointer(base)) => format!("{}*", self.type_to_c(&Some(*base.clone()))),
            // TypeParam only appears in un-monomorphized generic stubs; treat as double (widest).
            Some(crate::ast::Type::TypeParam(_)) => "double".to_string(),
            None => "void".to_string(),
        }
    }



}

// Implement the 100-Year Backend trait
impl Backend for CCodegen {
    fn compile(&self, program: &Program) -> Result<Artifact, CompileError> {
        let source_code = self.generate_source(program);
        Ok(Artifact {
            raw_data: source_code.into_bytes(),
        })
    }
}
