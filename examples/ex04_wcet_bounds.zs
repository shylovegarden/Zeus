// 04 -- WCET bounds: @wcet asserts a worst-case execution time budget.
// Because the for loop has a constant bound (0..8), the compiler can
// prove the function fits inside the declared step budget.
@wcet(5000)
fn sum_to(limit: i32) -> i32 {
    let mut acc: i32 = 0;
    for i in 0..8 {
        acc = acc + limit;
    }
    return acc;
}

pub fn main() {
    println(sum_to(5));  // 5 added 8 times = 40
}
