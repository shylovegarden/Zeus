use crate::ast::Program;

/// Represents a compiled artifact ready for execution or saving.
pub struct Artifact {
    pub raw_data: Vec<u8>,
}

pub enum CompileError {
    NotImplemented,
    OptimizationError(String),
    EmissionError(String),
}

/// The evolutionary hook. 
/// Any new hardware target in the future (QPU, Neural chips) just implements this trait.
pub trait Backend {
    /// Takes a lowered MLIR-like structure (or AST directly for the prototype)
    /// and compiles it down to hardware-specific machine code or C.
    fn compile(&self, program: &Program) -> Result<Artifact, CompileError>;
}

// Example default backend for the 10% rule
pub struct CTranspilerBackend;

impl Backend for CTranspilerBackend {
    fn compile(&self, _program: &Program) -> Result<Artifact, CompileError> {
        // Here we would traverse the AST/MLIR and generate the C code string
        Err(CompileError::NotImplemented)
    }
}
