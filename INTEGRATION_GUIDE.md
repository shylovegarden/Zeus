# INTEGRATION GUIDE: Adding Critical Flaws to main.rs

**Status:** Integration module created, needs to be wired into main.rs

---

## WHAT'S BEEN DONE

✅ **3 Critical Flaw Modules Created:**
1. `binary_verifier/mod.rs` - Assembly-level timing leak detection
2. `type_checker_strict.rs` - Sound type system with width checking
3. `honest_verification.rs` - Honest timeout reporting
4. `critical_fixes_integration.rs` - Unified API

---

## WHAT NEEDS TO BE DONE

### Step 1: Add module declaration to main.rs

Add to the top of `zeus_compiler/src/main.rs`:

```rust
// CRITICAL FLAW FIXES - Phase 1 Integration
mod critical_fixes_integration;
```

### Step 2: Replace type checking in analyzer

In `analyzer.rs`, find type checking code and replace with:

```rust
use crate::type_checker_strict::StrictTypeChecker;

let checker = StrictTypeChecker::new();
match checker.check_assignment(&target_type, &value_type) {
    Ok(()) => {},
    Err(e) => return Err(format!("Type error: {:?}", e)),
}
```

### Step 3: Replace verification calls

In `main.rs`, find where `formal_verifier` is called and replace:

```rust
// OLD:
// let mut verifier = FormalVerifier::new();
// verifier.verify(&program, false)?;

// NEW:
use crate::critical_fixes_integration::{apply_critical_flaws_check, print_critical_flaws_report};

let result = apply_critical_flaws_check(
    Some(Path::new(&base_name)),  // binary path
    &target_type,
    &value_type,
    &expr_to_verify,
    2000,  // timeout ms
);

print_critical_flaws_report(&result);

if !result.should_sign {
    eprintln!("Build completed but certificate will NOT be signed due to verification failures");
}
```

### Step 4: Update certificate signing

In `cert_sign.rs`, update to use HonestCertificate:

```rust
use crate::honest_verification::HonestCertificate;

// Only sign if should_sign is true
if certificate.should_sign {
    let signature = ed25519_sign(&data);
    certificate.signature = Some(signature);
} else {
    certificate.signature = None;
}
```

---

## TESTING

After integration, test with:

```bash
cd zeus_compiler
cargo test
cargo run -- build test_program.zs
```

### Test Cases:

1. **Type mismatch should fail:**
```zeus
pub fn main() {
    let x: u64 = 1000;
    let y: u32 = x;  // Should ERROR
}
```

2. **Timeout should report honestly:**
```zeus
pub fn complex() {
    // Many nested conditions
    // Should print "TIMEOUT" not "VERIFIED"
}
```

3. **Successful build should sign:**
```zeus
@constant_time
pub fn main() {
    println("Hello");
}
// Should generate signed certificate
```

---

## NEXT STEPS (Revolutionary Features)

After critical flaws are integrated:

### 1. AI Code Verification
```rust
// New attribute
@ai_generated
@verify_before_run
pub fn ai_code() { ... }
```

### 2. Blockchain Backend
```bash
zeus build contract.zs --target=evm
```

### 3. Medical Certification
```rust
@medical_device(class=3)
@fda_compliant
fn device() { ... }
```

### 4. Quantum Crypto
```rust
@post_quantum
@constant_time
fn kyber() { ... }
```

---

## FILES TO MODIFY

1. `zeus_compiler/src/main.rs` - Add module, wire integration
2. `zeus_compiler/src/analyzer.rs` - Use strict type checker
3. `zeus_compiler/src/cert_sign.rs` - Use honest certificates

---

## READY TO IMPLEMENT

The critical flaw modules are DONE. Just need to wire them into the compiler pipeline.

**Estimated time:** 2-3 hours to complete integration

---

**Status:** Phase 1 Week 2 - Integration in progress
