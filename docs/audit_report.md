# Zeus Compiler Deep Audit Report

## Summary of Findings

| # | Severity | Component | Bug | Status |
|---|----------|-----------|-----|--------|
| 1 | 🔴 CRITICAL | `secret` keyword | `RefCell::clone()` silently drops all secret vars — **memset NEVER fires** | FIXING |
| 2 | 🔴 CRITICAL | `makecontext` | Pointer passed as `int` arg — **truncated on 64-bit, causes segfault** | FIXING |
| 3 | 🟡 MODERATE | `@verify` | Timeout is 1000ms, spec says 2000ms | FIXING |
| 4 | 🟡 MODERATE | `_XOPEN_SOURCE` | Defined AFTER `<stdio.h>` — feature test macro has no effect | FIXING |
| 5 | 🟡 MODERATE | SoA transform | All fields hardcoded to `double` regardless of actual field type | FIXING |
| 6 | 🟡 MODERATE | `#pragma` | Pragma placement is INSIDE the `for` loop — preprocessor cannot nest this way | FIXING |
| 7 | 🟢 LOW | `_source` | Dead variable `_source` on line 31 of codegen.rs | FIXING |

---

## BUG 1: `secret` keyword — memset NEVER fires (CRITICAL)

> [!CAUTION]
> This is a **silent security vulnerability**. Secret variables are NEVER wiped from memory.

### Root Cause
[codegen.rs:279](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs#L279):
```rust
let scope = self.secret_vars.clone(); // <-- CLONES the entire RefCell!
scope.borrow_mut().last_mut()...push(name.clone()); // pushes into CLONE
```

`RefCell::clone()` creates a **deep copy** of the inner `Vec<Vec<String>>`. The `push` goes into the clone, and the original `self.secret_vars` (read at lines 135, 444) remains **permanently empty**. 

### Proof
The generated C for `let secret x = 42` in implicit main:
```c
int main() {
    double x = 42;
    return 0;    // <-- NO memset(&x, 0, sizeof(x)); !!!
}
```

### Fix
Replace `.clone()` with direct `.borrow_mut()`:
```rust
self.secret_vars.borrow_mut().last_mut()
    .expect("Internal Compiler Error: No secret scope")
    .push(name.clone());
```

---

## BUG 2: `makecontext` pointer truncation (CRITICAL)

> [!CAUTION]
> This is a **latent segfault** on 64-bit systems. It works by luck on some platforms but is UB.

### Root Cause
[codegen.rs:348](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs#L348):
```c
makecontext(&ctx, (void(*)()) worker, 1, &__zeus_tasks[i]);
```

Per POSIX, `makecontext` passes arguments as `int` values, NOT `void*`. On LP64 systems (macOS, Linux x86_64), `sizeof(int) == 4` but `sizeof(void*) == 8`. The pointer is **silently truncated** to 32 bits.

### Fix
Split the pointer into two `int` halves, reassemble inside the worker:
```c
// Caller:
uintptr_t _ptr = (uintptr_t)&__zeus_tasks[i];
makecontext(&ctx, (void(*)()) worker, 2, (int)(_ptr), (int)(_ptr >> 32));

// Worker:
void worker(int lo, int hi) {
    void* __zeus_ctx = (void*)((uintptr_t)(unsigned int)lo | ((uintptr_t)(unsigned int)hi << 32));
    ...
}
```

---

## BUG 3: `@verify` timeout is 1000ms, spec says 2000ms

### Root Cause
[analyzer.rs:63](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/analyzer.rs#L63):
```rust
let timeout_threshold = 1000;
```

### Fix
Change to `2000`.

---

## BUG 4: `_XOPEN_SOURCE` defined AFTER system headers

### Root Cause
[codegen.rs:45-47](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs#L45-L47):
```c
#include <stdio.h>     // <-- BEFORE _XOPEN_SOURCE
#include <stdlib.h>
#include <string.h>
...
#define _XOPEN_SOURCE 600   // <-- TOO LATE, has no effect
```

Per POSIX, feature test macros MUST be defined before ANY system header.

### Fix
Move `#define _XOPEN_SOURCE 600` to the very top of the generated file.

---

## BUG 5: SoA hardcodes all fields to `double`

### Root Cause
[codegen.rs:270](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs#L270):
```rust
out.push_str(&format!("{}double {}_{}[{}];\n", pad, name, fname, size_c));
//                       ^^^^^^ hardcoded!
```

Should use `self.type_to_c(&Some(ftype.clone()))` instead.

---

## BUG 6: `#pragma` inside `for` loop body

### Root Cause
The `#if defined(__APPLE__)` / `#pragma` pair is emitted inside the `for` loop at the wrong indentation level, mixed with regular code. Preprocessor directives cannot be conditionally scoped this way inside loop bodies — they're processed before compilation and don't respect braces.

### Fix
Move the pragma to wrap the ENTIRE fiber dispatch block (both loops), not just the interior of the initialization loop.

---

## BUG 7: Dead variable `_source`

### Root Cause
[codegen.rs:31](file:///Users/shy/Developer/ZEUS/zeus_compiler/src/codegen.rs#L31):
```rust
let _source = String::new(); // never used
```

### Fix
Remove it.
