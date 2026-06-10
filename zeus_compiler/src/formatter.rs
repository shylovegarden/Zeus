use crate::ast::{Expression, Program, Statement, Type};

pub struct Formatter {
    indent_level: usize,
    indent_size: usize,
}

impl Formatter {
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            indent_size: 4,
        }
    }

    pub fn format(program: &Program) -> String {
        let mut formatter = Self::new();
        formatter.format_program(program)
    }

    fn indent(&self) -> String {
        " ".repeat(self.indent_level * self.indent_size)
    }

    fn format_program(&mut self, program: &Program) -> String {
        let mut result = String::new();
        for (i, stmt) in program.statements.iter().enumerate() {
            result.push_str(&self.format_statement(stmt));
            if i < program.statements.len() - 1 {
                result.push_str("\n");
            }
        }
        result
    }

    fn format_statement(&mut self, stmt: &Statement) -> String {
        match stmt {
            Statement::Let { name, is_mut, is_secret, value, var_type: _ } => {
                let mut_str = if *is_mut { "mut " } else { "" };
                let sec_str = if *is_secret { "secret " } else { "" };
                format!(
                    "{}let {}{}{} = {};\n",
                    self.indent(),
                    mut_str,
                    sec_str,
                    name,
                    self.format_expression(value)
                )
            }
            Statement::StructDeclaration { name, is_component, fields } => {
                let comp_str = if *is_component { "component struct " } else { "struct " };
                let mut result = format!("{}{}{} {{\n", self.indent(), comp_str, name);
                self.indent_level += 1;
                for (field_name, field_type) in fields {
                    result.push_str(&format!(
                        "{}{}: {},\n",
                        self.indent(),
                        field_name,
                        self.format_type(field_type)
                    ));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::FunctionDeclaration {
                is_pub,
                name,
                parameters,
                return_type,
                body,
                attributes: _,
                secret_params: _,
            } => {
                let pub_str = if *is_pub { "pub " } else { "" };
                let params: Vec<String> = parameters
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, self.format_type(t)))
                    .collect();
                let ret_str = match return_type {
                    Some(t) => format!(" -> {}", self.format_type(t)),
                    None => "".to_string(),
                };
                let mut result = format!(
                    "{}{}fn {}({}){} {{\n",
                    self.indent(),
                    pub_str,
                    name,
                    params.join(", "),
                    ret_str
                );
                self.indent_level += 1;
                for b_stmt in body {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::ExternFunctionDeclaration {
                name,
                parameters,
                return_type,
            } => {
                let params: Vec<String> = parameters
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, self.format_type(t)))
                    .collect();
                let ret_str = match return_type {
                    Some(t) => format!(" -> {}", self.format_type(t)),
                    None => "".to_string(),
                };
                format!(
                    "{}extern fn {}({}){};\n",
                    self.indent(),
                    name,
                    params.join(", "),
                    ret_str
                )
            }
            Statement::While { condition, body } => {
                let mut result = format!("{}while ({}) {{\n", self.indent(), self.format_expression(condition));
                self.indent_level += 1;
                for b_stmt in body {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::For {
                iterator,
                start,
                end,
                body,
            } => {
                let mut result = format!(
                    "{}for {} in {}..{} {{\n",
                    self.indent(),
                    iterator,
                    self.format_expression(start),
                    self.format_expression(end)
                );
                self.indent_level += 1;
                for b_stmt in body {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::Assert(expr) => {
                format!("{}assert({});\n", self.indent(), self.format_expression(expr))
            }
            Statement::Import(path) => {
                format!("{}import {};\n", self.indent(), path)
            }
            Statement::Return(expr) => {
                format!("{}return {};\n", self.indent(), self.format_expression(expr))
            }
            Statement::ParallelBlock { iterator, start, end, statements } => {
                let mut result = format!(
                    "{}parallel ({} in {}..{}) {{\n",
                    self.indent(),
                    iterator,
                    self.format_expression(start),
                    self.format_expression(end)
                );
                self.indent_level += 1;
                for b_stmt in statements {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::TargetBlock { targets, statements } => {
                let mut result = format!("{}target {} {{\n", self.indent(), targets.join(", "));
                self.indent_level += 1;
                for b_stmt in statements {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::ProofBlock { statements } => {
                let mut result = format!("{}proof {{\n", self.indent());
                self.indent_level += 1;
                for b_stmt in statements {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::SafeStateBlock { statements } => {
                let mut result = format!("{}safestate {{\n", self.indent());
                self.indent_level += 1;
                for b_stmt in statements {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::EnclaveBlock { statements } => {
                let mut result = format!("{}enclave {{\n", self.indent());
                self.indent_level += 1;
                for b_stmt in statements {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::TestDeclaration { name, body } => {
                let mut result = format!("{}test fn {}() {{\n", self.indent(), name);
                self.indent_level += 1;
                for b_stmt in body {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::ExpressionStatement(expr) => {
                format!("{}{};\n", self.indent(), self.format_expression(expr))
            }
            Statement::Panic(msg) => {
                format!("{}panic \"{}\";\n", self.indent(), msg)
            }
            Statement::If { condition, consequence, alternative } => {
                let mut out = format!("{}if {} {{\n", self.indent(), self.format_expression(condition));
                self.indent_level += 1;
                for s in consequence {
                    out.push_str(&self.format_statement(s));
                }
                self.indent_level -= 1;
                if let Some(alt) = alternative {
                    out.push_str(&format!("{}}} else {{\n", self.indent()));
                    self.indent_level += 1;
                    for s in alt {
                        out.push_str(&self.format_statement(s));
                    }
                    self.indent_level -= 1;
                }
                out.push_str(&format!("{}}}\n", self.indent()));
                out
            }
            Statement::AtomicAdd { target, amount } => {
                format!("{}@atomic_add({}, {});\n", self.indent(), target, amount)
            }
            Statement::LineDirective(_) => String::new(),
            Statement::CfgBlock { arch, statements } => {
                let mut result = format!(
                    "{}@cfg(target_arch = \"{}\") {{\n",
                    self.indent(),
                    arch
                );
                self.indent_level += 1;
                for b_stmt in statements {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::ComptimeBlock { statements } => {
                let mut result = format!("{}comptime {{\n", self.indent());
                self.indent_level += 1;
                for b_stmt in statements {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
            Statement::ClusterBlock { statements } => {
                let mut result = format!("{}cluster {{\n", self.indent());
                self.indent_level += 1;
                for b_stmt in statements {
                    result.push_str(&self.format_statement(b_stmt));
                }
                self.indent_level -= 1;
                result.push_str(&format!("{}}}\n", self.indent()));
                result
            }
        }
    }

    fn format_expression(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::Number(n) => n.to_string(),
            Expression::StringLiteral(s) => format!("\"{}\"", s),
            Expression::Infix {
                left,
                operator,
                right,
            } => format!(
                "{} {} {}",
                self.format_expression(left),
                operator,
                self.format_expression(right)
            ),
            Expression::TensorDefinition { dimensions } => {
                let dims: Vec<String> = dimensions
                    .iter()
                    .map(|d| self.format_expression(d))
                    .collect();
                format!("tensor<{}>", dims.join(", "))
            }
            Expression::FunctionCall { name, arguments } => {
                let args: Vec<String> = arguments
                    .iter()
                    .map(|a| self.format_expression(a))
                    .collect();
                format!("{}({})", name, args.join(", "))
            }
            Expression::StructInit { name, fields } => {
                let f: Vec<String> = fields
                    .iter()
                    .map(|(n, e)| format!("{}: {}", n, self.format_expression(e)))
                    .collect();
                format!("{} {{ {} }}", name, f.join(", "))
            }
            Expression::FieldAccess { base, field } => {
                format!("{}.{}", self.format_expression(base), field)
            }
            Expression::IndexAccess { base, index } => {
                format!(
                    "{}[{}]",
                    self.format_expression(base),
                    self.format_expression(index)
                )
            }
            Expression::OramAccess { base, index } => {
                let base_str = self.format_expression(base);
                let index_str = self.format_expression(index);
                format!("{}[{}] /* ORAM */", base_str, index_str)
            }
            Expression::Try(expr) => {
                format!("{}?", self.format_expression(expr))
            }
            Expression::Comptime(inner) => {
                format!("comptime {}", self.format_expression(inner))
            }
            Expression::Prefix { operator, operand } => {
                let op = match operator.as_str() { "Minus" => "-", "Not" => "!", _ => operator.as_str() };
                format!("{}{}", op, self.format_expression(operand))
            }
            Expression::NvmeDmaMap { path, size } => {
                format!("@nvme_dma_map({}, {})", self.format_expression(path), self.format_expression(size))
            }
            Expression::ArrayLiteral(elements) => {
                let parts: Vec<String> = elements.iter().map(|e| self.format_expression(e)).collect();
                format!("[{}]", parts.join(", "))
            }
        }
    }

    fn format_type(&self, ty: &Type) -> String {
        match ty {
            Type::I8 => "i8".to_string(),
            Type::I32 => "i32".to_string(),
            Type::U64 => "u64".to_string(),
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Tensor { dimensions, is_sparse } => {
                let mut out = if *is_sparse { "sparse tensor".to_string() } else { "tensor".to_string() };
                if !dimensions.is_empty() {
                    out.push_str("<");
                    out.push_str(&dimensions.iter().map(|d| self.format_expression(d)).collect::<Vec<_>>().join(", "));
                    out.push_str(">");
                }
                out
            }
            Type::Array(base, size) => format!(
                "[{}; {}]",
                self.format_type(base),
                self.format_expression(size)
            ),
            Type::Struct(name) => name.clone(),
            Type::Unknown(name) => name.clone(),
            Type::Result(ok, err) => format!("Result<{}, {}>", self.format_type(ok), self.format_type(err)),
            Type::Pointer(base) => format!("*{}", self.format_type(base)),
        }
    }
}
