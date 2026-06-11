// Satellite Attitude Control - NASA Compliant
// Target: Demonstrate aerospace safety certification

struct Quaternion {
    w: f64,
    x: f64,
    y: f64,
    z: f64
}

struct Vector3 {
    x: f64,
    y: f64,
    z: f64
}

@space_qualified
@nasa_compliant
@radiation_hardened
@zero_heap
@wcet(2000)
pub fn attitude_control(
    current_attitude: Quaternion,
    target_attitude: Quaternion,
    gyro_rates: Vector3,
    dt: f64
) -> Vector3 {
    @requires(is_valid_quaternion(current_attitude))
    @requires(is_valid_quaternion(target_attitude))
    @requires(dt > 0.0 && dt < 1.0)
    @ensures(is_valid_control_output(result))
    
    // Calculate error quaternion
    let error = quaternion_error(current_attitude, target_attitude);
    
    // PD Controller for attitude control
    // Control law: torque = Kp * error - Kd * rate
    let kp: f64 = 2.0;   // Proportional gain
    let kd: f64 = 0.5;   // Derivative gain
    
    let mut control: Vector3;
    
    // X-axis control
    control.x = kp * error.x - kd * gyro_rates.x;
    
    // Y-axis control  
    control.y = kp * error.y - kd * gyro_rates.y;
    
    // Z-axis control
    control.z = kp * error.z - kd * gyro_rates.z;
    
    // Apply control limits (safety bounds)
    control.x = clamp(control.x, -10.0, 10.0);
    control.y = clamp(control.y, -10.0, 10.0);
    control.z = clamp(control.z, -10.0, 10.0);
    
    return control;
}

@space_qualified
@wcet(500)
fn quaternion_error(current: Quaternion, target: Quaternion) -> Quaternion {
    // Calculate rotation from current to target
    // q_error = q_target * conjugate(q_current)
    let mut error: Quaternion;
    
    error.w = target.w * current.w + target.x * current.x + 
              target.y * current.y + target.z * current.z;
    error.x = target.w * current.x - target.x * current.w - 
              target.y * current.z + target.z * current.y;
    error.y = target.w * current.y + target.x * current.z - 
              target.y * current.w - target.z * current.x;
    error.z = target.w * current.z - target.x * current.y + 
              target.y * current.x - target.z * current.w;
    
    return error;
}

fn clamp(val: f64, min: f64, max: f64) -> f64 {
    if val < min { return min; }
    if val > max { return max; }
    return val;
}

fn is_valid_quaternion(q: Quaternion) -> bool {
    // Check normalization: |q| should be close to 1
    let norm_sq = q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z;
    return norm_sq > 0.9 && norm_sq < 1.1;
}

fn is_valid_control_output(v: Vector3) -> bool {
    return v.x >= -10.0 && v.x <= 10.0 &&
           v.y >= -10.0 && v.y <= 10.0 &&
           v.z >= -10.0 && v.z <= 10.0;
}

pub fn main() {
    // Current attitude (identity quaternion)
    let current = Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };
    
    // Target attitude (90 degree rotation around Z)
    let target = Quaternion { 
        w: 0.707, 
        x: 0.0, 
        y: 0.0, 
        z: 0.707 
    };
    
    // Current rotation rates
    let rates = Vector3 { x: 0.1, y: 0.0, z: 0.05 };
    
    let dt: f64 = 0.01;  // 10ms control loop
    
    // Run 1000 control cycles
    let mut i: i32 = 0;
    let mut total_torque: f64 = 0.0;
    
    while i < 1000 {
        let control = attitude_control(current, target, rates, dt);
        total_torque = total_torque + control.x + control.y + control.z;
        i = i + 1;
    }
    
    println(total_torque);
}
