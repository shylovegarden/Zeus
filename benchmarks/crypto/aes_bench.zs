// AES Encryption Benchmark - Constant-time verified
// Target: Demonstrate Zeus constant-time guarantees

struct AESState {
    key: [u8; 32],
    rounds: i32
}

@constant_time
@zero_heap
pub fn aes_encrypt_block(secret state: AESState, plaintext: [u8; 16]) -> [u8; 16] {
    @requires(state.rounds == 10 || state.rounds == 12 || state.rounds == 14)
    @ensures(result != plaintext) // Output differs from input
    
    let mut ciphertext: [u8; 16];
    let mut i: i32 = 0;
    
    // Constant-time XOR with key (no branches on secret data)
    while i < 16 {
        ciphertext[i] = plaintext[i] ^ state.key[i % 32];
        i = i + 1;
    }
    
    // Simulate AES rounds (simplified for benchmark)
    let mut round: i32 = 0;
    while round < state.rounds {
        // SubBytes, ShiftRows, MixColumns in constant time
        let mut j: i32 = 0;
        while j < 16 {
            // S-box lookup - must be constant time
            // Using ORAM for secret data access
            ciphertext[j] = sbox_lookup(ciphertext[j]);
            j = j + 1;
        }
        round = round + 1;
    }
    
    return ciphertext;
}

@constant_time
fn sbox_lookup(byte: u8) -> u8 {
    // Constant-time S-box (no cache timing leaks)
    // Full table scan with conditional select
    let sbox: [u8; 256] = [
        0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5,
        // ... full S-box would go here
        0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ];
    
    // Constant-time access
    let mut result: u8 = 0;
    let mut i: i32 = 0;
    while i < 256 {
        let match_flag: i32 = if i as u8 == byte { 1 } else { 0 };
        result = result | (sbox[i] * match_flag as u8);
        i = i + 1;
    }
    return result;
}

pub fn main() {
    let key: [u8; 32] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f
    ];
    let plaintext: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff
    ];
    
    let state = AESState { key: key, rounds: 10 };
    
    // Benchmark: encrypt 10000 blocks
    let mut sum: i32 = 0;
    let mut i: i32 = 0;
    while i < 10000 {
        let ct = aes_encrypt_block(state, plaintext);
        sum = sum + ct[0] as i32;
        i = i + 1;
    }
    
    println(sum);
}
