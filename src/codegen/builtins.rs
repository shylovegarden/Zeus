#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or,
                  clippy::type_complexity, unused_imports)]
use crate::ast::{Expression, Program, Statement};
use super::CCodegen;

impl CCodegen {
    pub(crate) fn c_type_to_printf(&self, c_type: &str) -> (&'static str, &'static str) {
        match c_type {
            "bool" => ("%d", "(int)"),
            "float" | "double" => ("%f", "(double)"),
            "const char*" | "char*" => ("%s", ""),
            "int8_t" | "uint8_t" | "int32_t" | "uint32_t"
            | "uint64_t" | "int64_t" | "size_t" | "int" => ("%lld", "(long long)"),
            _ => ("%lld", "(long long)"),
        }
    }

    pub(crate) fn infer_arg_c_type(&self, expr: &Expression) -> String {
        match expr {
            Expression::StringLiteral(_) => "const char*".to_string(),
            Expression::Number(n) => {
                if n.fract() == 0.0 { "int64_t".to_string() } else { "double".to_string() }
            }
            Expression::Identifier(name) => {
                self.current_var_types.borrow().get(name).cloned()
                    .unwrap_or_else(|| "int64_t".to_string())
            }
            Expression::Infix { left, operator, right } => {
                match operator.as_str() {
                    "LessThan" | "GreaterThan" | "Equal" | "NotEqual"
                    | "GreaterEqual" | "LessEqual" => "bool".to_string(),
                    "Plus" | "Minus" | "Star" | "Slash" | "Percent" => {
                        let lt = self.infer_arg_c_type(left);
                        let rt = self.infer_arg_c_type(right);
                        if lt == "double" || lt == "float" || rt == "double" || rt == "float" {
                            "double".to_string()
                        } else if lt == "const char*" || rt == "const char*" {
                            "const char*".to_string()
                        } else {
                            "int64_t".to_string()
                        }
                    }
                    _ => self.infer_arg_c_type(left),
                }
            }
            Expression::FieldAccess { base, field } => {
                if let Expression::Identifier(var) = base.as_ref() {
                    if let Some(struct_name) = self.current_var_types.borrow().get(var).cloned() {
                        if let Some(fields) = self.struct_schemas.borrow().get(&struct_name) {
                            if let Some((_, ft)) = fields.iter().find(|(f, _)| f == field) {
                                return self.type_to_c(&Some(ft.clone()));
                            }
                        }
                    }
                }
                "int64_t".to_string()
            }
            Expression::FunctionCall { name, arguments } => {
                match name.as_str() {
                    "sqrt" | "pow" | "floor" | "ceil" => "double".to_string(),
                    "abs" => arguments.first().map(|a| self.infer_arg_c_type(a)).unwrap_or_else(|| "int64_t".to_string()),
                    "min" | "max" | "clamp" => {
                        if arguments.iter().any(|a| { let t=self.infer_arg_c_type(a); t=="double"||t=="float" }) { "double".to_string() } else { "int64_t".to_string() }
                    }
                    _ => "int64_t".to_string(),
                }
            }
            Expression::Comptime(inner) | Expression::Try(inner) => self.infer_arg_c_type(inner),
            _ => "int64_t".to_string(),
        }
    }

    pub(crate) fn lower_math_builtin(&self, name: &str, a: &[String]) -> Option<String> {
        match (name, a.len()) {
            ("abs", 1) => Some(format!("(_Generic(({x}), float: fabsf, double: fabs, default: llabs)({x}))", x = a[0])),
            ("min", 2) => Some(format!("(({a})<({b})?({a}):({b}))", a = a[0], b = a[1])),
            ("max", 2) => Some(format!("(({a})>({b})?({a}):({b}))", a = a[0], b = a[1])),
            ("clamp", 3) => Some(format!("(({x})<({lo})?({lo}):(({x})>({hi})?({hi}):({x})))", x = a[0], lo = a[1], hi = a[2])),
            ("sqrt", 1) => Some(format!("(sqrt({}))", a[0])),
            ("pow", 2) => Some(format!("(pow({}, {}))", a[0], a[1])),
            ("floor", 1) => Some(format!("(floor({}))", a[0])),
            ("ceil", 1) => Some(format!("(ceil({}))", a[0])),
            _ => None,
        }
    }

    pub(crate) fn generate_print_builtin(
        &self,
        arguments: &[Expression],
        newline: bool,
        pad: &str,
        parallel_ctx: Option<(&[(String, String)], &str)>,
    ) -> String {
        let mut fmt = String::new();
        let mut call_args: Vec<String> = Vec::new();
        for arg in arguments {
            let c_type = self.infer_arg_c_type(arg);
            let (spec, cast) = self.c_type_to_printf(&c_type);
            fmt.push_str(spec);
            let arg_c = match parallel_ctx {
                Some((shared, iter)) => self.generate_parallel_expression(arg, shared, iter),
                None => self.generate_expression(arg),
            };
            if cast.is_empty() {
                call_args.push(arg_c);
            } else {
                call_args.push(format!("{}({})", cast, arg_c));
            }
        }
        if newline {
            fmt.push_str("\\n");
        }
        if call_args.is_empty() {
            format!("{}printf(\"{}\");\n", pad, fmt)
        } else {
            format!("{}printf(\"{}\", {});\n", pad, fmt, call_args.join(", "))
        }
    }
}
