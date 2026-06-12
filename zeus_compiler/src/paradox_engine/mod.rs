pub mod semantic_opacity;
pub mod fhe_lower;
pub mod puf_bind;

use crate::ast::Program;

pub struct ParadoxEngine {
    enabled: bool,
}

impl ParadoxEngine {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn run_passes(&self, program: &mut Program) {
        if !self.enabled {
            return;
        }

        println!("\x1b[1;36m[SHY PARADOX ENGINE]\x1b[0m Engaging Defensive Posture...");
        
        let mut opacity = semantic_opacity::SemanticOpacity::new(self.enabled);
        opacity.obfuscate(program);

        let mut fhe = fhe_lower::FheLowering::new(self.enabled);
        fhe.lower_secrets(program);

        let mut puf = puf_bind::PufBinder::new(self.enabled);
        puf.bind(program);
    }
}
