// Calling functions; a function calling another function.
fn add(x: i32, y: i32) -> i32 {
    return x + y;
}

fn twice(n: i32) -> i32 {
    return add(n, n);
}

pub fn main() {
    let r: i32 = twice(add(3, 4));
    println(r);
}
