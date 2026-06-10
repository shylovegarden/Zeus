pub fn calculate_score(input: i32) -> i32 {
    let result = input * 2;
    proof { assert(result >= input); }
    return result;
}
