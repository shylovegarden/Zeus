# Tutorial 4: Zero-Heap Systems

**Time:** 15 minutes  
**Prerequisites:** [Tutorial 3: Constant-Time Crypto](./03-constant-time-crypto.md)

## What You'll Learn
- Why heap allocation is dangerous for systems programming
- How arena allocation works
- How to write zero-heap code

## The Problem: Heap Allocation

Traditional code uses `malloc` for dynamic memory:

```c
// Traditional C
void* buffer = malloc(1024);  // Can fail!
if (buffer == NULL) {
    // Handle error... but often forgotten
}
process(buffer);
free(buffer);  // Can forget, causing leak
```

**Problems:**
- Memory leaks (forget to free)
- Use-after-free (use after freeing)
- Allocation failures (OOM kills)
- Fragmentation (performance degradation)

## The Solution: Zero-Heap + Arena Allocation

Zeus enforces no `malloc` at compile time:

```zeus
@zero_heap
pub fn main() {
    // This is fine - stack allocation
    let buffer: [u8; 1024] = [0; 1024];
    process(buffer);
    // Automatically freed when function returns
}
```

## Step 1: Stack Allocation

Create `stack.zs`:

```zeus
@zero_heap
fn process_data() -> i32 {
    // Stack allocation - fixed size, no malloc
    let buffer: [u8; 256] = [0; 256];
    
    // Fill with data
    let mut i: i32 = 0;
    while i < 256 {
        buffer[i] = i as u8;
        i = i + 1;
    }
    
    // Return sum
    let mut sum: i32 = 0;
    i = 0;
    while i < 256 {
        sum = sum + buffer[i] as i32;
        i = i + 1;
    }
    
    return sum;
}

pub fn main() {
    let result = process_data();
    println(result);
}
```

## Step 2: Verify Zero-Heap

```bash
zeus verify --policy=zero-heap stack.zs
```

Expected output:
```
✅ Verification passed
   Properties verified:
   - zero_heap: No dynamic memory allocation

🔐 Certificate: stack.zcert
```

## Arena Allocation for Dynamic Needs

When you need "dynamic" allocation, use an arena:

```zeus
@zero_heap
fn process_messages(count: i32) {
    // Pre-allocate arena
    let arena: [u8; 4096] = [0; 4096];
    let mut used: i32 = 0;
    
    let mut i: i32 = 0;
    while i < count {
        // Allocate from arena (bump pointer)
        let msg_size: i32 = 64;
        if used + msg_size > 4096 {
            // Arena exhausted - handle gracefully
            return;
        }
        
        // Use arena[used..used+msg_size]
        // Process message...
        
        used = used + msg_size;
        i = i + 1;
    }
    
    // Arena automatically freed on function exit
}
```

## Why This Matters

### Real-Time Systems
- **Requirement**: Predictable timing
- **Problem**: `malloc` has unpredictable latency
- **Solution**: Zero-heap + arena = bounded allocation time

### Safety-Critical Systems  
- **Requirement**: No memory leaks ever
- **Problem**: Humans forget to free
- **Solution**: Compile-time enforcement

### Embedded Systems
- **Requirement**: Work without OS
- **Problem**: `malloc` requires OS
- **Solution**: Stack + arena work bare-metal

## What Breaks Zero-Heap?

❌ **Bad: malloc/free**
```zeus
@zero_heap
pub fn main() {
    let ptr = malloc(100);  // VIOLATION!
    free(ptr);
}
```

❌ **Bad: Variable-length arrays (VLA)**
```zeus
@zero_heap
fn bad(n: i32) {
    let arr: [i8; n];  // Not allowed in Zeus
}
```

✅ **Good: Fixed-size stack arrays**
```zeus
@zero_heap
fn good() {
    let arr: [i8; 100];  // Fixed size - OK
}
```

## Exercise: Convert to Zero-Heap

Take this traditional code:

```zeus
fn process_items(n: i32) {
    let items = malloc(n * 4);  // Bad!
    // ... process items ...
    free(items);  // Easy to forget
}
```

Convert it to zero-heap by:
1. Using a fixed maximum size
2. Stack-allocating the array
3. Handling overflow gracefully

## Combining Policies

Use multiple policies together:

```zeus
@zero_heap
@constant_time
fn secure_process(secret data: [u8; 64]) -> i32 {
    // Zero-heap: No memory leaks possible
    // Constant-time: No timing leaks
    
    let mut result: i32 = 0;
    let mut i: i32 = 0;
    while i < 64 {
        result = result ^ (data[i] as i32);
        i = i + 1;
    }
    return result;
}
```

Verify both:
```bash
zeus verify --policy=zero-heap,constant-time secure.zs
```

## Summary

✅ You understand heap dangers  
✅ You wrote zero-heap code  
✅ You learned arena allocation  
✅ You combined multiple policies  

**Key Takeaway**: Use `@zero_heap` for systems code that can't tolerate memory leaks or allocation failures.

Next: [Tutorial 5: CI/CD Integration](./05-ci-cd-integration.md)
