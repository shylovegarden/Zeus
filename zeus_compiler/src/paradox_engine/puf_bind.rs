use crate::ast::{Program, Statement, Expression};

pub struct PufBinder {
    enabled: bool,
}

impl PufBinder {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn bind(&mut self, program: &mut Program) {
        if !self.enabled { return; }
        println!("  -> [PUF Binding] Injecting Silicon-Entangled Z3 Contracts into AST...");
        
        // Find the main function or the first function to inject the PUF check
        for stmt in &mut program.statements {
            if let Statement::FunctionDeclaration { name, body, .. } = stmt {
                if name == "main" || !body.is_empty() {
                    let puf_check = Statement::Expression(Expression::HardwareEntanglement("PUF_SEED_VERIFY".to_string()));
                    body.insert(0, puf_check);
                    break;
                }
            }
        }
    }
}
