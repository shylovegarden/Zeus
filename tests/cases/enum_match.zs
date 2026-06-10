enum Direction { North, South, East, West }

pub fn main() {
    let d: Direction = Direction::North;
    match d {
        Direction::North => { println(1); }
        Direction::South => { println(2); }
        Direction::East => { println(3); }
        Direction::West => { println(4); }
        _ => { println(0); }
    }

    let e: Direction = Direction::West;
    match e {
        Direction::West => { println(99); }
        _ => { println(0); }
    }
}
