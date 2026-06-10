struct Entry {
    val: f64,
}

pub fn main() {
    let secret sbox = Entry[256];
    let i = 5;
    sbox[i].val = 42.0;
    let got = sbox[i].val;
}
