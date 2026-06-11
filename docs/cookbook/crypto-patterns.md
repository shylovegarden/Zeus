# Cryptography Patterns

Secure cryptographic operations in Zeus.

## AES Encryption Wrapper

```zeus
@constant_time
@zero_heap
fn aes_encrypt_block(
    secret key: [u8; 32],
    secret plaintext: [u8; 16],
    ciphertext: [u8; 16]
) {
    // AES encryption - single block
    // Implementation uses constant-time lookups
    let mut state: [u8; 16] = plaintext;
    
    // Key schedule and rounds...
    // All operations are constant-time
    
    // Copy result
    let mut i: i32 = 0;
    while i < 16 {
        ciphertext[i] = state[i];
        i = i + 1;
    }
}
```

## SHA-256 Hashing

```zeus
@zero_heap
fn sha256_hash(data: [u8; 64], hash: [u8; 32]) {
    // SHA-256 implementation
    // Uses pre-computed round constants
    // Zero-heap: No dynamic allocation
}
```

## HMAC Construction

```zeus
@constant_time
fn hmac_sha256(
    secret key: [u8; 32],
    message: [u8; 128],
    mac: [u8; 32]
) {
    // HMAC construction
    // inner key pad and outer key pad
    // Constant-time comparison at end
}
```

## Secure Random Number Generation

```zeus
@zero_heap
@constant_time
fn secure_random(bytes: [u8; 32]) {
    // Read from hardware RNG
    // Zero-heap: No malloc
    // Constant-time: No branches on output
}
```
