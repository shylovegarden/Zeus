# Zeus Cookbook: Common Patterns

Practical code patterns for secure systems programming.

---

## Table of Contents

1. [Safe Password Comparison](#1-safe-password-comparison)
2. [Constant-Time String Equality](#2-constant-time-string-equality)
3. [Bounded Array Access](#3-bounded-array-access)
4. [Arena Allocation Pattern](#4-arena-allocation-pattern)
5. [Fiber Spawning](#5-fiber-spawning)
6. [Secret Data Handling](#6-secret-data-handling)
7. [FFI Wrapper](#7-ffi-wrapper)
8. [Result Type Handling](#8-result-type-handling)
9. [Error Propagation](#9-error-propagation)
10. [Resource Cleanup](#10-resource-cleanup)

---

## 1. Safe Password Comparison

**Problem:** Comparing passwords with `==` leaks timing information through short-circuit evaluation.

**Solution:** Constant-time comparison using XOR.

```zeus
@constant_time
fn constant_time_compare(a: [u8; 32], b: [u8; 32]) -> bool {
    let mut diff: i32 = 0;
    let mut i: i32 = 0;
    while i < 32 {
        diff = diff | ((a[i] ^ b[i]) as i32);
        i = i + 1;
    }
    return diff == 0;
}

@constant_time
pub fn verify_password(secret input: [u8; 32], stored: [u8; 32]) -> bool {
    return constant_time_compare(input, stored);
}
```

**Why it works:** Always iterates through all 32 bytes, regardless of mismatch position.

---

## 2. Constant-Time String Equality

**Problem:** String comparison can leak which byte differs first.

**Solution:** Compare lengths first, then bytes.

```zeus
@constant_time
fn constant_time_string_eq(a: [u8; 64], b: [u8; 64], len_a: i32, len_b: i32) -> bool {
    // Compare lengths (constant-time)
    let len_eq = (len_a == len_b) as i32;
    
    let mut result: i32 = len_eq;
    let mut i: i32 = 0;
    while i < 64 {
        let byte_eq = (a[i] == b[i]) as i32;
        result = result & byte_eq;
        i = i + 1;
    }
    
    return result == 1;
}
```

---

## 3. Bounded Array Access

**Problem:** Array out-of-bounds access causes crashes or security vulnerabilities.

**Solution:** Compile-time bounds checking with @bounded attribute.

```zeus
@bounded
fn safe_access(data: [i32; 100], index: i32) -> Result<i32, Error> {
    if index < 0 || index >= 100 {
        return Err(Error::OutOfBounds);
    }
    return Ok(data[index]);
}

// Usage
let result = safe_access(data, 50);
match result {
    Ok(val) => println(val),
    Err(e) => println("Access failed"),
}
```

---

## 4. Arena Allocation Pattern

**Problem:** Dynamic allocation not allowed in @zero_heap code.

**Solution:** Pre-allocate arena and bump-allocate from it.

```zeus
@zero_heap
fn process_messages(count: i32) {
    // Pre-allocate arena (4KB)
    let arena: [u8; 4096] = [0; 4096];
    let mut used: i32 = 0;
    
    let mut i: i32 = 0;
    while i < count {
        let msg_size: i32 = 64;
        
        // Check arena space
        if used + msg_size > 4096 {
            return; // Arena full
        }
        
        // Use arena[used..used+msg_size]
        process_message(&arena[used], msg_size);
        
        // Bump pointer
        used = used + msg_size;
        i = i + 1;
    }
    
    // Arena automatically freed on function exit
}
```

---

## 5. Fiber Spawning

**Problem:** Traditional threads have high overhead and non-deterministic scheduling.

**Solution:** Lightweight fibers with cooperative scheduling.

```zeus
fn worker(id: i32) {
    let mut count: i32 = 0;
    while count < 100 {
        println("Worker " + id + ": " + count);
        count = count + 1;
        // Yield to other fibers
        fiber_yield();
    }
}

pub fn main() {
    // Spawn 3 workers
    fiber_spawn(|| worker(1));
    fiber_spawn(|| worker(2));
    fiber_spawn(|| worker(3));
    
    // Main fiber continues
    println("Main fiber done");
}
```

---

## 6. Secret Data Handling

**Problem:** Secret data may be leaked in stack traces, core dumps, or swap.

**Solution:** Mark data as secret, use secure wipe.

```zeus
pub fn handle_secret() {
    // Mark as secret
    let secret_key: [u8; 32] = [0; 32];
    
    // Use the secret
    let result = crypto_operation(secret_key);
    
    // Secure wipe (overwrite before deallocation)
    secure_wipe(secret_key);
    
    return result;
}
```

**Verification:** Zeus proves no secret data is copied to non-secret variables.

---

## 7. FFI Wrapper

**Problem:** Calling C code from Zeus needs safe wrappers.

**Solution:** Explicit FFI declarations with safety annotations.

```zeus
// Declare external C function
extern "C" {
    fn c_malloc(size: usize) -> *mut u8;
    fn c_free(ptr: *mut u8);
}

// Safe wrapper
fn safe_malloc(size: usize) -> Result<*mut u8, Error> {
    let ptr = c_malloc(size);
    if ptr.is_null() {
        return Err(Error::OutOfMemory);
    }
    return Ok(ptr);
}

@zero_heap
fn use_c_library() {
    // Use wrapper (not allowed in zero_heap - just example)
    // In practice: arena allocation preferred
}
```

---

## 8. Result Type Handling

**Problem:** Error handling can be verbose and error-prone.

**Solution:** Zeus Result type with ? operator.

```zeus
fn may_fail() -> Result<i32, Error> {
    if condition {
        return Ok(42);
    } else {
        return Err(Error::InvalidInput);
    }
}

fn caller() -> Result<i32, Error> {
    // ? propagates errors
    let value = may_fail()?;
    return Ok(value * 2);
}

pub fn main() {
    match caller() {
        Ok(val) => println("Success: " + val),
        Err(e) => println("Error: " + e),
    }
}
```

---

## 9. Error Propagation

**Problem:** Nested error handling creates deeply indented code.

**Solution:** Early returns with Result chaining.

```zeus
fn complex_operation() -> Result<i32, Error> {
    // Step 1
    let step1 = operation1()?;
    
    // Step 2
    let step2 = operation2(step1)?;
    
    // Step 3
    let step3 = operation3(step2)?;
    
    return Ok(step3);
}
```

**Benefit:** Flat structure, errors bubble up automatically.

---

## 10. Resource Cleanup

**Problem:** Resources (files, memory) must be cleaned up even on errors.

**Solution:** RAII pattern with deterministic cleanup.

```zeus
fn use_resource() {
    // Resource acquired
    let resource = acquire_resource();
    
    // Use resource
    let result = process(resource);
    
    // Resource automatically released when going out of scope
    // (Zeus guarantees cleanup in all code paths)
    
    return result;
}
```

**Verification:** Zeus proves no resource leaks.

---

## More Patterns

See individual pattern files:
- [crypto-patterns.md](./crypto-patterns.md) - Encryption, signing, hashing
- [network-patterns.md](./network-patterns.md) - Sockets, protocols, parsing
- [system-patterns.md](./system-patterns.md) - Low-level systems programming

---

## Contributing

Add your own patterns:
1. Create a new `.md` file
2. Follow the format: Problem → Solution → Code → Explanation
3. Include verification attributes
4. Test the code before submitting
