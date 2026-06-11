#!/usr/bin/env python3
"""Generate test ECG data for FDA validation"""

import numpy as np
import json

def generate_normal_ecg(duration=10, fs=360):
    """Generate normal sinus rhythm"""
    t = np.arange(0, duration, 1/fs)
    # Simplified ECG model
    ecg = np.sin(2 * np.pi * 1.2 * t)  # Heart rate ~72 BPM
    return t, ecg

def generate_afib(duration=10, fs=360):
    """Generate atrial fibrillation pattern"""
    t = np.arange(0, duration, 1/fs)
    # Irregular rhythm
    ecg = np.sin(2 * np.pi * np.random.uniform(0.8, 2.0, len(t)) * t)
    return t, ecg

if __name__ == "__main__":
    # Generate test cases
    normal_t, normal_ecg = generate_normal_ecg()
    afib_t, afib_ecg = generate_afib()
    
    # Save as JSON for Zeus input
    test_data = {
        "normal": normal_ecg.tolist(),
        "afib": afib_ecg.tolist()
    }
    
    with open("test_data.json", "w") as f:
        json.dump(test_data, f)
    
    print("✅ Test data generated")
