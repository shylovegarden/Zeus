// Attitude Control - NASA Class D Aerospace Software
// Satellite attitude determination and control system

@aerospace(class=d)
@nasa_compliant
@real_time
@zero_heap
@wcet(500)
@stack(4KB)

// ============================================================================
// DATA STRUCTURES
// ============================================================================

struct Quaternion {
    w: f64,
    x: f64,
    y: f64,
    z: f64,
}

struct Vector3 {
    x: f64,
    y: f64,
    z: f64,
}

struct Attitude {
    q: Quaternion,      // Orientation quaternion
    rate: Vector3,      // Angular rate
    timestamp: i32,     // Mission time
}

struct SensorData {
    gyro: Vector3,      // Gyroscope readings
    sun_vector: Vector3, // Sun sensor
    mag_vector: Vector3, // Magnetometer
    valid: bool,
}

struct ControlTorque {
    x: f64,
    y: f64,
    z: f64,
    valid: bool,
}

// ============================================================================
// QUATERNION MATH
// ============================================================================

@aerospace(class=d)
@zero_heap
@wcet(100)
pub fn quaternion_multiply(a: Quaternion, b: Quaternion) -> Quaternion {
    @ensures(abs(result.w) <= 2.0 && abs(result.x) <= 2.0 && 
             abs(result.y) <= 2.0 && abs(result.z) <= 2.0)
    
    let mut result: Quaternion;
    result.w = a.w*b.w - a.x*b.x - a.y*b.y - a.z*b.z;
    result.x = a.w*b.x + a.x*b.w + a.y*b.z - a.z*b.y;
    result.y = a.w*b.y - a.x*b.z + a.y*b.w + a.z*b.x;
    result.z = a.w*b.z + a.x*b.y - a.y*b.x + a.z*b.w;
    return result;
}

@aerospace(class=d)
@zero_heap
@wcet(50)
pub fn quaternion_normalize(q: Quaternion) -> Quaternion {
    let norm = sqrt(q.w*q.w + q.x*q.x + q.y*q.y + q.z*q.z);
    
    if norm > 0.0001 {
        let mut result: Quaternion;
        result.w = q.w / norm;
        result.x = q.x / norm;
        result.y = q.y / norm;
        result.z = q.z / norm;
        return result;
    }
    
    return q;
}

fn sqrt(x: f64) -> f64 {
    if x < 0.0 { return 0.0; }
    
    let mut guess = x;
    let mut i: i32 = 0;
    while i < 10 {
        guess = (guess + x/guess) / 2.0;
        i = i + 1;
    }
    return guess;
}

fn abs(x: f64) -> f64 {
    if x < 0.0 { return -x; } else { return x; }
}

// ============================================================================
// VECTOR OPERATIONS
// ============================================================================

@aerospace(class=d)
@zero_heap
@wcet(50)
pub fn vector_cross(a: Vector3, b: Vector3) -> Vector3 {
    let mut result: Vector3;
    result.x = a.y*b.z - a.z*b.y;
    result.y = a.z*b.x - a.x*b.z;
    result.z = a.x*b.y - a.y*b.x;
    return result;
}

@aerospace(class=d)
@zero_heap
@wcet(50)
pub fn vector_dot(a: Vector3, b: Vector3) -> f64 {
    return a.x*b.x + a.y*b.y + a.z*b.z;
}

@aerospace(class=d)
@zero_heap
@wcet(50)
pub fn vector_normalize(v: Vector3) -> Vector3 {
    let norm = sqrt(v.x*v.x + v.y*v.y + v.z*v.z);
    
    if norm > 0.0001 {
        let mut result: Vector3;
        result.x = v.x / norm;
        result.y = v.y / norm;
        result.z = v.z / norm;
        return result;
    }
    
    return v;
}

// ============================================================================
// TRIAD ATTITUDE DETERMINATION
// ============================================================================

@aerospace(class=d)
@zero_heap
@wcet(300)
pub fn triad_attitude_determination(
    sun_body: Vector3,
    sun_ref: Vector3,
    mag_body: Vector3,
    mag_ref: Vector3
) -> Quaternion {
    @requires(abs(vector_dot(sun_body, sun_body) - 1.0) < 0.1)
    @requires(abs(vector_dot(mag_body, mag_body) - 1.0) < 0.1)
    
    // Construct body frame vectors
    let b1 = vector_normalize(sun_body);
    let b2 = vector_normalize(vector_cross(sun_body, mag_body));
    let b3 = vector_cross(b1, b2);
    
    // Construct reference frame vectors
    let r1 = vector_normalize(sun_ref);
    let r2 = vector_normalize(vector_cross(sun_ref, mag_ref));
    let r3 = vector_cross(r1, r2);
    
    // TRIAD solution (simplified)
    // In real implementation, use QUEST or EKF
    let mut q: Quaternion;
    q.w = 1.0;
    q.x = 0.0;
    q.y = 0.0;
    q.z = 0.0;
    
    // Estimate from rotation matrix (simplified)
    let trace = vector_dot(b1, r1) + vector_dot(b2, r2) + vector_dot(b3, r3);
    
    if trace > 0.0 {
        let s = sqrt(trace + 1.0) * 2.0;
        q.w = 0.25 * s;
        q.x = (b2.z - b3.y) / s;
        q.y = (b3.x - b1.z) / s;
        q.z = (b1.y - b2.x) / s;
    }
    
    return quaternion_normalize(q);
}

// ============================================================================
// CONTROL LAW (PD Controller)
// ============================================================================

@aerospace(class=d)
@zero_heap
@wcet(200)
pub fn pd_attitude_control(
    current: Attitude,
    target: Attitude,
    kp: f64,  // Proportional gain
    kd: f64   // Derivative gain
) -> ControlTorque {
    @requires(kp > 0.0 && kd > 0.0)
    
    let mut torque: ControlTorque;
    
    // Quaternion error (simplified)
    let q_error = quaternion_multiply(target.q, current.q);
    
    // PD control
    torque.x = kp * q_error.x - kd * current.rate.x;
    torque.y = kp * q_error.y - kd * current.rate.y;
    torque.z = kp * q_error.z - kd * current.rate.z;
    
    // Clamp to actuator limits
    let max_torque = 0.1;  // Nm
    
    if torque.x > max_torque { torque.x = max_torque; }
    if torque.x < -max_torque { torque.x = -max_torque; }
    if torque.y > max_torque { torque.y = max_torque; }
    if torque.y < -max_torque { torque.y = -max_torque; }
    if torque.z > max_torque { torque.z = max_torque; }
    if torque.z < -max_torque { torque.z = -max_torque; }
    
    torque.valid = true;
    return torque;
}

// ============================================================================
// MAIN CONTROL LOOP
// ============================================================================

@aerospace(class=d)
@zero_heap
@wcet(500)
pub fn attitude_control_loop(
    sensors: SensorData,
    current: Attitude,
    target: Attitude
) -> ControlTorque {
    @requires(sensors.valid == true)
    
    let mut torque: ControlTorque;
    
    if !sensors.valid {
        torque.valid = false;
        return torque;
    }
    
    // Reference vectors (Sun and Earth magnetic field)
    let sun_ref: Vector3;
    sun_ref.x = 1.0;
    sun_ref.y = 0.0;
    sun_ref.z = 0.0;
    
    let mag_ref: Vector3;
    mag_ref.x = 0.0;
    mag_ref.y = 0.0;
    mag_ref.z = 1.0;
    
    // Attitude determination
    let estimated_q = triad_attitude_determination(
        sensors.sun_vector,
        sun_ref,
        sensors.mag_vector,
        mag_ref
    );
    
    // Update current attitude (simplified)
    let mut estimated: Attitude;
    estimated.q = estimated_q;
    estimated.rate = sensors.gyro;
    estimated.timestamp = current.timestamp;
    
    // Control law
    torque = pd_attitude_control(estimated, target, 0.05, 0.02);
    
    return torque;
}

pub fn main() {
    // Demo: Attitude control
    let mut sensors: SensorData;
    sensors.gyro.x = 0.01;
    sensors.gyro.y = 0.02;
    sensors.gyro.z = 0.01;
    sensors.sun_vector.x = 0.9;
    sensors.sun_vector.y = 0.1;
    sensors.sun_vector.z = 0.1;
    sensors.mag_vector.x = 0.1;
    sensors.mag_vector.y = 0.1;
    sensors.mag_vector.z = 0.9;
    sensors.valid = true;
    
    let mut current: Attitude;
    current.q.w = 1.0;
    current.q.x = 0.0;
    current.q.y = 0.0;
    current.q.z = 0.0;
    current.rate.x = 0.0;
    current.rate.y = 0.0;
    current.rate.z = 0.0;
    current.timestamp = 0;
    
    let mut target: Attitude;
    target.q.w = 1.0;
    target.q.x = 0.0;
    target.q.y = 0.0;
    target.q.z = 0.0;
    target.rate.x = 0.0;
    target.rate.y = 0.0;
    target.rate.z = 0.0;
    target.timestamp = 1000;
    
    let torque = attitude_control_loop(sensors, current, target);
    
    println(torque.x);
    println(torque.y);
    println(torque.z);
}
