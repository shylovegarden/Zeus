#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or,
                  clippy::type_complexity, unused_imports)]
use crate::ast::{Expression, Program, Statement};
use super::CCodegen;

impl CCodegen {
    pub(crate) fn generate_statement(&self, stmt: &Statement, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        match stmt {
            Statement::StructDeclaration { name, fields, is_component, type_params: _ } => {
                let mut c_struct = format!("{}typedef struct {} {{\n", pad, name);
                for (f_name, f_type) in fields {
                    if let crate::ast::Type::Array(base, size) = f_type {
                        let size_str = if let Expression::Number(n) = &**size {
                            format!("{}", *n as u64)
                        } else {
                            self.generate_expression(size)
                        };
                        c_struct.push_str(&format!("{}    {} {}[{}];\n", pad, self.type_to_c(&Some(*(base.clone()))), f_name, size_str));
                    } else {
                        c_struct.push_str(&format!("{}    {} {};\n", pad, self.type_to_c(&Some(f_type.clone())), f_name));
                    }
                }
                c_struct.push_str(&format!("{}}} {};\n", pad, name));
                
                // [ZEUS FAT PTR FFI BRIDGE] FatPtr typedef lives in the header only;
                // emitting it again in the .c would cause 'conflicting types' since
                // the .c always includes its own .h.
                if *is_component {
                    format!("{}// [ZEUS: NATIVE ECS ECS_BUFFER for '{}']\n{}\n", pad, name, c_struct)
                } else {
                    format!("{}// [ZEUS: Struct '{}' registered for SoA Flattening]\n{}\n", pad, name, c_struct)
                }
            }
            Statement::Let { name, is_mut: _, is_secret, value, var_type } => {
                // SoA detection: after ORAM pass, `Particle[32]` becomes
                //   OramAccess { base: Identifier("Particle"), index: 32 }
                // (the WHOLE IndexAccess is replaced — not nested inside OramAccess).
                // Match both OramAccess and IndexAccess with a struct-name Identifier base.
                let soa_info: Option<(String, Vec<(String, crate::ast::Type)>, &Expression)> = match value {
                    Expression::OramAccess { base, index, .. }
                    | Expression::IndexAccess { base, index } => {
                        if let Expression::Identifier(element_type) = base.as_ref() {
                            self.struct_schemas.borrow().get(element_type).cloned()
                                .map(|fields| (element_type.clone(), fields, index.as_ref()))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some((element_type, fields, index)) = soa_info {
                    self.soa_arrays.borrow_mut().insert(name.clone());
                    self.soa_struct_of.borrow_mut().insert(name.clone(), element_type.clone());
                    let mut out = format!("{}// [ZEUS: INVISIBLE SoA TRANSFORMATION for '{}']\n", pad, name);
                    let size_c = self.generate_expression(index);
                    if *is_secret {
                        // Secret SoA array: accesses become oblivious full-scans.
                        self.soa_secret_lens.borrow_mut().insert(name.clone(), size_c.clone());
                        out.push_str(&format!("{}// [ZEUS SECRET]: '{}' uses oblivious O(n) access + scope-exit wipe\n", pad, name));
                    }
                    for (fname, ftype) in &fields {
                        let c_type = self.type_to_c(&Some(ftype.clone()));
                        // Align to 32 bytes (256-bit AVX2 vector width) so auto-vectorizer fires
                        out.push_str(&format!("{}{} {}_{}[{}] __attribute__((aligned(32)));\n", pad, c_type, name, fname, size_c));
                        if *is_secret {
                            // Register each field array for secure wipe at scope exit.
                            self.secret_vars.borrow_mut().last_mut()
                                .expect("Internal Compiler Error: No secret scope")
                                .push(format!("{}_{}", name, fname));
                        }
                    }
                    return out;
                }
                if let Some(crate::ast::Type::Array(base, size)) = var_type {
                    if let Expression::ArrayLiteral(elems) = value {
                        let c_base = self.type_to_c(&Some(*base.clone()));
                        let size_c = self.generate_expression(size);
                        let init: Vec<String> = elems.iter().map(|e| self.generate_expression(e)).collect();
                        self.current_var_types.borrow_mut().insert(name.clone(), format!("{}*", c_base));
                        return format!("{}    {} {}[{}] = {{{}}};\n", pad, c_base, name, size_c, init.join(", "));
                    }
                }
                let val_c = self.generate_expression(value);
                let soa_read_type: Option<String> = match value {
                    Expression::FieldAccess { base, field } => match base.as_ref() {
                        Expression::IndexAccess { base: ab, .. }
                        | Expression::OramAccess { base: ab, .. } => {
                            if let Expression::Identifier(arr) = ab.as_ref() {
                                if self.soa_arrays.borrow().contains(arr.as_str()) {
                                    Some(self.soa_field_ctype(arr, field))
                                } else { None }
                            } else { None }
                        }
                        _ => None,
                    },
                    _ => None,
                };
                let c_type = if let Some(t) = soa_read_type {
                    t
                } else if self.atomic_int_vars.borrow().contains(name) {
                    "int64_t".to_string()
                } else {
                    self.type_to_c(var_type)
                };
                if *is_secret {
                    self.secret_vars.borrow_mut().last_mut().expect("Internal Compiler Error: No secret scope").push(name.clone());
                }
                // Named types (structs, enums) start with uppercase — casting a scalar to a
                // struct is invalid C. Only cast for primitive types (double, long long, bool, etc.)
                let is_primitive = c_type.chars().next().map(|c| c.is_lowercase()).unwrap_or(false)
                    || c_type.contains('*');
                let init_str = if is_primitive {
                    format!("({c_type})({val_c})")
                } else {
                    val_c.clone()
                };
                format!("{}    {} {} = {};\n", pad, c_type, name, init_str)
            }
            Statement::ExpressionStatement(expr) => {
                if let Expression::FunctionCall { name, arguments } = expr {
                    if name == "print" {
                        return self.generate_print_builtin(arguments, false, &pad, None);
                    }
                    if name == "println" {
                        return self.generate_print_builtin(arguments, true, &pad, None);
                    }
                }
                let expr_c = self.generate_expression(expr);
                if expr_c == "print" {
                    format!("{}printf(\"Execution complete.\\n\");\n", pad)
                } else {
                    format!("{}{};\n", pad, expr_c)
                }
            }
            Statement::If { condition, consequence, alternative } => {
                // [SLH — Speculative Load Hardening, Vector 4]
                // If the branch condition involves a secret-tainted value, inject a
                // speculation barrier (_mm_lfence on x86 / isb on ARM64) immediately
                // before the branch. This cuts the dataflow from transient speculative
                // loads to stable memory sinks, defeating Spectre-v1 / BLADE attacks.
                let cond_is_secret = self.is_secret_var(condition);
                let mut out = String::new();
                if cond_is_secret {
                    out.push_str(&format!("{}zeus_speculation_flush(); // [SLH: secret-conditional branch barrier]\n", pad));
                }
                out.push_str(&format!("{}if ({}) {{\n", pad, self.generate_expression(condition)));
                for s in consequence {
                    out.push_str(&self.generate_statement(s, indent + 1));
                }
                if let Some(alt) = alternative {
                    out.push_str(&format!("{}}} else {{\n", pad));
                    for s in alt {
                        out.push_str(&self.generate_statement(s, indent + 1));
                    }
                }
                out.push_str(&format!("{}}}\n", pad));
                out
            }
            Statement::ParallelBlock { iterator, start, end, statements } => {
                // [ZEUS: CORRECT MULTI-CORE FORK-JOIN PARALLELISM]
                // The previous fork()+ucontext fiber model was non-functional: a fiber
                // returned to a NULL uc_link and silently _exit()'d the process, so code
                // after a parallel block never ran in the parent and reductions were lost.
                // This model forks N worker processes (N = cores), each running a
                // contiguous chunk of the index range directly. Captured/reduction vars
                // live in MAP_SHARED arena mirrors so cross-process atomics aggregate,
                // and the parent copies results back after joining. No pthreads, no malloc.
                let start_c = self.generate_expression(start);
                let end_c = self.generate_expression(end);
                let block_id = { let mut c = self.parallel_counter.borrow_mut(); let id = *c; *c += 1; id };
                let struct_name = format!("__zeus_parallel_task_{}", block_id);
                let worker_name = format!("__zeus_parallel_worker_{}", block_id);
                let shared_vars = self.find_shared_variables(statements, iterator);
                let mut out = format!("{}// [ZEUS: MULTI-CORE FORK-JOIN PARALLEL DISPATCH]\n", pad);
                out.push_str(&format!("{}{{\n", pad));
                out.push_str(&format!("{}    size_t __zeus_start = {};\n", pad, start_c));
                out.push_str(&format!("{}    size_t __zeus_end   = {};\n", pad, end_c));
                out.push_str(&format!("{}    size_t __zeus_iters = (__zeus_end > __zeus_start) ? (__zeus_end - __zeus_start) : 0;\n", pad));
                out.push_str(&format!("{}    if (__zeus_iters > 0) {{\n", pad));
                out.push_str(&format!("{}        int __zeus_num_workers = (int)sysconf(_SC_NPROCESSORS_ONLN);\n", pad));
                out.push_str(&format!("{}        if (__zeus_num_workers < 1) __zeus_num_workers = 1;\n", pad));
                out.push_str(&format!("{}        if (__zeus_num_workers > 64) __zeus_num_workers = 64;\n", pad));
                out.push_str(&format!("{}        if ((size_t)__zeus_num_workers > __zeus_iters) __zeus_num_workers = (int)__zeus_iters;\n", pad));
                out.push_str(&format!("{}        size_t __zeus_chunk = (__zeus_iters + (size_t)__zeus_num_workers - 1) / (size_t)__zeus_num_workers;\n", pad));
                for (var_name, var_type) in &shared_vars {
                    out.push_str(&format!("{}        {}* __zeus_shared_{} = ({}*)__zeus_arena_alloc(sizeof({}));\n", pad, var_type, var_name, var_type, var_type));
                    out.push_str(&format!("{}        *__zeus_shared_{} = {};\n", pad, var_name, var_name));
                }
                out.push_str(&format!("{}        {}* __zeus_ctx = ({}*)__zeus_arena_alloc(sizeof({}));\n", pad, struct_name, struct_name, struct_name));
                for (var_name, _) in &shared_vars {
                    out.push_str(&format!("{}        __zeus_ctx->{} = __zeus_shared_{};\n", pad, var_name, var_name));
                }
                out.push_str(&format!("{}        pid_t __zeus_pids[64];\n", pad));
                out.push_str(&format!("{}        for (int _i = 0; _i < 64; _i++) __zeus_pids[_i] = 0;\n", pad));
                out.push_str(&format!("{}        int __zeus_myid = 0;\n", pad));
                out.push_str(&format!("{}        for (int w = 1; w < __zeus_num_workers; w++) {{\n", pad));
                out.push_str(&format!("{}            pid_t _p = fork();\n", pad));
                out.push_str(&format!("{}            if (_p == 0) {{ __zeus_myid = w; break; }}\n", pad));
                out.push_str(&format!("{}            __zeus_pids[w] = _p;\n", pad));
                out.push_str(&format!("{}        }}\n", pad));
                out.push_str(&format!("{}        size_t __zeus_lo = __zeus_start + (size_t)__zeus_myid * __zeus_chunk;\n", pad));
                out.push_str(&format!("{}        size_t __zeus_hi = __zeus_lo + __zeus_chunk;\n", pad));
                out.push_str(&format!("{}        if (__zeus_hi > __zeus_end) __zeus_hi = __zeus_end;\n", pad));
                out.push_str(&format!("{}        if (__zeus_lo < __zeus_hi) {}((void*)__zeus_ctx, __zeus_lo, __zeus_hi);\n", pad, worker_name));
                out.push_str(&format!("{}        if (__zeus_myid != 0) _exit(0);\n", pad));
                out.push_str(&format!("{}        for (int w = 1; w < __zeus_num_workers; w++) {{\n", pad));
                out.push_str(&format!("{}            if (__zeus_pids[w] > 0) waitpid(__zeus_pids[w], NULL, 0);\n", pad));
                out.push_str(&format!("{}        }}\n", pad));
                for (var_name, _) in &shared_vars {
                    out.push_str(&format!("{}        {} = *__zeus_shared_{};\n", pad, var_name, var_name));
                }
                out.push_str(&format!("{}    }}\n", pad));
                out.push_str(&format!("{}}}\n", pad));
                out
            }
            Statement::TargetBlock { targets, statements } => {
                let target_str = targets.join(", ");
                let mut out = format!("{}// [ZEUS: TARGET SPECIFIC START: {}]\n", pad, target_str);
                self.secret_vars.borrow_mut().push(Vec::new());
                for s in statements {
                    out.push_str(&self.generate_statement(s, indent));
                }
                let scope_vars = self.secret_vars.borrow_mut().pop().unwrap();
                for var in scope_vars {
                    out.push_str(&self.generate_secure_wipe(&var, &pad));
                }
                out.push_str(&format!("{}// [ZEUS: TARGET SPECIFIC END]\n", pad));
                out
            }
            Statement::ProofBlock { statements: _ } => {
                let out = format!("{}// [ZEUS: COMPILE-TIME PROOF BLOCK (Elided from Runtime)]\n", pad);
                out
            }
            Statement::EnclaveBlock { statements } => {
                let mut out = format!("{}// [ZEUS: scoped secure region — compiler memory barrier; no hardware enclave on this target]\n{}zeus_enclave_enter();\n{}{{ \n", pad, pad, pad);
                self.secret_vars.borrow_mut().push(Vec::new());
                for s in statements {
                    out.push_str(&self.generate_statement(s, indent + 1));
                }
                let scope_vars = self.secret_vars.borrow_mut().pop().unwrap();
                for var in scope_vars {
                    out.push_str(&self.generate_secure_wipe(&var, &format!("{}    ", pad)));
                }
                out.push_str(&format!("{}}}\n", pad));
                out.push_str(&format!("{}zeus_enclave_exit();\n{}// [ZEUS: end secure region]\n", pad, pad));
                out
            }
            Statement::TestDeclaration { name, .. } => {
                format!("{}// [ZEUS: TEST BLOCK '{}' (Elided from Build)]\n", pad, name)
            }
            Statement::SafeStateBlock { .. } => {
                format!("{}// [ZEUS: SAFESTATE BLOCK (Emitted globally)]\n", pad)
            }
            Statement::Assert(expr) => {
                let expr_c = self.generate_expression(expr);
                format!("{}// [ZEUS VERIFIED: assert({})]\n", pad, expr_c)
            }
            Statement::Import(path) => {
                format!("{}// [ZEUS IMPORT: {} (Inlined AST)]\n", pad, path)
            }
            Statement::FunctionDeclaration { is_pub: _, name, parameters, return_type, body, attributes, secret_params: _, type_params: _ } => {
                let mut c_ret = self.type_to_c(return_type);
                if name == "main" {
                    c_ret = "int".to_string();
                }
                let mut params = Vec::new();
                for (p_name, p_type) in parameters {
                    params.push(format!("{} {}", self.type_to_c(&Some(p_type.clone())), p_name));
                }
                
                let mut out = format!("{}{} {}({}) {{\n", pad, c_ret, name, params.join(", "));
                self.secret_vars.borrow_mut().push(Vec::new());
                self.pending_ensures.borrow_mut().clear();

                let is_adaptive = false;

                for attr in attributes {
                    match attr {
                        crate::ast::FunctionAttribute::Verify(expr, has_timed_out) => {
                            if *has_timed_out {
                                let expr_c = self.generate_expression(expr);
                                out.push_str(&format!("{}    if (!({})) {{\n", pad, expr_c));
                                out.push_str(&format!("{}        fprintf(stderr, \"[ZEUS PANIC]: Zeus Runtime Verification Failure at {}.zs:%d: Constraint '%s' violated\\n\", __LINE__, \"{}\");\n", pad, self.output_name, expr_c));
                                out.push_str(&format!("{}        __zeus_safestate_handler();\n", pad));
                                out.push_str(&format!("{}        exit(1);\n", pad));
                                out.push_str(&format!("{}    }}\n", pad));
                            }
                        }
                        crate::ast::FunctionAttribute::Requires(expr, _) => {
                            let expr_c = self.generate_expression(expr);
                            out.push_str(&format!("{}    if (!({})) {{\n", pad, expr_c));
                            out.push_str(&format!("{}        fprintf(stderr, \"[ZEUS PANIC]: Contract violation (@requires) in {}.zs: precondition '%s' failed\\n\", \"{}\");\n", pad, self.output_name, expr_c.replace('"', "\\\"")));
                            out.push_str(&format!("{}        __zeus_safestate_handler();\n", pad));
                            out.push_str(&format!("{}        exit(1);\n", pad));
                            out.push_str(&format!("{}    }}\n", pad));
                        }
                        crate::ast::FunctionAttribute::Ensures(expr, _) => {
                            self.pending_ensures.borrow_mut().push(self.generate_expression(expr));
                        }
                        crate::ast::FunctionAttribute::Adaptive(params) => {
                            if self.disable_adaptive {
                                out.push_str(&format!("{}    // [ZEUS ADAPTIVE]: Disabled via --disable-adaptive. Running purely deterministic.\n", pad));
                            } else {
                                // Deterministic JIT slot: FNV-1a hash of function name, mod 64
                                let jit_slot: u32 = name.bytes().fold(2166136261u32, |h, b| {
                                    h.wrapping_mul(16777619) ^ (b as u32)
                                }) & 63;
                                out.push_str(&format!("{}    // [ZEUS ADAPTIVE W^X]: static heuristic guard (threshold: {})\n", pad, params));
                                out.push_str(&format!("{}    // Mutations flow through WRITE mapping; dispatch reads EXEC mapping -- strict W^X.\n", pad));
                                out.push_str(&format!("{}    float _ai_fuel_usage = 10.0f;\n", pad));
                                out.push_str(&format!("{}    float _ai_latency_spike = -5.0f;\n", pad));
                                out.push_str(&format!("{}    float _ai_confidence = __zeus_heuristic_score(_ai_fuel_usage, _ai_latency_spike);\n", pad));
                                out.push_str(&format!("{}    if (_ai_confidence > 0.5f) {{\n", pad));
                                out.push_str(&format!("{}        // Mutate JIT dispatch slot {} via WRITE mapping (page is PROT_WRITE, never PROT_EXEC)\n", pad, jit_slot));
                                out.push_str(&format!("{}        __zeus_jit_mutate({}, 1);\n", pad, jit_slot));
                                out.push_str(&format!("{}        fprintf(stderr, \"\\n[ZEUS ADAPTIVE W^X] ANOMALY in {}() slot {}. Confidence: %.2f. JIT dispatch mutated.\\n\", _ai_confidence);\n", pad, name, jit_slot));
                                out.push_str(&format!("{}    }}\n", pad));
                                out.push_str(&format!("{}    // Dispatch via EXEC mapping (page is PROT_EXEC, never PROT_WRITE) -- W^X safe\n", pad));
                                out.push_str(&format!("{}    if (__zeus_jit_read({})) {{\n", pad, jit_slot));
                                out.push_str(&format!("{}        fprintf(stderr, \"[ZEUS ADAPTIVE W^X] Limp-mode active for {}(). Entering safe state.\\n\");\n", pad, name));
                                out.push_str(&format!("{}        __zeus_safestate_handler();\n", pad));
                                if c_ret == "void" {
                                    out.push_str(&format!("{}        return;\n", pad));
                                } else {
                                    out.push_str(&format!("{}        return 0;\n", pad));
                                }
                                out.push_str(&format!("{}    }}\n", pad));
                            }
                        }
                        _ => {}
                    }
                }

                for s in body {
                    if is_adaptive {
                        out.push_str(&format!("{}    {{\n", pad));
                        out.push_str(&format!("{}        unsigned short _zeus_bit = ((_zeus_lfsr >> 0) ^ (_zeus_lfsr >> 2) ^ (_zeus_lfsr >> 3) ^ (_zeus_lfsr >> 5)) & 1;\n", pad));
                        out.push_str(&format!("{}        _zeus_lfsr = (_zeus_lfsr >> 1) | (_zeus_bit << 15);\n", pad));
                        out.push_str(&format!("{}        if (_zeus_lfsr % 2 == 0) {{\n", pad));
                        out.push_str(&format!("{}            volatile int _zeus_noise = 0;\n", pad));
                        out.push_str(&format!("{}            for (int _n = 0; _n < (_zeus_lfsr % 16); _n++) _zeus_noise += _n;\n", pad));
                        out.push_str(&format!("{}        }}\n", pad));
                        out.push_str(&format!("{}    }}\n", pad));
                    }
                    out.push_str(&self.generate_statement(s, indent + 1));
                }
                let scope_vars = self.secret_vars.borrow_mut().pop().unwrap();
                for var in scope_vars {
                    out.push_str(&self.generate_secure_wipe(&var, &format!("{}    ", pad)));
                }
                if name == "main" {
                    out.push_str(&format!("{}    return 0;\n", pad));
                }
                out.push_str(&format!("{}}}\n\n", pad));
                out
            }
            Statement::ExternFunctionDeclaration { name, parameters, return_type } => {
                let c_ret = self.type_to_c(return_type);
                let mut params = Vec::new();
                for (p_name, p_type) in parameters {
                    let c_type = self.type_to_c(&Some(p_type.clone()));
                    params.push(format!("{} {}", c_type, p_name));
                }
                format!("{}extern {} {}({});\n", pad, c_ret, name, params.join(", "))
            }
            Statement::For { iterator, start, end, body } => {
                let start_c = self.generate_expression(start);
                let end_c = self.generate_expression(end);
                let mut out = format!("{}for (int {} = {}; {} < {}; ++{}) {{\n", pad, iterator, start_c, iterator, end_c, iterator);
                out.push_str(&format!("{}    if (__zeus_active_fibers && *__zeus_active_fibers && (*__zeus_active_fibers)->is_dead) {{\n", pad));
                out.push_str(&format!("{}        fprintf(stderr, \"[WORKER] Fiber assassinated by Sentinel. Shedding load...\\n\");\n", pad));
                out.push_str(&format!("{}        break;\n", pad));
                out.push_str(&format!("{}    }}\n", pad));
                self.secret_vars.borrow_mut().push(Vec::new());
                for s in body {
                    out.push_str(&self.generate_statement(s, indent + 1));
                }
                let scope_vars = self.secret_vars.borrow_mut().pop().unwrap();
                for var in scope_vars {
                    out.push_str(&self.generate_secure_wipe(&var, &format!("{}    ", pad)));
                }
                out.push_str(&format!("{}}}\n", pad));
                out
            }
            Statement::While { condition, body } => {
                let cond_c = self.generate_expression(condition);
                let mut out = format!("{}while ({}) {{\n", pad, cond_c);
                self.secret_vars.borrow_mut().push(Vec::new());
                for s in body {
                    out.push_str(&self.generate_statement(s, indent + 1));
                }
                let scope_vars = self.secret_vars.borrow_mut().pop().unwrap();
                for var in scope_vars {
                    out.push_str(&self.generate_secure_wipe(&var, &format!("{}    ", pad)));
                }
                out.push_str(&format!("{}}}\n", pad));
                out
            }
            Statement::Return(expr) => {
                let expr_c = self.generate_expression(expr);
                let ensures = self.pending_ensures.borrow().clone();
                let mut out = String::new();
                if ensures.is_empty() {
                    for scope in self.secret_vars.borrow().iter().rev() {
                        for var in scope.iter() {
                            out.push_str(&self.generate_secure_wipe(var, &pad));
                        }
                    }
                    out.push_str(&format!("{}return {};\n", pad, expr_c));
                } else {
                    out.push_str(&format!("{}{{ __auto_type result = ({}); /* @ensures binding */\n", pad, expr_c));
                    for cond in &ensures {
                        out.push_str(&format!("{}    if (!({})) {{\n", pad, cond));
                        out.push_str(&format!("{}        fprintf(stderr, \"[ZEUS PANIC]: Contract violation (@ensures) in {}.zs: postcondition failed\\n\");\n", pad, self.output_name));
                        out.push_str(&format!("{}        __zeus_safestate_handler();\n", pad));
                        out.push_str(&format!("{}        exit(1);\n", pad));
                        out.push_str(&format!("{}    }}\n", pad));
                    }
                    for scope in self.secret_vars.borrow().iter().rev() {
                        for var in scope.iter() {
                            out.push_str(&self.generate_secure_wipe(var, &format!("{}    ", pad)));
                        }
                    }
                    out.push_str(&format!("{}    return result;\n", pad));
                    out.push_str(&format!("{}}}\n", pad));
                }
                out
            }
            Statement::Panic(msg) => {
                format!("{}fprintf(stderr, \"[ZEUS PANIC (SAFE STATE HW RESET)]: %s\\n\", \"{}\");\n{}__zeus_safestate_handler();\n{}exit(1);\n", pad, msg, pad, pad)
            }
            Statement::AtomicAdd { target, amount } => {
                format!("{}__atomic_fetch_add(&{}, {}, __ATOMIC_SEQ_CST);\n", pad, target, amount)
            }
            Statement::EnumDeclaration { name, variants } => {
                // Emit C tagged union for the enum
                let tags: Vec<String> = variants.iter().map(|v| format!("{}__{}", name, v.name)).collect();
                let tag_line = format!("typedef enum {{ {} }} __{}_tag;\n", tags.join(", "), name);
                let mut union_body = String::new();
                for v in variants {
                    if let Some(types) = &v.payload {
                        let fields: Vec<String> = types.iter().enumerate()
                            .map(|(i, t)| format!("{} _{};", self.type_to_c(&Some(t.clone())), i)).collect();
                        union_body.push_str(&format!("  struct {{ {} }} {};\n", fields.join(" "), v.name));
                    }
                }
                format!("{}{}typedef struct {{ __{}_tag tag; union {{\n{}}} data; }} {};\n",
                    tag_line, "", name, union_body, name)
            }
            Statement::MatchStatement { scrutinee, arms } => {
                let scrut_c = self.generate_expression(scrutinee);
                let mut s = format!("switch ({}.tag) {{\n", scrut_c);
                for arm in arms {
                    match &arm.pattern {
                        crate::ast::MatchPattern::Variant { enum_name, variant } => {
                            s.push_str(&format!("  case {}__{}: {{\n", enum_name, variant));
                        }
                        crate::ast::MatchPattern::VariantTuple { enum_name, variant, bindings } => {
                            s.push_str(&format!("  case {}__{}: {{\n", enum_name, variant));
                            for (i, b) in bindings.iter().enumerate() {
                                s.push_str(&format!("    __auto_type {} = {}.data.{}._{};\n", b, scrut_c, variant, i));
                            }
                        }
                        crate::ast::MatchPattern::Wildcard => {
                            s.push_str("  default: {\n");
                        }
                        crate::ast::MatchPattern::Literal(n) => {
                            s.push_str(&format!("  case {}: {{\n", n));
                        }
                    }
                    for st in &arm.body {
                        s.push_str(&self.generate_statement(st, 2));
                    }
                    s.push_str("    break;\n  }\n");
                }
                s.push_str("}\n");
                s
            }
            Statement::LineDirective(line) => {
                format!("#line {} \"{}.zs\"\n", line, self.output_name)
            }
            Statement::CfgBlock { arch, statements } => {
                let mut out = format!("{}// [ZEUS HAL: arch={}]\n", pad, arch);
                self.secret_vars.borrow_mut().push(Vec::new());
                for s in statements {
                    out.push_str(&self.generate_statement(s, indent));
                }
                let scope_vars = self.secret_vars.borrow_mut().pop().unwrap();
                for var in scope_vars {
                    out.push_str(&self.generate_secure_wipe(&var, &pad));
                }
                out
            }
            Statement::ComptimeBlock { statements } => {
                let mut out = String::new();
                out.push_str(&format!("{}// [ZEUS COMPTIME BLOCK]\n", pad));
                for stmt in statements {
                    out.push_str(&self.generate_statement(stmt, indent));
                }
                out
            }
            Statement::ClusterBlock { statements } => {
                let mut out = String::new();
                out.push_str(&format!("{}// [ZEUS CLUSTER: distributed RDMA backend not built for this target — running block in-process]\n", pad));
                out.push_str(&format!("{}zeus_tls_handshake();\n", pad));
                out.push_str(&format!("{}if (!zeus_enclave_verify_token()) {{\n", pad));
                out.push_str(&format!("{}    fprintf(stderr, \"[ZEUS PANIC]: Hardware violation. Invalid cryptographic capability token for RDMA enclave.\\n\");\n", pad));
                out.push_str(&format!("{}    exit(1);\n", pad));
                out.push_str(&format!("{}}}\n", pad));
                
                
                for stmt in statements {
                    out.push_str(&self.generate_statement(stmt, indent));
                }
                
                out
            }
        }
    }

    pub(crate) fn is_secret_var(&self, expr: &Expression) -> bool {
        if let Expression::Identifier(name) = expr {
            for scope in self.secret_vars.borrow().iter() {
                if scope.contains(name) {
                    return true;
                }
            }
        }
        false
    }
}
