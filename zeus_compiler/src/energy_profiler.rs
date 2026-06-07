use crate::ast::{Program, Statement, Expression};

pub struct EnergyProfiler {
    total_cost_mj: f64,
    warnings: Vec<String>,
}

impl EnergyProfiler {
    pub fn new() -> Self {
        Self {
            total_cost_mj: 0.0,
            warnings: Vec::new(),
        }
    }

    pub fn analyze_and_get_warnings(program: &Program) -> (f64, Vec<String>) {
        let mut profiler = EnergyProfiler::new();
        for stmt in &program.statements {
            profiler.total_cost_mj += Self::analyze_statement(stmt, &mut profiler.warnings);
        }
        (profiler.total_cost_mj, profiler.warnings)
    }

    pub fn analyze(program: &Program) -> f64 {
        let mut profiler = EnergyProfiler::new();
        for stmt in &program.statements {
            profiler.total_cost_mj += Self::analyze_statement(stmt, &mut profiler.warnings);
        }

        if !profiler.warnings.is_empty() {
            println!("\n\x1b[1;36m[ZEUS ENERGY PROFILER]\x1b[0m Analysis Complete.");
            for warning in &profiler.warnings {
                println!("\x1b[33m[WARNING]\x1b[0m {}", warning);
            }
            
            if profiler.total_cost_mj > 1000.0 {
                println!("\x1b[33m[RECOMMENDATION]\x1b[0m Consider vectorizing heavily nested or expensive scalar operations inside `parallel` blocks.");
            }
            println!("Total Estimated Base Energy Footprint: {:.2} mJ per execution run.\n", profiler.total_cost_mj);
        }
        
        profiler.total_cost_mj
    }

    fn analyze_statement(stmt: &Statement, warnings: &mut Vec<String>) -> f64 {
        match stmt {
            Statement::FunctionDeclaration { name, body, .. } => {
                let mut cost = 0.1; // base cost
                for s in body {
                    cost += Self::analyze_statement(s, warnings);
                }
                
                // If a function is extremely expensive, warn about it.
                if cost > 100.0 {
                    warnings.push(format!("[WARNING] Function `{}` is extremely energy-intensive (Estimated: {:.2} mJ).", name, cost));
                }
                cost
            }
            Statement::For { iterator: _, start, end, body } => {
                // Scalar loops are exactly what we want to discourage
                let loop_iterations = match (start, end) {
                    (Expression::Number(s), Expression::Number(e)) => e - s,
                    _ => 1000.0, // fallback heuristic
                };
                
                let mut body_cost = 0.0;
                for s in body {
                    body_cost += Self::analyze_statement(s, warnings);
                }
                
                let total_loop_cost = loop_iterations * (body_cost + 0.5); // 0.5 mJ overhead per iteration for branches
                
                warnings.push(format!("[WARNING] Scalar `for` loop detected with O(N) energy footprint (Estimated: {:.2} mJ per invocation).", total_loop_cost));
                warnings.push(format!("[RECOMMENDATION] Consider vectorizing this operation inside a `parallel` block."));
                
                total_loop_cost
            }
            Statement::ParallelBlock { statements, .. } => {
                let mut cost = 0.0;
                for s in statements {
                    cost += Self::analyze_statement(s, warnings);
                }
                // Parallel blocks are highly optimized, energy cost grows logarithmically or is fractionated.
                // We'll estimate it as 10% of the scalar cost plus a small startup penalty.
                (cost * 0.1) + 2.0
            }
            Statement::Let { value, .. } | Statement::ExpressionStatement(value) | Statement::Return(value) => {
                Self::analyze_expression(value)
            }
            _ => 0.1,
        }
    }

    fn analyze_expression(expr: &Expression) -> f64 {
        match expr {
            Expression::Infix { left, right, .. } => {
                0.2 + Self::analyze_expression(left) + Self::analyze_expression(right)
            }
            Expression::TensorDefinition { dimensions } => {
                let mut size = 1.0;
                for dim in dimensions {
                    if let Expression::Number(n) = dim {
                        size *= n;
                    }
                }
                // Memory allocation has an energy cost
                (size * 0.01).max(1.0)
            }
            _ => 0.1,
        }
    }
}
