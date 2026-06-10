// WASM-target demo: pure integer functions (the verifiable subset).
// Compiles to WAT and runs in any WebAssembly runtime.

@wcet(2000)
@deterministic
fn relu(v: i32) -> i32 {
    if v > 0 { return v; }
    return 0;
}

@wcet(2000)
@deterministic
fn clampv(v: i32, lo: i32, hi: i32) -> i32 {
    if v < lo { return lo; }
    if v > hi { return hi; }
    return v;
}

// fixed 4-tap dot product (weights baked in) -- a 1-neuron "tensor" op, no arrays
@wcet(4000)
@deterministic
fn neuron4(x0: i32, x1: i32, x2: i32, x3: i32) -> i32 {
    let mut acc: i32 = 0;
    acc = 2 * x0 - x1 + 3 * x2 + x3;
    return relu(acc);
}

// bounded accumulation loop -> exercises the wasm loop lowering
@wcet(40000)
@deterministic
fn ramp(n_unused: i32) -> i32 {
    let mut s: i32 = 0;
    for i in 0..10 { s = s + i; }
    return s;
}

pub fn main() {
    println(neuron4(3, 1, 2, 1));
}
