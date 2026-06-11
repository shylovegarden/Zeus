#!/usr/bin/env python3
"""HIL (Hardware-in-Loop) simulation for attitude control"""

import numpy as np

class SatelliteSimulator:
    def __init__(self):
        self.attitude = np.array([1.0, 0.0, 0.0, 0.0])  # Quaternion
        self.rate = np.array([0.01, 0.02, 0.01])  # rad/s
        
    def step(self, torque, dt=0.1):
        """Simulate one control cycle"""
        # Simplified dynamics
        self.rate += torque * dt
        # Update attitude (simplified)
        self.attitude += np.random.normal(0, 0.001, 4)
        self.attitude /= np.linalg.norm(self.attitude)
        
    def get_sensor_data(self):
        """Return simulated sensor readings"""
        return {
            "gyro": self.rate + np.random.normal(0, 0.001, 3),
            "sun": np.array([1.0, 0.1, 0.1]) + np.random.normal(0, 0.01, 3),
            "mag": np.array([0.1, 0.1, 1.0]) + np.random.normal(0, 0.01, 3)
        }

if __name__ == "__main__":
    sim = SatelliteSimulator()
    
    # Run 100 control cycles
    for i in range(100):
        sensors = sim.get_sensor_data()
        # In real test, Zeus control would be called here
        torque = np.array([0.001, 0.002, 0.001])
        sim.step(torque)
    
    print(f"✅ Simulation complete: attitude = {sim.attitude}")
