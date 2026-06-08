use std::fs;

#[derive(Debug, Clone)]
pub struct HardwareBlueprint {
    pub arch_name: String,
    pub register_count: usize,
    pub simd_width: usize,
    pub is_quantum: bool,
    pub compiler_flags: Vec<String>,
}

impl HardwareBlueprint {
    pub fn load_from_file(path: &str) -> Option<HardwareBlueprint> {
        let content = fs::read_to_string(path).ok()?;
        let mut arch_name = String::new();
        let mut register_count = 16;
        let mut simd_width = 128;
        let mut is_quantum = false;
        let mut compiler_flags = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") { continue; }
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 { continue; }
            let key = parts[0].trim();
            let val = parts[1].trim();

            match key {
                "ARCH_NAME" => arch_name = val.to_string(),
                "REGISTER_COUNT" => register_count = val.parse().unwrap_or(16),
                "SIMD_WIDTH" => simd_width = val.parse().unwrap_or(128),
                "QUANTUM_CAPABLE" => is_quantum = val == "true",
                "FLAGS" => compiler_flags = val.split(',').map(|s| s.trim().to_string()).collect(),
                _ => {}
            }
        }

        Some(HardwareBlueprint {
            arch_name,
            register_count,
            simd_width,
            is_quantum,
            compiler_flags,
        })
    }
}
