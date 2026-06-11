// ECG Monitor - FDA Class II Medical Device
// Real-time arrhythmia detection with formal verification

@medical_device(class=2)
@iec62304_compliant
@fda_submission(510k=true)
@zero_heap
@wcet(1000)
@stack(2KB)

// ============================================================================
// DATA STRUCTURES
// ============================================================================

struct ECGSample {
    timestamp: i32,      // milliseconds
    voltage: f64,        // mV
    lead: i32,          // 0=I, 1=II, 2=III, etc.
    quality: i32,       // Signal quality 0-100
}

struct HeartBeat {
    timestamp: i32,
    r_peak_amplitude: f64,
    interval_ms: i32,    // Time since last beat
}

struct ArrhythmiaResult {
    alarm_level: i32,    // 0=none, 1=warning, 2=critical
    heart_rate: f64,
    arrhythmia_type: i32,  // 0=normal, 1=AFib, 2=VTach, 3=Brady, 4=Tachy
    confidence: f64,
}

// ============================================================================
// SIGNAL PROCESSING
// ============================================================================

@medical_device(class=2)
@zero_heap
@wcet(200)
pub fn filter_signal(sample: ECGSample, prev_samples: [ECGSample; 5]) -> ECGSample {
    @requires(sample.quality >= 0 && sample.quality <= 100)
    @ensures(result.quality >= 0 && result.quality <= 100)
    
    let mut filtered = sample;
    
    // Simple moving average filter for noise reduction
    let mut sum: f64 = sample.voltage;
    let mut i: i32 = 0;
    while i < 5 {
        sum = sum + prev_samples[i].voltage;
        i = i + 1;
    }
    filtered.voltage = sum / 6.0;
    
    // Quality check
    if sample.quality < 50 {
        filtered.quality = sample.quality / 2;
    }
    
    return filtered;
}

@medical_device(class=2)
@zero_heap
@wcet(300)
pub fn detect_r_peak(
    sample: ECGSample,
    prev_samples: [ECGSample; 10],
    threshold: f64
) -> bool {
    @requires(threshold > 0.0)
    @ensures(result == true implies sample.voltage > threshold)
    
    let current = sample.voltage;
    
    // R-peak detection: local maximum above threshold
    let mut is_max = true;
    let mut i: i32 = 0;
    while i < 10 {
        if prev_samples[i].voltage >= current {
            is_max = false;
        }
        i = i + 1;
    }
    
    return is_max && (current > threshold);
}

// ============================================================================
// ARRHYTHMIA DETECTION
// ============================================================================

@medical_device(class=2)
@zero_heap
@wcet(500)
pub fn detect_arrhythmia(
    beats: [HeartBeat; 8],
    num_beats: i32
) -> ArrhythmiaResult {
    @requires(num_beats > 0 && num_beats <= 8)
    @ensures(result.alarm_level >= 0 && result.alarm_level <= 2)
    @ensures(result.heart_rate >= 0.0)
    
    let mut result: ArrhythmiaResult;
    result.alarm_level = 0;
    result.arrhythmia_type = 0;
    result.confidence = 0.0;
    
    if num_beats < 2 {
        result.heart_rate = 0.0;
        return result;
    }
    
    // Calculate average heart rate
    let mut total_interval: i32 = 0;
    let mut i: i32 = 1;
    while i < num_beats {
        total_interval = total_interval + beats[i].interval_ms;
        i = i + 1;
    }
    
    let avg_interval = total_interval / (num_beats - 1);
    result.heart_rate = 60000.0 / (avg_interval as f64);
    
    // Detect arrhythmias
    if result.heart_rate < 60.0 {
        // Bradycardia
        result.alarm_level = 1;
        result.arrhythmia_type = 3;
        result.confidence = 0.85;
    } else if result.heart_rate > 100.0 {
        // Tachycardia  
        result.alarm_level = 1;
        result.arrhythmia_type = 4;
        result.confidence = 0.80;
    }
    
    // Check for irregular rhythm (AFib indicator)
    let mut irregularity = 0.0;
    i = 2;
    while i < num_beats {
        let diff = abs(beats[i].interval_ms - beats[i-1].interval_ms);
        if diff > 100 {  // > 100ms variation
            irregularity = irregularity + 1.0;
        }
        i = i + 1;
    }
    
    if irregularity > ((num_beats - 2) as f64) * 0.3 {
        result.alarm_level = 2;  // Critical - AFib suspected
        result.arrhythmia_type = 1;
        result.confidence = 0.75;
    }
    
    return result;
}

fn abs(x: i32) -> i32 {
    if x < 0 { return -x; } else { return x; }
}

// ============================================================================
// MAIN CONTROL LOOP
// ============================================================================

@medical_device(class=2)
@zero_heap
@wcet(1000)
pub fn ecg_main_loop(
    samples: [ECGSample; 100],
    num_samples: i32
) -> ArrhythmiaResult {
    @requires(num_samples > 0 && num_samples <= 100)
    
    let mut beats: [HeartBeat; 8];
    let mut beat_count: i32 = 0;
    let mut prev_samples: [ECGSample; 10];
    let mut i: i32 = 0;
    
    // Initialize prev_samples
    while i < 10 {
        prev_samples[i] = samples[0];
        i = i + 1;
    }
    
    i = 0;
    while i < num_samples {
        // Filter signal
        let filtered = filter_signal(samples[i], prev_samples);
        
        // Detect R-peaks
        if detect_r_peak(filtered, prev_samples, 0.5) {
            if beat_count < 8 {
                beats[beat_count].timestamp = filtered.timestamp;
                beats[beat_count].r_peak_amplitude = filtered.voltage;
                if beat_count > 0 {
                    beats[beat_count].interval_ms = filtered.timestamp - beats[beat_count - 1].timestamp;
                } else {
                    beats[beat_count].interval_ms = 0;
                }
                beat_count = beat_count + 1;
            }
        }
        
        // Update prev_samples
        let mut j: i32 = 0;
        while j < 9 {
            prev_samples[j] = prev_samples[j + 1];
            j = j + 1;
        }
        prev_samples[9] = filtered;
        
        i = i + 1;
    }
    
    // Analyze arrhythmia
    return detect_arrhythmia(beats, beat_count);
}

pub fn main() {
    // Demo: Simulate ECG processing
    let mut samples: [ECGSample; 100];
    
    // Initialize with sample data
    let mut i: i32 = 0;
    while i < 100 {
        samples[i].timestamp = i * 10;
        samples[i].voltage = 1.0;  // Normal baseline
        samples[i].lead = 0;
        samples[i].quality = 90;
        i = i + 1;
    }
    
    // Process
    let result = ecg_main_loop(samples, 100);
    
    println(result.heart_rate);
    println(result.alarm_level);
}
