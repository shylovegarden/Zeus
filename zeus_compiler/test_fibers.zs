@verify(i < 20)
pub fn process(i: i32) {
    let x = i * 2;
}

pub fn main() {
    process(10);
}

