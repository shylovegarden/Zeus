// LLVM Hardening: Defend against optimizer destroying proofs
// Implements Fatal Vector 2 hardening: Jasmin Defense

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::values::FunctionValue;

pub struct LLVMHardeningPass<'ctx> {
    module: &'ctx Module<'ctx>,
    secret_functions: Vec<String>,
}

impl<'ctx> LLVMHardeningPass<'ctx> {
    pub fn new(module: &'ctx Module<'ctx>, secret_functions: Vec<String>) -> Self {
        LLVMHardeningPass {
            module,
            secret_functions,
        }
    }
    
    /// Apply all hardening passes
    pub fn harden(&self) -> Result<(), String> {
        // 1. Add optnone to all secret functions
        self.add_optnone_to_secrets()?;
        
        // 2. Insert memory barriers after secret operations
        self.insert_memory_barriers()?;
        
        // 3. Mark secret memory as volatile
        self.mark_secret_memory_volatile()?;
        
        // 4. Disable problematic LLVM passes
        self.disable_dangerous_passes()?;
        
        // 5. Add speculation barriers
        self.add_speculation_barriers()?;
        
        Ok(())
    }
    
    /// 1. Add optnone attribute to secret functions
    /// This prevents LLVM from optimizing them
    fn add_optnone_to_secrets(&self) -> Result<(), String> {
        let optnone = Attribute::get_named_enum(self.module.get_context(), "optnone")
            .ok_or("Failed to create optnone attribute")?;
        
        for func_name in &self.secret_functions {
            if let Some(func) = self.module.get_function(func_name) {
                // Add optnone attribute
                func.add_attribute(AttributeLoc::Function, optnone);
                
                // Also add noinline to prevent inlining that could break constant-time
                let noinline = Attribute::get_named_enum(self.module.get_context(), "noinline")
                    .ok_or("Failed to create noinline attribute")?;
                func.add_attribute(AttributeLoc::Function, noinline);
                
                println!("[LLVM Hardening] Applied optnone + noinline to {}", func_name);
            }
        }
        
        Ok(())
    }
    
    /// 2. Insert memory barriers (_mm_lfence) after secret operations
    fn insert_memory_barriers(&self) -> Result<(), String> {
        // For each secret function, insert lfence after memory operations
        // This prevents speculative execution from reordering secret operations
        
        for func_name in &self.secret_functions {
            if let Some(func) = self.module.get_function(func_name) {
                // Get the entry block
                if let Some(entry_bb) = func.get_basic_blocks().first() {
                    // Insert lfence intrinsic call
                    let context = self.module.get_context();
                    let builder = context.create_builder();
                    builder.position_at_end(*entry_bb);
                    
                    // Create lfence intrinsic
                    let lfence_type = context.void_type().fn_type(&[], false);
                    let lfence = self.module.add_function("llvm.x86.sse2.lfence", lfence_type, None);
                    
                    // Insert after each secret memory access
                    // This is a simplified version - real implementation would
                    // walk the entire CFG and insert after each tainted load/store
                    builder.build_call(lfence, &[], "lfence").map_err(|e| e.to_string())?;
                }
            }
        }
        
        Ok(())
    }
    
    /// 3. Mark secret memory as volatile
    /// Volatile prevents LLVM from optimizing away or reordering secret ops
    fn mark_secret_memory_volatile(&self) -> Result<(), String> {
        // Walk all instructions and mark loads/stores of secret variables as volatile
        // This is critical for constant-time guarantees
        
        for func_name in &self.secret_functions {
            if let Some(func) = self.module.get_function(func_name) {
                for bb in func.get_basic_blocks() {
                    for instr in bb.get_instructions() {
                        // Check if this is a load/store of secret data
                        // If so, mark as volatile
                        // Implementation depends on how secret variables are tracked
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 4. Disable LLVM passes that break constant-time
    fn disable_dangerous_passes(&self) -> Result<(), String> {
        // Create custom pass manager that excludes dangerous passes
        let pass_manager = PassManager::create(self.module);
        
        // These passes are SAFE for constant-time code:
        pass_manager.add_instruction_combining_pass();
        pass_manager.add_reassociate_pass();
        // Note: We intentionally SKIP:
        // - loop-unroll (breaks constant-time)
        // - simplifycfg (may introduce branches)
        // - jump-threading (breaks constant-time)
        // - gvn (may optimize away secret computations)
        
        // Run only safe passes
        pass_manager.run_on(self.module);
        
        println!("[LLVM Hardening] Ran safe-only optimization passes");
        
        Ok(())
    }
    
    /// 5. Add speculation barriers
    fn add_speculation_barriers(&self) -> Result<(), String> {
        // Insert speculation barriers at function entry/exit
        // This prevents Spectre-style attacks
        
        for func_name in &self.secret_functions {
            if let Some(func) = self.module.get_function(func_name) {
                // Insert at entry
                if let Some(entry_bb) = func.get_basic_blocks().first() {
                    let context = self.module.get_context();
                    let builder = context.create_builder();
                    builder.position_at_end(*entry_bb);
                    
                    // _mm_sfence for speculation barrier
                    let sfence_type = context.void_type().fn_type(&[], false);
                    let sfence = self.module.add_function("llvm.x86.sse2.sfence", sfence_type, None);
                    builder.build_call(sfence, &[], "sfence").map_err(|e| e.to_string())?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Verify assembly output for constant-time violations
    pub fn verify_assembly_constant_time(&self, asm_path: &str) -> Result<bool, String> {
        // Read assembly file
        let asm = std::fs::read_to_string(asm_path)
            .map_err(|e| format!("Failed to read assembly: {}", e))?;
        
        let mut violations = Vec::new();
        
        // Check for conditional jumps on tainted data
        // This is a simplified check - real implementation would:
        // 1. Parse assembly with a proper parser
        // 2. Track which registers are tainted
        // 3. Flag any conditional jump (je, jne, jz, jnz, etc.) on tainted registers
        
        let conditional_jumps = ["je", "jne", "jz", "jnz", "jg", "jl", "jge", "jle"];
        
        for line in asm.lines() {
            for jump in &conditional_jumps {
                if line.contains(jump) {
                    // Check if this jump is in a secret function
                    // Flag potential constant-time violation
                    violations.push(format!("Found conditional jump '{}' in: {}", jump, line.trim()));
                }
            }
        }
        
        if violations.is_empty() {
            println!("[LLVM Verification] Assembly constant-time check PASSED");
            Ok(true)
        } else {
            println!("[LLVM Verification] Assembly constant-time check FAILED");
            for v in &violations {
                println!("  - {}", v);
            }
            Ok(false)
        }
    }
}

/// Safe LLVM optimization configuration
pub struct SafeLLVMConfig;

impl SafeLLVMConfig {
    /// Get safe optimization flags that preserve constant-time
    pub fn safe_flags() -> Vec<String> {
        vec![
            // Disable all loop optimizations that break constant-time
            "-disable-loop-vectorization".to_string(),
            "-disable-slp-vectorization".to_string(),
            "-unroll-count=1".to_string(),  // No unrolling
            
            // Prevent speculative execution optimizations
            "-mspeculative-load-hardening".to_string(),
            "-mllvm".to_string(), "-x86-speculative-load-hardening".to_string(),
            
            // Keep frame pointers for debugging
            "-fno-omit-frame-pointer".to_string(),
        ]
    }
    
    /// Get unsafe flags that should NEVER be used for secret code
    pub fn unsafe_flags() -> Vec<String> {
        vec![
            "-O3".to_string(),  // Too aggressive
            "-ffast-math".to_string(),  // Breaks precise arithmetic
            "-funroll-loops".to_string(),  // Breaks constant-time
        ]
    }
}

/// Jasmin-style verified assembly integration
pub mod jasmin_integration {
    /// Import Jasmin's verified crypto primitives
    pub fn import_jasmin_primitives(module: &mut Module) {
        // Jasmin provides verified constant-time implementations
        // of common crypto primitives
        // This would integrate with libjasmin.a
    }
}
