// Crypto Library Template - Constant-Time Verified
@crypto_library
@constant_time
@zero_heap

@constant_time
pub fn constant_time_compare(a: [u8; 32], b: [u8; 32]) -> i32 {
    @ensures(result == 0 implies arrays_equal(a, b))
    let mut result: i32 = 0;
    let mut i: i32 = 0;
    while i < 32 {
        result = result | ((a[i] ^ b[i]) as i32);
        i = i + 1;
    }
    return result;
}

fn arrays_equal(a: [u8; 32], b: [u8; 32]) -> bool {
    let mut i: i32 = 0;
    while i < 32 {
        if a[i] != b[i] { return false; }
        i = i + 1;
    }
    return true;
}

pub fn main() {
    let a: [u8; 32];
    let b: [u8; 32];
    let eq = constant_time_compare(a, b);
    println(eq);
}
