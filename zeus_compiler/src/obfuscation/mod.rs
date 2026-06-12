pub mod vmp;
pub mod runtime_guard;

use crate::ast::{Program, Statement};

/// Applies AI-Resistant Obfuscation to the AST
pub struct ObfuscationEngine {
    enabled: bool,
}

impl ObfuscationEngine {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn obfuscate(&self, program: &mut Program) {
        if !self.enabled {
            return;
        }
        println!("\x1b[1;35m[ZEUS DRM]\x1b[0m Applying AST-level Control Flow Flattening and Opaque Predicates...");
        for stmt in &mut program.statements {
            self.obfuscate_statement(stmt);
        }
    }

    fn obfuscate_statement(&self, stmt: &mut Statement) {
        if let Statement::FunctionDeclaration { body, .. } = stmt {
            if body.is_empty() { return; }

            // 1. Inject Opaque Predicate
            // if (7 * 7 == 49) { <original body> } else { <dead code> }
            let opaque_condition = Expression::Infix {
                left: Box::new(Expression::Infix {
                    left: Box::new(Expression::Number(7.0)),
                    operator: "Star".to_string(),
                    right: Box::new(Expression::Number(7.0)),
                }),
                operator: "Equal".to_string(),
                right: Box::new(Expression::Number(49.0)),
            };

            let dead_code = vec![Statement::Return(Expression::Number(-1.0))];

            let original_body = std::mem::take(body);
            
            // 2. Control Flow Flattening
            // Wrap the original body inside a state machine:
            // let mut __zeus_state = 0;
            // while (__zeus_state < 1) { 
            //    __zeus_state = 1; 
            //    <original body>
            // }
            let state_var = "__zeus_state".to_string();
            let let_state = Statement::Let {
                name: state_var.clone(),
                is_mut: true,
                is_secret: false,
                var_type: Some(crate::ast::Type::I32),
                value: Expression::Number(0.0),
            };

            let set_state = Statement::Let {
                name: state_var.clone(),
                is_mut: true,
                is_secret: false,
                var_type: Some(crate::ast::Type::I32),
                value: Expression::Number(1.0),
            };

            let mut flat_body = vec![set_state];
            flat_body.extend(original_body);

            let while_loop = Statement::While {
                condition: Expression::Infix {
                    left: Box::new(Expression::Identifier(state_var.clone())),
                    operator: "LessThan".to_string(),
                    right: Box::new(Expression::Number(1.0)),
                },
                body: flat_body,
            };

            let opaque_if = Statement::If {
                condition: opaque_condition,
                consequence: vec![let_state, while_loop],
                alternative: Some(dead_code),
            };

            *body = vec![opaque_if];
            println!("  -> Injected Opaque Predicate and Flattened Control Flow");
        }
    }
}
