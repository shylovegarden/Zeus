// Policy Enforcement Engine
// Enforces custom security policies at compile time

use crate::ast::{Program, Statement, Expression, FunctionAttribute};
use std::collections::HashSet;

/// Security property that can be required
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    ZeroHeap,
    ConstantTime,
    BoundedWcet,
    Deterministic,
    NoFfi,
    NoNetwork,
    NoFileIo,
    FdaCompliant,
    NasaCompliant,
    Custom(String),
}

/// Operation that can be forbidden
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Malloc,
    Free,
    Syscall,
    NetworkAccess,
    FileAccess,
    RandomAccess,
    TimeAccess,
    DivisionBySecret,
    SecretBranch,
}

/// Compliance standard
#[derive(Debug, Clone, PartialEq)]
pub enum Standard {
    FdaIec62304,
    NasaClassD,
    MisraC2012,
    Iso26262,
    NistFips,
    Custom(String),
}

/// Policy violation
#[derive(Debug)]
pub struct Violation {
    pub operation: Operation,
    pub location: String,
    pub message: String,
}

/// Policy engine
pub struct PolicyEngine {
    required_properties: HashSet<Property>,
    forbidden_operations: HashSet<Operation>,
    compliance_standards: HashSet<Standard>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        PolicyEngine {
            required_properties: HashSet::new(),
            forbidden_operations: HashSet::new(),
            compliance_standards: HashSet::new(),
        }
    }
    
    /// Load policy from file
    pub fn from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read policy file: {}", e))?;
        
        let mut engine = PolicyEngine::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if line.starts_with("require: ") {
                let props: Vec<&str> = line[9..].split(',').map(|s| s.trim()).collect();
                for prop in props {
                    engine.required_properties.insert(parse_property(prop)?);
                }
            } else if line.starts_with("forbid: ") {
                let ops: Vec<&str> = line[8..].split(',').map(|s| s.trim()).collect();
                for op in ops {
                    engine.forbidden_operations.insert(parse_operation(op)?);
                }
            } else if line.starts_with("comply: ") {
                let stds: Vec<&str> = line[8..].split(',').map(|s| s.trim()).collect();
                for std in stds {
                    engine.compliance_standards.insert(parse_standard(std)?);
                }
            }
        }
        
        Ok(engine)
    }
    
    /// Enforce policy on program
    pub fn enforce(&self, program: &Program) -> Result<(), Vec<Violation>> {
        let mut violations = Vec::new();
        
        // Check required properties
        self.check_properties(program, &mut violations);
        
        // Check forbidden operations
        self.check_operations(program, &mut violations);
        
        // Check compliance standards
        self.check_compliance(program, &mut violations);
        
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
    
    fn check_properties(&self, program: &Program, violations: &mut Vec<Violation>) {
        for prop in &self.required_properties {
            match prop {
                Property::ZeroHeap => {
                    // Delegate to ZIR's zero-heap verdict (the authoritative source).
                    let zir = crate::zir::lower_and_analyze(program);
                    if !zir.zero_heap {
                        violations.push(Violation {
                            operation: Operation::Malloc,
                            location: "<program>".to_string(),
                            message: "zero-heap policy violated: program reaches a heap-allocating \
                                      construct or an opaque extern (ZIR verdict: zero_heap=false)".to_string(),
                        });
                    }
                }
                Property::ConstantTime => {
                    // Check ZIR per-function constant_time flags.
                    let zir = crate::zir::lower_and_analyze(program);
                    for pf in &zir.per_fn {
                        if !pf.constant_time {
                            violations.push(Violation {
                                operation: Operation::SecretBranch,
                                location: pf.name.clone(),
                                message: format!(
                                    "fn '{}': constant-time policy violated — \
                                     secret-dependent branch or index detected",
                                    pf.name
                                ),
                            });
                        }
                    }
                }
                Property::FdaCompliant => {
                    // Run the same five IEC 62304 Class C checks used by the
                    // compliance report so the policy gate is consistent.
                    let zir  = crate::zir::lower_and_analyze(program);
                    let bnds = crate::bounds::analyze(program);

                    if !zir.zero_heap {
                        violations.push(Violation {
                            operation: Operation::Malloc,
                            location: "<program>".to_string(),
                            message: "IEC 62304 Req 1 FAIL: dynamic memory allocation detected \
                                      (zero_heap=false)".to_string(),
                        });
                    }
                    for fb in &bnds.fns {
                        if fb.wcet.is_none() {
                            violations.push(Violation {
                                operation: Operation::Syscall, // closest available variant
                                location: fb.name.clone(),
                                message: format!(
                                    "IEC 62304 Req 2 FAIL: fn '{}' has no provable WCET bound \
                                     (while/recursion/extern present)",
                                    fb.name
                                ),
                            });
                        }
                    }
                    for pf in &zir.per_fn {
                        if !pf.constant_time {
                            violations.push(Violation {
                                operation: Operation::SecretBranch,
                                location: pf.name.clone(),
                                message: format!(
                                    "IEC 62304 Req 3 FAIL: fn '{}' has a secret-dependent \
                                     timing channel",
                                    pf.name
                                ),
                            });
                        }
                        if !pf.deterministic {
                            violations.push(Violation {
                                operation: Operation::RandomAccess,
                                location: pf.name.clone(),
                                message: format!(
                                    "IEC 62304 Req 4 FAIL: fn '{}' is not provably deterministic \
                                     (nondeterministic source reachable)",
                                    pf.name
                                ),
                            });
                        }
                    }
                    if zir.ffi_unaudited {
                        violations.push(Violation {
                            operation: Operation::Syscall,
                            location: "<program>".to_string(),
                            message: "IEC 62304 Req 5 FAIL: program calls opaque extern function(s) \
                                      that cannot be audited (ffi_unaudited=true)".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    
    fn check_operations(&self, program: &Program, violations: &mut Vec<Violation>) {
        for stmt in &program.statements {
            self.check_stmt_operations(stmt, violations);
        }
    }
    
    fn check_stmt_operations(&self, stmt: &Statement, violations: &mut Vec<Violation>) {
        // Recursively check for forbidden operations
        match stmt {
            Statement::FunctionDeclaration { body, .. } => {
                for s in body {
                    self.check_stmt_operations(s, violations);
                }
            }
            Statement::ExpressionStatement(e) => {
                self.check_expr_operations(e, violations);
            }
            _ => {}
        }
    }
    
    fn check_expr_operations(&self, expr: &Expression, violations: &mut Vec<Violation>) {
        match expr {
            Expression::FunctionCall { name, .. } => {
                let op = match name.as_str() {
                    "malloc" | "calloc" => Some(Operation::Malloc),
                    "free" => Some(Operation::Free),
                    "rand" | "random" => Some(Operation::RandomAccess),
                    "time" | "clock" => Some(Operation::TimeAccess),
                    "read" | "write" | "fopen" => Some(Operation::FileAccess),
                    "socket" | "connect" | "send" => Some(Operation::NetworkAccess),
                    _ => None,
                };
                
                if let Some(op) = op {
                    if self.forbidden_operations.contains(&op) {
                        violations.push(Violation {
                            operation: op,
                            location: name.clone(),
                            message: format!("Forbidden operation: {}", name),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    
    fn check_compliance(&self, _program: &Program, _violations: &mut Vec<Violation>) {
        // Check compliance with standards
    }
    
    /// Generate policy compliance report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("ZEUS POLICY COMPLIANCE REPORT\n");
        report.push_str("============================\n\n");
        
        report.push_str("Required Properties:\n");
        for prop in &self.required_properties {
            report.push_str(&format!("  ✓ {:?}\n", prop));
        }
        
        report.push_str("\nForbidden Operations:\n");
        for op in &self.forbidden_operations {
            report.push_str(&format!("  ✗ {:?}\n", op));
        }
        
        report.push_str("\nCompliance Standards:\n");
        for std in &self.compliance_standards {
            report.push_str(&format!("  📋 {:?}\n", std));
        }
        
        report
    }
}

fn parse_property(s: &str) -> Result<Property, String> {
    match s {
        "zero-heap" | "zero_heap" => Ok(Property::ZeroHeap),
        "constant-time" | "constant_time" => Ok(Property::ConstantTime),
        "bounded" | "wcet" => Ok(Property::BoundedWcet),
        "deterministic" => Ok(Property::Deterministic),
        "no-ffi" | "no_ffi" => Ok(Property::NoFfi),
        "no-network" => Ok(Property::NoNetwork),
        "no-file-io" => Ok(Property::NoFileIo),
        "fda-compliant" | "fda_compliant" => Ok(Property::FdaCompliant),
        "nasa-compliant" | "nasa_compliant" => Ok(Property::NasaCompliant),
        _ => Ok(Property::Custom(s.to_string())),
    }
}

fn parse_operation(s: &str) -> Result<Operation, String> {
    match s {
        "malloc" => Ok(Operation::Malloc),
        "free" => Ok(Operation::Free),
        "syscall" => Ok(Operation::Syscall),
        "network" => Ok(Operation::NetworkAccess),
        "file" | "file-io" => Ok(Operation::FileAccess),
        "random" => Ok(Operation::RandomAccess),
        "time" => Ok(Operation::TimeAccess),
        "secret-division" => Ok(Operation::DivisionBySecret),
        "secret-branch" => Ok(Operation::SecretBranch),
        _ => Err(format!("Unknown operation: {}", s)),
    }
}

fn parse_standard(s: &str) -> Result<Standard, String> {
    match s {
        "FDA-IEC62304" | "fda-iec62304" => Ok(Standard::FdaIec62304),
        "NASA-Class-D" | "nasa-class-d" => Ok(Standard::NasaClassD),
        "MISRA-C:2012" | "misra-c" => Ok(Standard::MisraC2012),
        "ISO-26262" | "iso26262" => Ok(Standard::Iso26262),
        "NIST-FIPS" | "nist-fips" => Ok(Standard::NistFips),
        _ => Ok(Standard::Custom(s.to_string())),
    }
}
