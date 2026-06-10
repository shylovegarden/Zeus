// FLAGSHIP: Smart contract -- provably deterministic + gas(WCET)-bounded.
@deterministic
@wcet(2000)
fn supply(balances: [i32; 8]) -> i32 {
    let mut total: i32 = 0;
    for i in 0..8 { total = total + balances[i]; }
    return total;
}
pub fn main() {
    let bal: [i32; 8] = [100, 50, 25, 10, 5, 1, 0, 0];
    println(supply(bal));
}
