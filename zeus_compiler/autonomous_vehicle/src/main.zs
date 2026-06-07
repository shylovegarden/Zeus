
import zeus.hw;



pub fn process_telemetry() {
    let mut frames = CanBusFrame[10];
    frames[0].id;
    Assign;
    1;
    frames[0].dlc;
    Assign;
    8;
    let raw_payload = 1;
    BitShiftLeft;
    31;
    Pipe;
    65;
    BitShiftLeft;
    8;
    frames[0].payload;
    Assign;
    raw_payload;
    for i in 0..1 {
        let speed = decode_can_speed(frames[i].payload);
        let fault = is_critical_fault(frames[i].payload);
    }
}
