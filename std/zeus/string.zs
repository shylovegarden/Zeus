
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
