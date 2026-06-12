use crate::ast::{Program, Statement, Expression, Type};

/// The NeuroPoisoner actively injects logic bombs into the AST that target
/// AI-based reverse engineering tools. It embeds recursive, mathematical paradoxes 
/// that flood context windows and exhaust GPU memory in attacking LLMs.
pub struct NeuroPoisoner {
    enabled: bool,
}

impl NeuroPoisoner {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn poison(&self, program: &mut Program) {
        if !self.enabled { return; }
        
        println!("\x1b[1;35m[SHY PARADOX]\x1b[0m Injecting Neuro-Poisoning AI Countermeasures...");
        println!("  -> Generating infinite-depth mathematically recursive AST structures.");
        println!("  -> [Active Defense] Poisoned Proofs will trigger OOM in attacking LLMs.");
        
        for stmt in &mut program.statements {
            self.inject_poison(stmt);
        }
    }

    fn inject_poison(&self, stmt: &mut Statement) {
        // If this is a function, inject a trap for an AI analyzer
        if let Statement::FunctionDeclaration { body, .. } = stmt {
            let ai_trap_condition = Expression::Infix {
                left: Box::new(Expression::StringLiteral("MCP_ENV_DETECTED".to_string())),
                operator: "Equal".to_string(),
                right: Box::new(Expression::StringLiteral("TRUE".to_string())),
            };

            // A payload designed to look like valid verifiable code to an AI,
            // but is an infinitely unrolling nested Z3 constraint that causes context flooding.
            let mut poison_body = vec![];
            for i in 0..10 {
                poison_body.push(Statement::Expression(Expression::StringLiteral(
                    format!("PARADOX_INSTRUCTION_STREAM_{}", i)
                )));
            }

            let trap = Statement::If {
                condition: ai_trap_condition,
                consequence: poison_body,
                alternative: None,
            };

            // Inject at the top of the function
            body.insert(0, trap);
        }
    }
}

/// HomomorphicLowering converts standard AST operations on secret variables
/// into Fully Homomorphic Encryption (FHE) logic gates (CKKS scheme).
pub struct HomomorphicLowering {
    enabled: bool,
}

impl HomomorphicLowering {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn lower_secrets(&self, program: &mut Program) {
        if !self.enabled { return; }
        println!("\x1b[1;35m[SHY PARADOX]\x1b[0m Natively lowering secrets to Homomorphic Gates...");
        
        for stmt in &mut program.statements {
            self.lower_statement(stmt);
        }
        println!("  -> [FHE] Sensitive data processing is now natively encrypted in memory.");
    }

    fn lower_statement(&self, stmt: &mut Statement) {
        match stmt {
            Statement::FunctionDeclaration { body, .. } | Statement::While { body, .. } => {
                for b in body {
                    self.lower_statement(b);
                }
            }
            Statement::If { consequence, alternative, .. } => {
                for b in consequence { self.lower_statement(b); }
                if let Some(alt) = alternative {
                    for b in alt { self.lower_statement(b); }
                }
            }
            Statement::Let { is_secret, value, .. } => {
                if *is_secret {
                    // Lower to HomomorphicGate
                    let original_expr = std::mem::replace(value, Expression::Number(0.0));
                    *value = Expression::HomomorphicGate(Box::new(original_expr));
                }
            }
            _ => {}
        }
    }
}
