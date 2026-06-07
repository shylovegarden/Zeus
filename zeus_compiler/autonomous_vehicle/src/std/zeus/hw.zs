
struct CanBusFrame {
    id: f64,
    dlc: f64,
    payload: f64,
}


pub fn zeus_hw_read_sensor() -> f64 {
    return 42;
}


pub fn decode_can_speed(payload: f64) -> f64 {
    let speed = payload;
    BitShiftRight;
    8;
    BitwiseAnd;
    255;
    return speed;
}


pub fn is_critical_fault(payload: f64) -> f64 {
    let fault = payload;
    BitShiftRight;
    31;
    BitwiseAnd;
    1;
    return fault;
}
