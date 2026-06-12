use crate::ast::{Program, Statement, Expression};

pub struct FheLowering {
    enabled: bool,
}

impl FheLowering {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn lower_secrets(&mut self, program: &mut Program) {
        if !self.enabled { return; }
        println!("  -> [Native FHE] Lowering @secret variables to Homomorphic logic gates...");
        
        for stmt in &mut program.statements {
            self.lower_statement(stmt);
        }
    }

    fn lower_statement(&mut self, stmt: &mut Statement) {
        match stmt {
            Statement::FunctionDeclaration { body, .. } | Statement::While { body, .. } => {
                for b in body { self.lower_statement(b); }
            }
            Statement::If { consequence, alternative, .. } => {
                for b in consequence { self.lower_statement(b); }
                if let Some(alt) = alternative {
                    for b in alt { self.lower_statement(b); }
                }
            }
            Statement::Let { is_secret, value, .. } => {
                if *is_secret {
                    let original = std::mem::replace(value, Expression::Number(0.0));
                    *value = Expression::HomomorphicGate(Box::new(original));
                }
            }
            _ => {}
        }
    }
}
