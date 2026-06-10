// 08 -- Tiny net: one fixed-weight neuron computing a dot product.
// Inputs and weights live in a small SoA buffer; the loop is constant-
// bounded so the compiler proves a worst-case time (@wcet) and that the
// result is reproducible (@deterministic). This is "AI baked in" the
// honest way: fixed weights + a certified, bounded computation.
struct Lane {
    w: i32,
    x: i32,
}

@wcet(60000)
@deterministic
fn neuron(x0: i32, x1: i32, x2: i32, x3: i32) -> i32 {
    let buf = Lane[4];
    buf[0].x = x0;
    buf[1].x = x1;
    buf[2].x = x2;
    buf[3].x = x3;
    buf[0].w = 2;
    buf[1].w = 0 - 1;
    buf[2].w = 3;
    buf[3].w = 1;
    let mut acc: i32 = 0;
    for i in 0..4 {
        acc = acc + buf[i].w * buf[i].x;
    }
    return acc;
}

pub fn main() {
    // 2*3 + (-1)*1 + 3*2 + 1*1 = 6 - 1 + 6 + 1 = 12
    println(neuron(3, 1, 2, 1));
}
