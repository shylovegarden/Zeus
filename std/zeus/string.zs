
struct ZeusString {
    ptr: u64,
    len: u64,
    cap: u64,
}

extern fn strlen(s: str) -> u64
extern fn strcpy(dst: str, src: str) -> str
extern fn strcat(dst: str, src: str) -> str
extern fn strcmp(a: str, b: str) -> i32
extern fn strncpy(dst: str, src: str, n: u64) -> str
extern fn memcpy(dst: u64, src: u64, n: u64) -> u64

pub fn string_len(s: str) -> u64 {
    return strlen(s);
}

pub fn string_eq(a: str, b: str) -> bool {
    let r = strcmp(a, b);
    return r == 0;
}

struct BoundedString {
    buffer: [i8; 256],
    len: u64,
}

pub fn string_concat_bounded(a: BoundedString, b: BoundedString) -> BoundedString {
    let mut result = BoundedString { buffer: [0], len: 0 };
    let mut i = 0;
    while i < a.len {
        result.buffer[i] = a.buffer[i];
        i = i + 1;
    }
    let mut j = 0;
    while j < b.len {
        if i < 256 {
            result.buffer[i] = b.buffer[j];
            i = i + 1;
        }
        j = j + 1;
    }
    result.len = i;
    return result;
}

pub fn string_slice_bounded(s: BoundedString, start: u64, end: u64) -> BoundedString {
    let mut result = BoundedString { buffer: [0], len: 0 };
    let mut i = start;
    let mut j = 0;
    while i < end {
        if i < s.len {
            if j < 256 {
                result.buffer[j] = s.buffer[i];
                j = j + 1;
            }
        }
        i = i + 1;
    }
    result.len = j;
    return result;
}
