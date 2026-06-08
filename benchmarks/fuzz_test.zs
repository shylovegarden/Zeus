@adaptive(0.5)
pub fn main() {
    println("Initializing Zeus Micro AI Inference test.");

    // The @adaptive block activates the AI model. 
    // If run without --tune, it will use the static weights [0.25, -0.5, 0.8, -0.1]
    // If run with --tune, the compiler trains and injects the new weights [0.85, -0.12, 0.99, -0.05]
    
    println("Anomaly checked completed without triggering formal limp mode.");
}
