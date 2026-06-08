#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    I8,
    I32,
    U64,
    F32,
    F64,
    Bool,
    Tensor { dimensions: Vec<Expression>, is_sparse: bool },
    Array(Box<Type>, Box<Expression>),
    Struct(String),
    Unknown(String),
    Result(Box<Type>, Box<Type>),
    Pointer(Box<Type>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
    Let {
        name: String,
        is_mut: bool,
        is_secret: bool,
        var_type: Option<Type>,
        value: Expression,
    },
    StructDeclaration {
        name: String,
        is_component: bool,
        fields: Vec<(String, Type)>,
    },
    FunctionDeclaration {
        is_pub: bool,
        name: String,
        parameters: Vec<(String, Type)>,
        return_type: Option<Type>,
        body: Vec<Statement>,
        attributes: Vec<FunctionAttribute>,
    },
    ExternFunctionDeclaration {
        name: String,
        parameters: Vec<(String, Type)>,
        return_type: Option<Type>,
    },
    TestDeclaration {
        name: String,
        body: Vec<Statement>,
    },
    For {
        iterator: String,
        start: Expression,
        end: Expression,
        body: Vec<Statement>,
    },
    Assert(Expression),
    Import(String),
    Return(Expression),
    ParallelBlock {
        iterator: String,
        start: Expression,
        end: Expression,
        statements: Vec<Statement>,
    },
    If {
        condition: Expression,
        consequence: Vec<Statement>,
        alternative: Option<Vec<Statement>>,
    },
    TargetBlock {
        targets: Vec<String>,
        statements: Vec<Statement>,
    },
    ProofBlock {
        statements: Vec<Statement>,
    },
    SafeStateBlock {
        statements: Vec<Statement>,
    },
    EnclaveBlock {
        statements: Vec<Statement>,
    },
    Panic(String),
    ExpressionStatement(Expression),
    LineDirective(usize),
    CfgBlock {
        arch: String,
        statements: Vec<Statement>,
    },
    ComptimeBlock {
        statements: Vec<Statement>,
    },
    ClusterBlock {
        statements: Vec<Statement>,
    },
    AtomicAdd {
        target: String,
        amount: String,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum FunctionAttribute {
    Verify(Expression, bool), // expression, has_timed_out
    Adaptive(String), // The raw parameter content for simplicity
    FfiExport,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Identifier(String),
    Number(f64),
    StringLiteral(String),
    
    Infix {
        left: Box<Expression>,
        operator: String,
        right: Box<Expression>,
    },
    
    TensorDefinition {
        dimensions: Vec<Expression>,
    },
    FunctionCall {
        name: String,
        arguments: Vec<Expression>,
    },
    StructInit {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    FieldAccess {
        base: Box<Expression>,
        field: String,
    },
    IndexAccess {
        base: Box<Expression>,
        index: Box<Expression>,
    },
    Try(Box<Expression>),
    Comptime(Box<Expression>),
    NvmeDmaMap {
        path: Box<Expression>,
        size: Box<Expression>,
    },
    OramAccess {
        base: Box<Expression>,
        index: Box<Expression>,
        bound: usize,
    },
}
