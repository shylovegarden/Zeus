#![allow(clippy::enum_variant_names)]
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
    /// A generic type parameter, e.g. `T` in `fn foo<T>(x: T) -> T`.
    TypeParam(String),
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
        /// Generic type parameters, e.g. `["T", "E"]` for `struct Foo<T, E>`.
        type_params: Vec<String>,
    },
    FunctionDeclaration {
        is_pub: bool,
        name: String,
        /// Generic type parameters, e.g. `["T"]` for `fn foo<T>`.
        type_params: Vec<String>,
        parameters: Vec<(String, Type)>,
        secret_params: Vec<String>,
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
    While {
        condition: Expression,
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
    /// `enum Color { Red, Green, Rgb(i32, i32, i32) }`
    EnumDeclaration {
        name: String,
        variants: Vec<EnumVariantDef>,
    },
    /// `match expr { ... }`
    MatchStatement {
        scrutinee: Expression,
        arms: Vec<MatchArm>,
    },
}

/// One variant in an enum declaration
#[derive(Debug, PartialEq, Clone)]
pub struct EnumVariantDef {
    pub name: String,
    /// None = unit variant, Some(types) = tuple variant
    pub payload: Option<Vec<Type>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum FunctionAttribute {
    Verify(Expression, bool), // expression, has_timed_out
    Requires(Expression, bool), // precondition (runtime-checked at entry)
    Ensures(Expression, bool),  // postcondition (may reference reserved `result`)
    Adaptive(String), // The raw parameter content for simplicity
    FfiExport,
    Wcet(u64),  // @wcet(N): declared worst-case execution-time bound (steps)
    Stack(u64), // @stack(N): declared stack-size bound (bytes)
    ConstantTime, // @constant_time: function must have no secret-dependent timing channel
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Identifier(String),
    Number(f64),
    StringLiteral(String),
    HomomorphicGate(Box<Expression>),
    HardwareEntanglement(String),
    
    Infix {
        left: Box<Expression>,
        operator: String,
        right: Box<Expression>,
    },

    Prefix {
        operator: String,
        operand: Box<Expression>,
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
    ArrayLiteral(Vec<Expression>),
    /// Enum variant constructor: `Color::Red` or `Color::Rgb(1, 2, 3)`
    EnumVariant {
        enum_name: String,
        variant: String,
        payload: Vec<Expression>,
    },
    /// Match expression
    MatchExpr {
        scrutinee: Box<Expression>,
        arms: Vec<MatchArm>,
    },
}

/// One arm of a match: `Pattern => { body }`
#[derive(Debug, PartialEq, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Vec<Statement>,
}

/// Patterns supported in match arms
#[derive(Debug, PartialEq, Clone)]
pub enum MatchPattern {
    /// `Color::Red` — unit variant, no bindings
    Variant { enum_name: String, variant: String },
    /// `Color::Rgb(r, g, b)` — tuple variant with binding names
    VariantTuple { enum_name: String, variant: String, bindings: Vec<String> },
    /// `_` — wildcard
    Wildcard,
    /// literal number
    Literal(f64),
}
