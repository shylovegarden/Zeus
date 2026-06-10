// VERIFIED EDGE-AI: a fixed-weight, quantized 2-layer perceptron.
//   4 inputs  ->  3 ReLU hidden neurons  ->  1 linear output.
// Integer (INT8-style) weights are baked in; there is NO heap and every loop is
// constant-bounded, so the Zeus compiler PROVES a worst-case execution time
// (@wcet) and that the inference is reproducible (@deterministic). This is the
// honest form of "AI baked in": fixed weights + certified bounded inference,
// not a self-modifying binary. Retraining happens offline; the new weights are
// recompiled and re-certified.

struct Lane { w: i32, x: i32 }

@wcet(4000)
@deterministic
fn relu(v: i32) -> i32 {
    if v > 0 { return v; }
    return 0;
}

@wcet(60000)
@deterministic
fn neuron(x0: i32, x1: i32, x2: i32, x3: i32, w0: i32, w1: i32, w2: i32, w3: i32, b: i32) -> i32 {
    let buf = Lane[4];
    buf[0].x = x0;
    buf[1].x = x1;
    buf[2].x = x2;
    buf[3].x = x3;
    buf[0].w = w0;
    buf[1].w = w1;
    buf[2].w = w2;
    buf[3].w = w3;
    let mut acc: i32 = b;
    for i in 0..4 { acc = acc + buf[i].w * buf[i].x; }
    return relu(acc);
}

@wcet(250000)
@deterministic
fn infer(x0: i32, x1: i32, x2: i32, x3: i32) -> i32 {
    let h0: i32 = neuron(x0, x1, x2, x3, 2, 0 - 1, 3, 1, 0);
    let h1: i32 = neuron(x0, x1, x2, x3, 1, 1, 0 - 2, 4, 0 - 3);
    let h2: i32 = neuron(x0, x1, x2, x3, 0 - 1, 2, 1, 0 - 1, 2);
    let mut y: i32 = 0;
    y = h0 + 2 * h1 + h2;
    return y;
}

pub fn main() {
    println(infer(3, 1, 2, 1));
}
