struct TelemetryFrame {
    speed: f64,
    temp: f64,
}

@verify(speed < 300.0)
@adaptive(120.0)
pub fn process_telemetry(frame: TelemetryFrame) {
    let mut speed = frame.speed;
    let mut temp = frame.temp;
    
    cluster {
        speed = speed * 1.5;
        temp = temp + 10.0;
    }
}
