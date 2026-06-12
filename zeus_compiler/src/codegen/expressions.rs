#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or,
                  clippy::type_complexity, unused_imports)]
use crate::ast::{Expression, Program, Statement};
use super::CCodegen;

impl CCodegen {
    pub fn generate_expression(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::Number(val) => val.to_string(),
            Expression::StringLiteral(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Expression::Infix { left, operator, right } => {
                // Oblivious write: assignment into a secret SoA array element.
                if operator == "Assign" {
                    if let Some((arr, field, idx_expr)) = self.secret_soa_target(left) {
                        let r = self.generate_expression(right);
                        let idx_c = self.generate_expression(&idx_expr);
                        let n = self.soa_secret_lens.borrow().get(&arr).cloned().unwrap();
                        let t = self.soa_field_ctype(&arr, &field);
                        return format!("({{ {t} _zv = ({t})({r}); __zeus_owrite_bytes({a}_{f}, (size_t)({n}), sizeof({a}_{f}[0]), (size_t)({i}), &_zv); }})",
                            t=t, r=r, a=arr, f=field, n=n, i=idx_c);
                    }
                }
                let l = self.generate_expression(left);
                let r = self.generate_expression(right);
                let l_sec = self.is_secret_var(left);
                let r_sec = self.is_secret_var(right);
                
                if (l_sec || r_sec) && ["Plus", "Minus", "Star", "Slash"].contains(&operator.as_str()) {
                    return format!("__zeus_io_circuit_math((double)({}), (double)({}), {})", l, r, match operator.as_str() {
                        "Plus" => "0",
                        "Minus" => "1",
                        "Star" => "2",
                        "Slash" => "3",
                        _ => "0"
                    });
                }
                
                let op = match operator.as_str() {
                    "Plus" => "+",
                    "Minus" => "-",
                    "Star" => "*",
                    "Slash" => "/",
                    "Percent" => "%",
                    "Assign" => "=",
                    "PlusAssign" => "+=",
                    "MinusAssign" => "-=",
                    "StarAssign" => "*=",
                    "SlashAssign" => "/=",
                    "PercentAssign" => "%=",
                    "LessThan" => "<",
                    "GreaterThan" => ">",
                    "Equal" => "==",
                    "GreaterEqual" => ">=",
                    "LessEqual" => "<=",
                    "BitShiftLeft" => "<<",
                    "BitShiftRight" => ">>",
                    "BitwiseAnd" => "&",
                    "Pipe" => "|",
                    "And" => "&&",
                    "Or" => "||",
                    _ => operator.as_str(),
                };
                let is_bitwise = ["BitShiftLeft", "BitShiftRight", "BitwiseAnd", "Pipe"].contains(&operator.as_str());
                if operator.as_str() == "AtSign" {
                    format!("({{\n    zeus_tensor* _res = __zeus_arena_alloc(sizeof(zeus_tensor));\n    zeus_tensor* _l = {};\n    zeus_tensor* _r = {};\n    _res->dim1 = _l->dim1;\n    _res->dim2 = _r->dim2;\n    _res->data = __zeus_arena_alloc(_res->dim1 * _res->dim2 * sizeof(double));\n    for (size_t _i = 0; _i < _res->dim1; _i++) {{\n        for (size_t _j = 0; _j < _res->dim2; _j++) {{\n            double _sum = 0.0;\n            for (size_t _k = 0; _k < _l->dim2; _k++) {{\n                _sum += _l->data[_i * _l->dim2 + _k] * _r->data[_k * _r->dim2 + _j];\n            }}\n            _res->data[_i * _res->dim2 + _j] = _sum;\n        }}\n    }}\n    _res;\n}})", l, r)
                } else if is_bitwise {
                    format!("((int){} {} (int){})", l, op, r)
                } else if (operator.as_str() == "Slash" || operator.as_str() == "Percent")
                    && !matches!(right.as_ref(), Expression::Number(n) if *n != 0.0)
                    && { let t = self.infer_arg_c_type(right); t != "double" && t != "float" }
                {
                    // Guard integer divide/modulo: a clean deterministic trap instead of
                    // an undefined SIGFPE crash. Elided for constant-nonzero/float divisors.
                    format!("({{ __typeof__({r}) _zd = ({r}); if (_zd == 0) __zeus_div_zero_trap(); ({l}) {op} _zd; }})", r=r, l=l, op=op)
                } else {
                    format!("({} {} {})", l, op, r)
                }
            }
            Expression::TensorDefinition { dimensions } => {
                let dim1 = if dimensions.len() > 0 { self.generate_expression(&dimensions[0]) } else { "1".to_string() };
                let dim2 = if dimensions.len() > 1 { self.generate_expression(&dimensions[1]) } else { "1".to_string() };
                format!("({{\n    zeus_tensor* _t = __zeus_arena_alloc(sizeof(zeus_tensor));\n    _t->dim1 = {};\n    _t->dim2 = {};\n    _t->data = __zeus_arena_alloc(_t->dim1 * _t->dim2 * sizeof(double));\n    _t;\n}})", dim1, dim2)
            }
            Expression::FunctionCall { name, arguments } => {
                let args_c: Vec<String> = arguments.iter().map(|a| self.generate_expression(a)).collect();
                // Result<T,E> constructors
                if name == "Ok" && args_c.len() == 1 {
                    return format!("ZEUS_OK({})", args_c[0]);
                }
                if name == "Err" && args_c.len() == 1 {
                    // String literal errors use ZEUS_ERR_STR; numeric/other use ZEUS_ERR.
                    let is_str = matches!(arguments.first(), Some(crate::ast::Expression::StringLiteral(_)));
                    return if is_str {
                        format!("ZEUS_ERR_STR({})", args_c[0])
                    } else {
                        format!("ZEUS_ERR({})", args_c[0])
                    };
                }
                // unwrap() — extract ok_val, panic on error (uses ZEUS_UNWRAP macro)
                if name == "unwrap" && args_c.len() == 1 {
                    return format!("ZEUS_UNWRAP({})", args_c[0]);
                }
                // unwrap_or(default) — extract ok_val or use default
                if name == "unwrap_or" && args_c.len() == 2 {
                    return format!("({{ zeus_result_t _u = (zeus_result_t)({}); _u.is_error ? (double)({}) : _u.ok_val; }})", args_c[0], args_c[1]);
                }
                // is_ok() / is_err() predicates
                if name == "is_ok" && args_c.len() == 1 {
                    return format!("(!(zeus_result_t)({})).is_error", args_c[0]);
                }
                if name == "is_err" && args_c.len() == 1 {
                    return format!("((zeus_result_t)({})).is_error", args_c[0]);
                }
                if name == "print" || name == "println" {
                    let _ = &args_c;
                    let nl = name == "println";
                    let body = self.generate_print_builtin(arguments, nl, "", None);
                    let trimmed = body.trim_end().trim_end_matches(';');
                    format!("({})", trimmed)
                } else if let Some(builtin) = self.lower_math_builtin(name, &args_c) {
                    builtin
                } else if let Some((params, ret_type)) = self.extern_functions.borrow().get(name) {
                    let mut is_soa_translation_needed = false;
                    for (_, p_type) in params.iter() {
                        if let crate::ast::Type::Struct(_) = p_type {
                            is_soa_translation_needed = true;
                        }
                    }

                    if is_soa_translation_needed {
                        let mut block = String::from("({ ");
                        let mut call_args = Vec::new();
                        let mut writes_back = Vec::new();

                        for (i, (_, p_type)) in params.iter().enumerate() {
                            if let crate::ast::Type::Struct(struct_name) = p_type {
                                // [ZEUS FAT PTR FFI BRIDGE] Whole-array SoA: pass FatPtr*, zero copy.
                                if let Expression::Identifier(arr_name) = &arguments[i] {
                                    if self.soa_arrays.borrow().contains(arr_name.as_str()) {
                                        let fp_name = format!("_zeus_fp_{}", i);
                                        block.push_str(&format!("{}_FatPtr {}; ", struct_name, fp_name));
                                        if let Some(fields) = self.struct_schemas.borrow().get(struct_name) {
                                            for (f_name, _) in fields {
                                                block.push_str(&format!("{}.{} = {}_{};  ", fp_name, f_name, arr_name, f_name));
                                            }
                                            if let Some((first_f, _)) = fields.first() {
                                                block.push_str(&format!("{}.len = sizeof({}_{})/sizeof(*{}_{});  ",
                                                    fp_name, arr_name, first_f, arr_name, first_f));
                                            }
                                        }
                                        call_args.push(format!("&{}", fp_name));
                                        continue;
                                    }
                                }
                                // Element-level: index into SoA → temp struct copy-in/copy-out
                                if let Expression::IndexAccess { base, index } = &arguments[i] {
                                    if let Expression::Identifier(arr_name) = &**base {
                                        let idx_c = self.generate_expression(index);
                                        let temp_name = format!("_zeus_tmp_{}", i);
                                        block.push_str(&format!("{} {}; ", struct_name, temp_name));
                                        if let Some(fields) = self.struct_schemas.borrow().get(struct_name) {
                                            for (f_name, _) in fields {
                                                block.push_str(&format!("{}.{} = {}_{}[(size_t)({})]; ", temp_name, f_name, arr_name, f_name, idx_c));
                                                writes_back.push(format!("{}_{}[(size_t)({})] = {}.{}; ", arr_name, f_name, idx_c, temp_name, f_name));
                                            }
                                        }
                                        call_args.push(format!("&{}", temp_name));
                                        continue;
                                    }
                                }
                            }
                            call_args.push(args_c[i].clone());
                        }

                        let c_ret = self.type_to_c(ret_type);
                        if c_ret == "void" {
                            block.push_str(&format!("size_t __phoenix_mark = zeus_arena_offset; {}({}); ", name, call_args.join(", ")));
                            for wb in writes_back { block.push_str(&wb); }
                            block.push_str("zeus_arena_offset = __phoenix_mark; })");
                        } else {
                            block.push_str(&format!("size_t __phoenix_mark = zeus_arena_offset; {} _res = {}({}); ", c_ret, name, call_args.join(", ")));
                            for wb in writes_back { block.push_str(&wb); }
                            block.push_str("zeus_arena_offset = __phoenix_mark; _res; })");
                        }
                        block
                    } else {
                        let c_ret = self.type_to_c(ret_type);
                        if c_ret == "void" {
                            format!("({{ size_t __phoenix_mark = *zeus_arena_offset; {}({}); *zeus_arena_offset = __phoenix_mark; }})", name, args_c.join(", "))
                        } else {
                            format!("({{ size_t __phoenix_mark = *zeus_arena_offset; {} _res = {}({}); *zeus_arena_offset = __phoenix_mark; _res; }})", c_ret, name, args_c.join(", "))
                        }
                    }
                } else {
                    format!("{}({})", name, args_c.join(", "))
                }
            }
            Expression::StructInit { name, fields } => {
                let parts: Vec<String> = fields.iter()
                    .map(|(f, v)| format!(".{} = {}", f, self.generate_expression(v)))
                    .collect();
                format!("(({}){{ {} }})", name, parts.join(", "))
            }
            Expression::FieldAccess { base, field } => {
                // SoA field access: particles[i].x → particles_x[i]
                // After ORAM pass, particles[i] is OramAccess; before it's IndexAccess.
                let soa_field: Option<String> = match &**base {
                    Expression::IndexAccess { base: arr_base, index }
                    | Expression::OramAccess { base: arr_base, index, .. } => {
                        if let Expression::Identifier(arr_name) = arr_base.as_ref() {
                            let is_soa = self.soa_arrays.borrow().contains(arr_name.as_str());
                            let secret_len = self.soa_secret_lens.borrow().get(arr_name).cloned();
                            if let Some(n) = secret_len {
                                // Oblivious read: full-scan constant-time select.
                                let idx_c = self.generate_expression(index);
                                let t = self.soa_field_ctype(arr_name, field);
                                Some(format!("({{ {t} _zo; __zeus_oread_bytes(&_zo, {a}_{f}, (size_t)({n}), sizeof({a}_{f}[0]), (size_t)({i})); _zo; }})",
                                    t=t, a=arr_name, f=field, n=n, i=idx_c))
                            } else if is_soa {
                                Some(format!("{}_{}[(size_t)({})]", arr_name, field, self.generate_expression(index)))
                            } else { None }
                        } else { None }
                    }
                    _ => None,
                };
                if let Some(s) = soa_field { return s; }
                if field == "data" {
                    format!("{}->{}", self.generate_expression(base), field)
                } else {
                    format!("{}.{}", self.generate_expression(base), field)
                }
            }
            Expression::IndexAccess { base, index } => {
                format!("{}[(size_t)({})]", self.generate_expression(base), self.generate_expression(index))
            }
            Expression::OramAccess { base, index, bound: _ } => {
                let b = self.generate_expression(base);
                let i = self.generate_expression(index);
                // ORAM Dummy Sequence: Flattening memory access to disguise hardware bus activity
                format!("{}[(size_t)({})] /* note: oblivious protection applies to `secret` fixed-size struct arrays; this is a direct access */", b, i)
            }
            Expression::Try(inner) => {
                let inner_c = self.generate_expression(inner);
                format!("ZEUS_TRY({})", inner_c)
            }
            Expression::Comptime(inner) => {
                self.generate_expression(inner)
            }
            Expression::Prefix { operator, operand } => {
                let o = self.generate_expression(operand);
                let op = match operator.as_str() { "Minus" => "-", "Not" => "!", _ => operator.as_str() };
                format!("({}{})", op, o)
            }
            Expression::NvmeDmaMap { path, size } => {
                let p = self.generate_expression(path);
                let s = self.generate_expression(size);
                if self.is_target_nvme {
                    format!("({{\n    #ifndef O_DIRECT\n    #define O_DIRECT 0\n    #endif\n    int _fd = open({}, O_RDWR | O_DIRECT | O_SYNC);\n    if (_fd < 0) {{ perror(\"open NVMe\"); exit(1); }}\n    void* _map = mmap(NULL, {}, PROT_READ | PROT_WRITE, MAP_SHARED, _fd, 0);\n    if (_map == MAP_FAILED) {{ perror(\"mmap NVMe\"); exit(1); }}\n    _map;\n}})", p, s)
                } else {
                    format!("({{\n    // Fallback standard POSIX I/O since --target=nvme was not provided\n    FILE* _f = fopen({}, \"r+\");\n    if (!_f) {{ perror(\"fopen fallback\"); exit(1); }}\n    void* _map = malloc({});\n    fread(_map, 1, {}, _f);\n    fclose(_f);\n    _map;\n}})", p, s, s)
                }
            }
            Expression::ArrayLiteral(elems) => {
                let parts: Vec<String> = elems.iter().map(|e| self.generate_expression(e)).collect();
                format!("{{{}}}", parts.join(", "))
            }
            Expression::EnumVariant { enum_name, variant, payload } => {
                if payload.is_empty() {
                    // Compound literal for a unit variant: ((Dir){ .tag = Dir__North })
                    format!("(({}){{ .tag = {}__{}  }})", enum_name, enum_name, variant)
                } else {
                    let args: Vec<String> = payload.iter().map(|e| self.generate_expression(e)).collect();
                    // Compound literal with data: ((Dir){ .tag = Dir__V, .data.V = { arg0, arg1 } })
                    let data_inits: Vec<String> = args.iter().enumerate()
                        .map(|(i, a)| format!("._{}={}", i, a)).collect();
                    format!("(({}){{ .tag = {}__{}  , .data.{} = {{ {} }} }})",
                        enum_name, enum_name, variant, variant, data_inits.join(", "))
                }
            }
            Expression::MatchExpr { scrutinee, arms } => {
                // Emit as a C compound statement expression (gcc extension)
                let mut s = format!("({{ __auto_type __scrut = {}; ", self.generate_expression(scrutinee));
                for (i, arm) in arms.iter().enumerate() {
                    let cond = match &arm.pattern {
                        crate::ast::MatchPattern::Variant { enum_name, variant } => {
                            format!("__scrut.tag == {}__{}", enum_name, variant)
                        }
                        crate::ast::MatchPattern::VariantTuple { enum_name, variant, bindings: _ } => {
                            format!("__scrut.tag == {}__{}", enum_name, variant)
                        }
                        crate::ast::MatchPattern::Wildcard => "1".to_string(),
                        crate::ast::MatchPattern::Literal(n) => format!("__scrut == {}", n),
                    };
                    let mut body_stmts: Vec<String> = arm.body.iter().map(|s| self.generate_statement(s, 0)).collect();
                    if let crate::ast::MatchPattern::VariantTuple { variant, bindings, .. } = &arm.pattern {
                        for (j, b) in bindings.iter().enumerate().rev() {
                            if b != "_" {
                                body_stmts.insert(0, format!("__auto_type {} = __scrut.data.{}._{}; ", b, variant, j));
                            }
                        }
                    }
                    let body = body_stmts.join(" ");
                    if i == 0 { s.push_str(&format!("if ({}) {{ {} }}", cond, body)); }
                    else if matches!(arm.pattern, crate::ast::MatchPattern::Wildcard) {
                        s.push_str(&format!(" else {{ {} }}", body));
                    } else {
                        s.push_str(&format!(" else if ({}) {{ {} }}", cond, body));
                    }
                }
                s.push_str(" 0; })");
                s
            }
        }
    }

}
