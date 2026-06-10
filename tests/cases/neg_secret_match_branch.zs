enum Color { Red, Green, Blue }

@constant_time
pub fn process(secret c: Color) {
    match c {
        Color::Red   => { println(1); }
        Color::Green => { println(2); }
        Color::Blue  => { println(3); }
        _ => { println(0); }
    }
}

pub fn main() {
    process(Color::Red);
}
