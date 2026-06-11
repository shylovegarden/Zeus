# Week 2: Integration Tasks - Critical Flaw Fixes

**Goal:** Integrate the 3 critical flaw modules into the compiler pipeline
**Duration:** Week 2 of Phase 1 (4 weeks total)
**Status:** Ready to begin

---

## OVERVIEW

Week 1 delivered 3 standalone modules:
1. ✅ `binary_verifier/` - Binary-level verification
2. ✅ `type_checker_strict.rs` - Sound type system
3. ✅ `honest_verification.rs` - Honest timeout reporting

**Week 2 Mission:** Wire these into the actual compiler so they RUN during builds

---

## TASK 1: Binary Verifier Integration (Days 1-2)

### Objective
Run binary verification after compilation, before certificate signing

### Steps

1. **Add to build pipeline** (modify `main.rs`)
   ```rust
   // After C compilation, before certificate generation
   fn build_project(...) {
       // ... existing compilation code ...
       
       // NEW: Binary verification
       let binary_path = format!("{}", base_name);
       let mut verifier = binary_verifier::BinaryVerifier::new();
       
       match verifier.verify_constant_time(Path::new(&binary_path)) {
           BinaryVerificationResult::ConstantTime => {
               // Continue to certificate
           }
           BinaryVerificationResult::TimingLeaks(leaks) => {
               eprintln!("❌ Binary has timing leaks:");
               for leak in leaks {
                   eprintln!("  {:?}", leak);
               }
               std::process::exit(1);
           }
           BinaryVerificationResult::Failed(e) => {
               eprintln!("⚠️  Binary verification failed: {}", e);
               // Don't sign certificate
           }
       }
       
       // Generate certificate ONLY if binary verified
       write_certificate(..., binary_verified: true);
   }
   ```

2. **Update certificate generation** (`cert_sign.rs`)
   - Add `binary_verified: bool` field
   - Only sign if `binary_verified == true`

3. **Install Capstone** (dependency)
   ```toml
   # Cargo.toml
   [dependencies]
   capstone = "0.11"
   ```

4. **Implement disassembly** (enhance `binary_verifier/mod.rs`)
   - Use Capstone to disassemble x86_64
   - Parse instructions
   - Check for conditional jumps

### Acceptance Criteria
- [ ] Binary verification runs after `clang` compilation
- [ ] `binary_verified` field in certificate
- [ ] No signature if binary has timing leaks
- [ ] Test: Compile program with secret branch, verify detection

---

## TASK 2: Strict Type Checker Integration (Days 3-4)

### Objective
Replace existing type checker with strict version

### Steps

1. **Find existing type checks** (search codebase)
   ```bash
   grep -r "check_type\|type_check\|Type::" zeus_compiler/src/ --include="*.rs"
   ```

2. **Replace in analyzer** (`analyzer.rs`)
   ```rust
   // OLD: Existing type check
   // if target_type != value_type { ... }
   
   // NEW: Strict type check
   use crate::type_checker_strict::StrictTypeChecker;
   
   let checker = StrictTypeChecker::new();
   if let Err(e) = checker.check_assignment(&target_type, &value_type) {
       return Err(format!("Type error: {:?}", e));
   }
   ```

3. **Update error messages**
   - Replace generic type errors with specific width mismatch errors
   - Add helpful suggestions

4. **Fix existing violations**
   - Find all code that violates strict rules
   - Add explicit casts where needed
   - Update tests

5. **Add literal overflow checks**
   ```rust
   // In expression evaluation
   if let Expression::Number(n) = expr {
       if let Err(e) = checker.check_literal_fit(n, &target_type) {
           return Err(format!("Overflow: {:?}", e));
       }
   }
   ```

### Acceptance Criteria
- [ ] `u64` → `u32` assignment fails with clear error
- [ ] `let x: u32 = 10000000000` fails at compile time
- [ ] All existing tests updated
- [ ] New strict type tests pass

---

## TASK 3: Honest Verification Integration (Days 5-6)

### Objective
Replace formal_verifier with honest timeout reporting

### Steps

1. **Find verification calls** (search codebase)
   ```bash
   grep -r "FormalVerifier\|formal_verifier\|verify(" zeus_compiler/src/ --include="*.rs"
   ```

2. **Replace in build pipeline** (`main.rs`)
   ```rust
   // OLD: Existing verification
   // let mut verifier = FormalVerifier::new();
   // verifier.verify(&program, false)?;
   
   // NEW: Honest verification
   use crate::honest_verification::{HonestVerifier, HonestVerificationResult};
   
   let verifier = HonestVerifier::new(2000); // 2000ms timeout
   
   // Verify each function
   for func in &program.functions {
       match verifier.verify(&func.to_string()) {
           HonestVerificationResult::Verified { .. } => {
               // Continue
           }
           HonestVerificationResult::Timeout { attempted_ms } => {
               eprintln!("⚠️  Proof timeout for {} ({}ms)", func.name, attempted_ms);
               eprintln!("   Security properties NOT verified");
               // Continue but DON'T sign certificate
               certificate.should_sign = false;
           }
           HonestVerificationResult::Failed { reason } => {
               return Err(format!("Verification failed: {}", reason));
           }
       }
   }
   ```

3. **Update certificate** (`cert_sign.rs`)
   - Use `HonestCertificate` structure
   - Remove Ed25519 signature on timeout
   - Add `verified: false` field

4. **Update CLI output** (`main.rs`)
   ```rust
   // After verification
   certificate.print_report();
   
   if !certificate.verified {
       eprintln!("\n⚠️  Build completed but NOT verified");
       eprintln!("   Review warnings above");
   }
   ```

5. **Test timeout handling**
   ```rust
   // Create function that causes Z3 timeout
   fn complex_proof() {
       // Many nested conditions
       // Should trigger timeout
   }
   
   // Verify output says "TIMEOUT" not "VERIFIED"
   ```

### Acceptance Criteria
- [ ] Timeout clearly reported (not hidden)
- [ ] Certificate shows `verified: false` on timeout
- [ ] No Ed25519 signature on unverified certificates
- [ ] CLI exit code non-zero on timeout
- [ ] Test: Complex function triggers honest timeout report

---

## TASK 4: Integration Testing (Day 7)

### Objective
Verify all 3 fixes work together end-to-end

### Test Cases

1. **Binary Verification Test**
   ```zeus
   // test_binary_verify.zs
   @constant_time
   pub fn main() {
       let secret x: u64 = 100;
       if x > 50 {  // Secret branch
           println("big");
       }
   }
   ```
   - Compile with `-O3`
   - Should detect timing leak in binary
   - Certificate should NOT be signed

2. **Type System Test**
   ```zeus
   // test_type_strict.zs
   pub fn main() {
       let x: u64 = 1000;
       let y: u32 = x;  // Should ERROR
   }
   ```
   - Should fail with "Width mismatch"
   - Should not compile

3. **Timeout Honesty Test**
   ```zeus
   // test_timeout.zs
   pub fn complex() {
       // 100 nested conditions
       // Should timeout
   }
   ```
   - Should print "TIMEOUT - NOT VERIFIED"
   - Should NOT print "VERIFIED"
   - Certificate should NOT be signed

4. **Success Path Test**
   ```zeus
   // test_success.zs
   @constant_time
   @zero_heap
   pub fn main() {
       let x: u32 = 42;
       println(x);
   }
   ```
   - Should compile successfully
   - Binary verification should pass
   - Certificate should be signed
   - Should print "VERIFIED"

### Acceptance Criteria
- [ ] All 4 integration tests pass
- [ ] End-to-end build works
- [ ] Certificates honest and accurate

---

## DAILY BREAKDOWN

### Day 1 (Monday)
- Morning: Binary verifier integration scaffold
- Afternoon: Certificate `binary_verified` field

### Day 2 (Tuesday)
- Morning: Capstone disassembly implementation
- Afternoon: Binary verification tests

### Day 3 (Wednesday)
- Morning: Find existing type checks
- Afternoon: Replace with strict type checker

### Day 4 (Thursday)
- Morning: Fix existing type violations
- Afternoon: Literal overflow checks

### Day 5 (Friday)
- Morning: Find verification calls
- Afternoon: Replace with honest verification

### Day 6 (Saturday)
- Morning: Certificate signature logic
- Afternoon: CLI output updates

### Day 7 (Sunday)
- Morning: Integration test writing
- Afternoon: End-to-end validation

---

## SUCCESS CRITERIA (End of Week 2)

- [ ] Binary verification runs on every build
- [ ] `u64` → `u32` fails with clear error
- [ ] Timeout reports honestly (not "VERIFIED")
- [ ] No signatures on unverified certificates
- [ ] All integration tests pass
- [ ] 1000+ line program compiles with fixes

---

## RISKS & MITIGATIONS

| Risk | Mitigation |
|------|------------|
| Capstone install fails | Use `objdump` fallback |
| Existing code breaks | Fix violations incrementally |
| Tests fail | Debug and fix before proceeding |
| Integration complex | Test each module independently first |

---

## READY TO BEGIN?

Start with **Task 1, Day 1**: Binary verifier integration

First action:
```bash
cd /Users/shy/Developer/ZEUS/zeus_compiler
cargo add capstone
# Then modify main.rs to call binary_verifier
```

---

**Week 2 Goal:** All 3 critical flaws integrated and running in production builds
