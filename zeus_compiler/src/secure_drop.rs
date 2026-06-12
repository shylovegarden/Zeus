use crate::ast::{Program, Statement};

pub struct SecureDropPass;

impl SecureDropPass {
    pub fn new() -> Self {
        Self
    }

    pub fn inject(&self, program: &mut Program) {
        println!("  -> [Secure Drop] Injecting volatile memory wipes for `secret` variables...");
        for stmt in &mut program.statements {
            self.inject_stmt(stmt);
        }
    }

    fn inject_stmt(&self, stmt: &mut Statement) {
        match stmt {
            Statement::FunctionDeclaration { body, .. }
            | Statement::While { body, .. }
            | Statement::For { body, .. } => {
                let mut new_body = Vec::new();
                let mut secrets_in_scope = Vec::new();
                for mut b in std::mem::take(body) {
                    if let Statement::Let { name, is_secret: true, .. } = &b {
                        secrets_in_scope.push(name.clone());
                    }
                    self.inject_stmt(&mut b);
                    new_body.push(b);
                }
                for s in secrets_in_scope {
                    new_body.push(Statement::SecureWipe(s));
                }
                *body = new_body;
            }
            Statement::If { consequence, alternative, .. } => {
                let mut new_cons = Vec::new();
                let mut secrets_in_cons = Vec::new();
                for mut b in std::mem::take(consequence) {
                    if let Statement::Let { name, is_secret: true, .. } = &b {
                        secrets_in_cons.push(name.clone());
                    }
                    self.inject_stmt(&mut b);
                    new_cons.push(b);
                }
                for s in secrets_in_cons {
                    new_cons.push(Statement::SecureWipe(s));
                }
                *consequence = new_cons;

                if let Some(alt) = alternative {
                    let mut new_alt = Vec::new();
                    let mut secrets_in_alt = Vec::new();
                    for mut b in std::mem::take(alt) {
                        if let Statement::Let { name, is_secret: true, .. } = &b {
                            secrets_in_alt.push(name.clone());
                        }
                        self.inject_stmt(&mut b);
                        new_alt.push(b);
                    }
                    for s in secrets_in_alt {
                        new_alt.push(Statement::SecureWipe(s));
                    }
                    *alt = new_alt;
                }
            }
            Statement::MatchStatement { arms, .. } => {
                for arm in arms {
                    let mut new_body = Vec::new();
                    let mut secrets_in_arm = Vec::new();
                    for mut b in std::mem::take(&mut arm.body) {
                        if let Statement::Let { name, is_secret: true, .. } = &b {
                            secrets_in_arm.push(name.clone());
                        }
                        self.inject_stmt(&mut b);
                        new_body.push(b);
                    }
                    for s in secrets_in_arm {
                        new_body.push(Statement::SecureWipe(s));
                    }
                    arm.body = new_body;
                }
            }
            _ => {}
        }
    }
}
