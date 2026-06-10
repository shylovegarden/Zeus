// 07 -- Structs + Structure-of-Arrays (SoA): `Name[N]` allocates a fixed
// buffer of N records, decomposed into aligned per-field arrays. Access
// individual fields with arr[i].field. No heap is used.
struct Point {
    x: i32,
    y: i32,
}

pub fn main() {
    let pts = Point[4];
    pts[0].x = 10;
    pts[0].y = 20;
    pts[1].x = 3;
    pts[1].y = 4;
    let sum: i32 = pts[0].x + pts[0].y + pts[1].x + pts[1].y;
    println(sum);  // 37
}
