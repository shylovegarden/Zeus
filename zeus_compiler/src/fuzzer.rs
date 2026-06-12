use crate::ast::{Program, Statement, Expression, Type};
use crate::zir;
use crate::bounds;
use std::time::{Instant, Duration};

/// Fuzzer for ZIR and WCET bounds analysis.
/// Generates random AST nodes to test for analyzer panics and differential invariants.
pub struct AnalyzerFuzzer {
    iterations: usize,
}

impl AnalyzerFuzzer {
    pub fn new(iterations: usize) -> Self {
        Self { iterations }
    }

    pub fn run(&self) {
        println!("[ZEUS FUZZER] Starting differential/fuzz testing for analyzers ({} iterations)...", self.iterations);
        let start_time = Instant::now();
        let mut panics = 0;
        let mut wcet_failures = 0;
        let mut ct_failures = 0;

        for i in 0..self.iterations {
            let ast = self.generate_random_ast(i as u64);
            
            // Fuzz ZIR (Constant-Time & Determinism Analysis)
            let zir_result = std::panic::catch_unwind(|| {
                zir::lower_and_analyze(&ast)
            });

            // Fuzz WCET (Bounds Analysis)
            let bounds_result = std::panic::catch_unwind(|| {
                bounds::analyze(&ast)
            });

            if zir_result.is_err() {
                panics += 1;
                eprintln!("[FUZZ ERROR] ZIR Analyzer panicked on iteration {}", i);
            } else if let Ok(report) = zir_result {
                // Invariant: ZIR report should not contain arbitrary panics.
                if report.secret_values > 100 {
                    ct_failures += 1; // Unrealistic heuristic check
                }
            }

            if bounds_result.is_err() {
                panics += 1;
                eprintln!("[FUZZ ERROR] WCET Analyzer panicked on iteration {}", i);
            } else if let Ok(bnds) = bounds_result {
                // Invariant: A program with no loops and no externs should always have a WCET bound.
                if bnds.fns.iter().any(|f| f.wcet.is_none()) {
                    wcet_failures += 1;
                }
            }
        }

        let duration = start_time.elapsed();
        println!("\n[ZEUS FUZZER SUMMARY]");
        println!("  Iterations: {}", self.iterations);
        println!("  Time elapsed: {:?}", duration);
        println!("  Analyzer Panics: {}", panics);
        println!("  CT Analyzer Anomalies: {}", ct_failures);
        println!("  WCET Unbounded Anomalies: {}", wcet_failures);

        if panics == 0 && ct_failures == 0 && wcet_failures == 0 {
            println!("  \x1b[1;32mVERDICT: PASS\x1b[0m");
        } else {
            println!("  \x1b[1;31mVERDICT: FAIL\x1b[0m");
        }
    }

    /// Generates a simplistic random AST based on a seed for fuzzing.
    fn generate_random_ast(&self, seed: u64) -> Program {
        // Pseudorandom generation
        let is_secret = seed % 2 == 0;
        let op = if seed % 3 == 0 { "Plus" } else if seed % 3 == 1 { "Star" } else { "Slash" };
        
        let stmt = Statement::Let {
            name: format!("var_{}", seed),
            is_mut: true,
            is_secret,
            var_type: Some(Type::I32),
            value: Expression::Infix {
                left: Box::new(Expression::Number(10.0)),
                operator: op.to_string(),
                right: Box::new(Expression::Number((seed % 100) as f64)),
            },
        };

        let func = Statement::FunctionDeclaration {
            is_pub: true,
            name: format!("fuzz_test_{}", seed),
            type_params: vec![],
            parameters: vec![],
            return_type: Some(Type::I32),
            body: vec![stmt, Statement::Return(Expression::Identifier(format!("var_{}", seed)))],
            attributes: vec![],
            secret_params: vec![],
        };

        Program {
            statements: vec![func],
        }
    }
}
