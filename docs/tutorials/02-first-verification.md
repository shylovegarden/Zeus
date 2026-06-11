# Tutorial 2: Your First Verification

**Time:** 10 minutes  
**Prerequisites:** [Tutorial 1: Getting Started](./01-getting-started.md)

## What You'll Learn
- How to add security policies to your code
- How to run Zeus verification
- How to read the security certificate

## The Problem

Regular code has bugs that can be exploited:
- Memory leaks (forgotten allocations)
- Timing attacks (secret-dependent branches)
- Buffer overflows (out-of-bounds access)

Zeus mathematically proves these bugs don't exist.

## Step 1: Write a Function

Create `secure.zs`:

```zeus
fn double(x: i32) -> i32 {
    return x * 2;
}

pub fn main() {
    let result = double(21);
    println(result);
}
```

Test it:
```bash
zeus build secure.zs
./secure
# Output: 42
```

## Step 2: Add a Security Policy

Add the `@zero_heap` attribute:

```zeus
@zero_heap
fn double(x: i32) -> i32 {
    return x * 2;
}

pub fn main() {
    let result = double(21);
    println(result);
}
```

## Step 3: Verify

```bash
zeus verify --policy=zero-heap secure.zs
```

Expected output:
```
✅ Verification passed
   Properties verified:
   - zero_heap: No dynamic memory allocation

🔐 Certificate: secure.zcert
```

## Understanding the Certificate

The `.zcert` file contains:
- Function name: `double`
- Properties: `zero_heap`
- Proof: Mathematical verification result
- Signature: Ed25519 signature

View it:
```bash
cat secure.zcert
```

## Step 4: Try a Failing Example

Create `bad.zs`:

```zeus
@zero_heap
pub fn main() {
    // This will fail - malloc is not allowed
    let ptr = malloc(100);
}
```

Verify it:
```bash
zeus verify --policy=zero-heap bad.zs
```

Expected output:
```
❌ Verification failed
   Policy violation: zero_heap
   Location: bad.zs:4
   Issue: Dynamic memory allocation (malloc)
```

## Key Concepts

**@zero_heap**: Proves no `malloc`, `calloc`, `realloc`, or `free` calls exist.

**Certificate**: Cryptographic proof of verification. Share this with auditors.

**Policy Enforcement**: CI/CD can fail builds that don't verify.

## Exercise

Try adding `@constant_time` to a function and verify it:

```zeus
@constant_time
fn compare(a: i32, b: i32) -> bool {
    return a == b;
}
```

(Hint: Simple comparisons are usually constant-time)

## Troubleshooting

**"Policy not satisfied"**
- Check your code follows the policy rules
- For zero_heap: No malloc/free calls
- For constant_time: No secret-dependent branches

**"Certificate not generated"**
- Verification must pass first
- Check for compilation errors

## Summary

✅ You wrote a verified function  
✅ You generated a security certificate  
✅ You understand policy enforcement  

Next: [Tutorial 3: Constant-Time Crypto](./03-constant-time-crypto.md)
