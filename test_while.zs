pub fn main() {
    let mut i: i32 = 0;
    while (i < 10) {
        i = i + 1;
    }
    println(i);
    let a: i32 = 5;
    if (a > 0 && a < 10) { println(1); }
    if (a < 0 || a > 3) { println(2); }
    if (a < 0 || a > 100) { println(404); }
}
