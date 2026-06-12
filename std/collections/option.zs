// std/collections/option.zs
// Zero-heap Option enum using Zeus tagged unions

enum OptionI32 {
    Some(i32),
    None,
}

pub fn is_some(opt: OptionI32) -> bool {
    match opt {
        OptionI32::Some(v) => { return true; }
        OptionI32::None => { return false; }
    }
}

pub fn unwrap_or(opt: OptionI32, default_val: i32) -> i32 {
    match opt {
        OptionI32::Some(v) => { return v; }
        OptionI32::None => { return default_val; }
    }
}
