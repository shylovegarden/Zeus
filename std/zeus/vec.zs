
struct ZeusVec {
    ptr: u64,
    len: u64,
    cap: u64,
    elem_size: u64,
}

extern fn memcpy(dst: u64, src: u64, n: u64) -> u64

pub fn vec_len(v: ZeusVec) -> u64 {
    return v.len;
}

pub fn vec_cap(v: ZeusVec) -> u64 {
    return v.cap;
}

pub fn vec_is_empty(v: ZeusVec) -> bool {
    return v.len == 0;
}
