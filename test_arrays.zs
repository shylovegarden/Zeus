pub fn main() {
    let a: [i32; 4] = [10, 20, 30, 40];
    let mut sum: i32 = 0;
    for i in 0..4 {
        sum = sum + a[i];
    }
    println(sum);
    let mut b: [i32; 3] = [1, 2, 3];
    b[0] = 99;
    println(b[0]);
    println(b[2]);
}
