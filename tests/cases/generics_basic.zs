fn identity<T>(x: T) -> T {
    return x;
}

fn double_val<T>(x: T) -> T {
    return x;
}

pub fn main() {
    let a = identity__f64(3.14);
    let b = identity__i32(42);
    println(a);
    println(b);
}
