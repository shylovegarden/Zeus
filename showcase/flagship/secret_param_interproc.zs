fn reveal(secret k: i32) -> i32 {
    return k
}

fn consume() -> i32 {
    let x: i32 = reveal(9)
    if x > 0 {
        return 1
    }
    return 0
}

fn main() {
    let r: i32 = consume()
    println(r)
}
