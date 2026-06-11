# Tutorial 3: Constant-Time Cryptography

**Time:** 15 minutes  
**Prerequisites:** [Tutorial 2: First Verification](./02-first-verification.md)

## What You'll Learn
- Why timing attacks are dangerous
- How to write constant-time code
- How Zeus proves constant-time properties

## The Danger: Timing Attacks

Imagine a password check that returns early on mismatch:

```
// BAD: Vulnerable to timing attack
fn check_password(input: [u8; 4], stored: [u8; 4]) -> bool {
    for i in 0..4 {
        if input[i] != stored[i] {  // Returns early!
            return false;
        }
    }
    return true;
}
```

An attacker can measure how long the check takes and guess the password one byte at a time!

## The Solution: Constant-Time Comparison

Write code that takes the same time regardless of input:

```zeus
@constant_time
fn verify_password(secret input: [u8; 4], stored: [u8; 4]) -> bool {
    let mut diff: i32 = 0;
    let mut i: i32 = 0;
    while i < 4 {
        diff = diff | ((input[i] ^ stored[i]) as i32);
        i = i + 1;
    }
    return diff == 0;
}

pub fn main() {
    let stored: [u8; 4] = [1, 2, 3, 4];
    let input: [u8; 4] = [1, 2, 3, 4];
    let result = verify_password(input, stored);
    println(result);
}
```

## Step 1: Write the Function

Create `password.zs`:

```zeus
@constant_time
fn verify_password(secret input: [u8; 8], stored: [u8; 8]) -> bool {
    let mut diff: i32 = 0;
    let mut i: i32 = 0;
    while i < 8 {
        diff = diff | ((input[i] ^ stored[i]) as i32);
        i = i + 1;
    }
    return diff == 0;
}

pub fn main() {
    let stored: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
    let input: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
    
    if verify_password(input, stored) {
        println("Access granted");
    } else {
        println("Access denied");
    }
}
```

## Step 2: Verify Constant-Time

```bash
zeus verify --policy=constant-time password.zs
```

Expected output:
```
✅ Verification passed
   Properties verified:
   - constant_time: No secret-dependent timing

🔐 Certificate: password.zcert
```

## Understanding the Proof

Zeus proves:
1. No branches depend on `secret input`
2. All iterations execute (no early returns)
3. Memory access patterns are uniform

## What Breaks Constant-Time?

❌ **Bad: Secret-dependent if statement**
```zeus
if secret_value == 0 {  // Timing leak!
    do_something();
}
```

❌ **Bad: Early return**
```zeus
for i in 0..n {
    if data[i] == target {  // Returns early!
        return i;
    }
}
```

✅ **Good: Fixed iterations**
```zeus
@constant_time
fn search(data: [i32; 100], target: i32) -> i32 {
    let mut result: i32 = -1;
    let mut i: i32 = 0;
    while i < 100 {
        let match = (data[i] == target) as i32;
        result = if match == 1 { i } else { result };
        i = i + 1;
    }
    return result;
}
```

## Real-World Impact

**Heartbleed (2014)**: Timing side-channel in OpenSSL leaked private keys

**Spectre/Meltdown (2018)**: Hardware timing attacks bypassed isolation

**Constant-time code prevents these attacks.**

## Exercise: Fix the Vulnerable Code

Take this vulnerable function:

```zeus
fn find_user(username: [u8; 8]) -> i32 {
    let users = [[0, 0, 0, 0, 0, 0, 0, 1], 
                 [0, 0, 0, 0, 0, 0, 0, 2]];
    let mut i: i32 = 0;
    while i < 2 {
        if users[i] == username {  // Secret comparison!
            return i;
        }
        i = i + 1;
    }
    return -1;
}
```

Make it constant-time by:
1. Adding `@constant_time` attribute
2. Removing the early return
3. Using constant-time comparison

## Testing Your Fix

```bash
zeus verify --policy=constant-time your_fix.zs
```

If it passes, you've successfully eliminated the timing side-channel!

## Summary

✅ You understand timing attacks  
✅ You wrote constant-time code  
✅ You verified it with Zeus  

**Key Takeaway**: Always use `@constant_time` for cryptographic code that handles secrets.

Next: [Tutorial 4: Zero-Heap Systems](./04-zero-heap-systems.md)
