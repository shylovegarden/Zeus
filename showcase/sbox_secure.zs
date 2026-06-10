// =============================================================================
// Zeus Security Showcase: access-pattern-oblivious secret table lookup
// =============================================================================
//
// THREAT: A classic cache-timing side channel. When a program indexes a table
// with a SECRET value (the textbook example is the AES S-box, indexed by
// key-dependent bytes), the CPU cache leaves a footprint: the cache lines
// touched depend on the secret index. An attacker sharing the machine can
// probe the cache (Prime+Probe / Flush+Reload) and recover the secret index
// from *which* line was loaded -- without ever reading the data itself.
//
// ZEUS DEFENSE: declaring the table `secret` makes every indexed access compile
// to a CONSTANT-TIME full scan over ALL entries (see __zeus_oread_bytes /
// __zeus_owrite_bytes in the emitted C). The set of memory locations touched is
// the SAME for every index, so the access pattern reveals nothing about k.
//
// This file builds a 256-entry secret table and performs several secret-index
// lookups. The printed results are identical to an ordinary table[k] lookup
// (see sbox_naive.c) -- correctness is preserved; only the access pattern
// changes.
// =============================================================================

struct Entry {
    val: i32,
}

pub fn main() {
    // `secret` => array indexing becomes an oblivious (constant-time) full scan.
    let secret sbox = Entry[256];

    // Populate the table with a simple invertible-looking byte permutation:
    //   sbox[i].val = (i * 7 + 11) & 255
    // (Stands in for the real AES S-box; the security property is identical.)
    let mut i: i32 = 0;
    while i < 256 {
        sbox[i].val = (i * 7 + 11) & 255;
        i = i + 1;
    }

    // ---- Secret-index lookups -----------------------------------------------
    // In a real cipher these indices are derived from the secret key/state.
    // Each read below compiles to a 256-entry oblivious scan: the memory
    // access pattern is INDEPENDENT of k1/k2/k3.
    let k1: i32 = 42;
    let k2: i32 = 200;
    let k3: i32 = 7;

    let v1 = sbox[k1].val;
    let v2 = sbox[k2].val;
    let v3 = sbox[k3].val;

    // Expected (also produced by naive C):
    //   42*7+11 = 305 & 255 =  49
    //  200*7+11 =1411 & 255 = 131
    //    7*7+11 =  60 & 255 =  60
    println(v1);
    println(v2);
    println(v3);
}
