// Aerospace Control Template - NASA Compliant
@space_qualified
@nasa_compliant
@radiation_hardened

struct Vector3 { x: f64, y: f64, z: f64 }

@space_qualified
@wcet(1000)
@stack(2048)
pub fn attitude_determination(sun_sensor: Vector3, mag_sensor: Vector3) -> Vector3 {
    @requires(is_valid_vector(sun_sensor))
    @requires(is_valid_vector(mag_sensor))
    @ensures(is_valid_vector(result))
    
    // TRIAD algorithm (simplified)
    let mut attitude: Vector3;
    attitude.x = sun_sensor.x * 0.5 + mag_sensor.x * 0.5;
    attitude.y = sun_sensor.y * 0.5 + mag_sensor.y * 0.5;
    attitude.z = sun_sensor.z * 0.5 + mag_sensor.z * 0.5;
    
    return normalize(attitude);
}

fn is_valid_vector(v: Vector3) -> bool {
    return v.x >= -1.0 && v.x <= 1.0 &&
           v.y >= -1.0 && v.y <= 1.0 &&
           v.z >= -1.0 && v.z <= 1.0;
}

fn normalize(v: Vector3) -> Vector3 {
    let len = sqrt(v.x * v.x + v.y * v.y + v.z * v.z);
    if len > 0.0 {
        return Vector3 { x: v.x / len, y: v.y / len, z: v.z / len };
    }
    return v;
}

pub fn main() {
    let sun = Vector3 { x: 1.0, y: 0.0, z: 0.0 };
    let mag = Vector3 { x: 0.0, y: 1.0, z: 0.0 };
    let att = attitude_determination(sun, mag);
    println(att.x);
}
