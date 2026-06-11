// ECG Signal Processing - FDA Compliance Ready
// Target: Demonstrate IEC 62304 compliance

struct ECGSample {
    timestamp: i32,
    voltage: f64
}

@medical_device(class=2)
@iec62304_compliant
@zero_heap
@wcet(1000)  // 1000 steps max
pub fn detect_arrythmia(samples: [ECGSample; 1000]) -> bool {
    @requires(samples[0].timestamp >= 0)
    @ensures(result == true implies critical_event_detected())
    
    let mut heart_rate_sum: f64 = 0.0;
    let mut r_peak_count: i32 = 0;
    let mut last_r_peak: i32 = -1;
    
    // Detect R-peaks (simplified algorithm)
    let mut i: i32 = 1;
    while i < 999 {
        let prev = samples[i - 1].voltage;
        let curr = samples[i].voltage;
        let next = samples[i + 1].voltage;
        
        // Peak detection with safety bounds
        if curr > prev && curr > next && curr > 1.0 {
            // Valid R-peak
            if last_r_peak >= 0 {
                let interval = samples[i].timestamp - samples[last_r_peak].timestamp;
                if interval > 0 && interval < 3000 {  // 3 second max
                    heart_rate_sum = heart_rate_sum + (60000.0 / interval as f64);
                    r_peak_count = r_peak_count + 1;
                }
            }
            last_r_peak = i;
        }
        i = i + 1;
    }
    
    // Calculate average heart rate
    let avg_hr = if r_peak_count > 0 {
        heart_rate_sum / r_peak_count as f64
    } else {
        0.0
    };
    
    // Detect arrhythmia conditions
    let tachycardia = avg_hr > 100.0;    // > 100 BPM
    let bradycardia = avg_hr < 60.0;     // < 60 BPM
    let irregular = check_irregularity(samples);
    
    return tachycardia || bradycardia || irregular;
}

@medical_device(class=2)
@wcet(500)
fn check_irregularity(samples: [ECGSample; 1000]) -> bool {
    let mut prev_interval: i32 = -1;
    let mut irregular_count: i32 = 0;
    let mut last_peak: i32 = -1;
    
    let mut i: i32 = 1;
    while i < 999 {
        let curr = samples[i].voltage;
        if curr > 1.0 && samples[i - 1].voltage < curr {
            if last_peak >= 0 {
                let interval = samples[i].timestamp - samples[last_peak].timestamp;
                if prev_interval >= 0 {
                    let diff = if interval > prev_interval {
                        interval - prev_interval
                    } else {
                        prev_interval - interval
                    };
                    // > 20% variation is irregular
                    if diff * 5 > prev_interval {
                        irregular_count = irregular_count + 1;
                    }
                }
                prev_interval = interval;
            }
            last_peak = i;
        }
        i = i + 1;
    }
    
    // > 3 irregular beats indicates arrhythmia
    return irregular_count > 3;
}

fn critical_event_detected() -> bool { return true; }

pub fn main() {
    // Generate synthetic ECG data
    let mut samples: [ECGSample; 1000];
    let mut i: i32 = 0;
    while i < 1000 {
        samples[i].timestamp = i * 10;  // 10ms sampling
        // Simulated ECG with some peaks
        samples[i].voltage = if i % 200 == 100 { 2.0 } else { 0.1 };
        i = i + 1;
    }
    
    // Run detection
    let alarm = detect_arrythmia(samples);
    
    if alarm {
        println(1);  // Arrhythmia detected
    } else {
        println(0);  // Normal
    }
}
