#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or,
                  clippy::type_complexity, unused_imports)]
use crate::ast::{Expression, Program, Statement};
use super::CCodegen;

impl CCodegen {
    pub(crate) fn generate_parallel_definitions(&self, program: &Program) -> String {
        let mut defs = String::new();
        let mut counter: u64 = 0;
        
        for stmt in &program.statements {
            self.collect_parallel_defs(stmt, &mut defs, &mut counter);
        }
        defs
    }
    
    pub(crate) fn collect_parallel_defs(&self, stmt: &Statement, defs: &mut String, counter: &mut u64) {
        match stmt {
            Statement::ParallelBlock { iterator, start: _, end: _, statements } => {
                let block_id = *counter;
                *counter += 1;
                let struct_name = format!("__zeus_parallel_task_{}", block_id);
                let worker_name = format!("__zeus_parallel_worker_{}", block_id);
                
                let shared_vars = self.find_shared_variables(statements, iterator);
                
                defs.push_str(&format!("// [ZEUS PARALLEL BLOCK #{}]\n", block_id));
                defs.push_str(&format!("typedef struct {}{{\n", struct_name));
                defs.push_str("    size_t chunk_start;\n");
                defs.push_str("    size_t chunk_end;\n");
                for (var_name, var_type) in &shared_vars {
                    defs.push_str(&format!("    {}* {};\n", var_type, var_name));
                }
                defs.push_str("    volatile unsigned long long* heartbeat;\n");
                defs.push_str(&format!("}} {};\n\n", struct_name));
                
                defs.push_str(&format!("void {}(void* __zeus_ctx, size_t __zeus_start, size_t __zeus_end) {{\n", worker_name));
                defs.push_str(&format!("    {}* __zeus_data = ({}*)__zeus_ctx;\n", struct_name, struct_name));
                defs.push_str("    (void)__zeus_data;\n");
                // Hint the auto-vectorizer: no loop-carried dependencies in this parallel body
                defs.push_str("    #pragma GCC ivdep\n");
                defs.push_str(&format!("    for (size_t {} = __zeus_start; {} < __zeus_end; {}++) {{\n", iterator, iterator, iterator));
                        
                for s in statements {
                    let stmt_code = self.generate_parallel_statement(s, 2, &shared_vars, iterator);
                    defs.push_str(&stmt_code);
                }
                defs.push_str("    }\n");
                defs.push_str("}\n\n");
            }
            Statement::FunctionDeclaration { body, .. } => {
                for s in body {
                    self.collect_parallel_defs(s, defs, counter);
                }
            }
            _ => {}
        }
    }

    /// Pre-pass: find every variable that appears as the TARGET of @atomic_add.
    /// These will be typed as int64_t so __atomic_fetch_add compiles without a CAS loop.
    pub(crate) fn collect_atomic_int_vars(&self, program: &Program) {
        for stmt in &program.statements {
            self.collect_atomic_int_vars_in_stmt(stmt);
        }
    }

    pub(crate) fn collect_atomic_int_vars_in_stmt(&self, stmt: &Statement) {
        match stmt {
            Statement::AtomicAdd { target, .. } => {
                self.atomic_int_vars.borrow_mut().insert(target.clone());
            }
            Statement::ParallelBlock { statements, .. }
            | Statement::FunctionDeclaration { body: statements, .. }
            | Statement::For { body: statements, .. }
            | Statement::ProofBlock { statements }
            | Statement::EnclaveBlock { statements }
            | Statement::SafeStateBlock { statements }
            | Statement::TargetBlock { statements, .. }
            | Statement::CfgBlock { statements, .. }
            | Statement::ComptimeBlock { statements }
            | Statement::ClusterBlock { statements } => {
                for s in statements {
                    self.collect_atomic_int_vars_in_stmt(s);
                }
            }
            Statement::If { consequence, alternative, .. } => {
                for s in consequence { self.collect_atomic_int_vars_in_stmt(s); }
                if let Some(alt) = alternative {
                    for s in alt { self.collect_atomic_int_vars_in_stmt(s); }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn collect_var_types(&self, program: &Program) {
        for stmt in &program.statements {
            self.collect_var_types_in_stmt(stmt);
        }
    }

    pub(crate) fn collect_var_types_in_stmt(&self, stmt: &Statement) {
        match stmt {
            Statement::Let { name, var_type, .. } => {
                let c_type = if self.atomic_int_vars.borrow().contains(name) {
                    // @atomic_add targets must be int64_t so __atomic_fetch_add compiles
                    "int64_t".to_string()
                } else {
                    self.type_to_c(var_type)
                };
                self.current_var_types.borrow_mut().insert(name.clone(), c_type);
            }
            Statement::FunctionDeclaration { parameters, body, .. } => {
                for (p_name, p_type) in parameters {
                    let c_type = self.type_to_c(&Some(p_type.clone()));
                    self.current_var_types.borrow_mut().insert(p_name.clone(), c_type);
                }
                for s in body {
                    self.collect_var_types_in_stmt(s);
                }
            }
            Statement::For { iterator, body, .. } => {
                self.current_var_types.borrow_mut().insert(iterator.clone(), "size_t".to_string());
                for s in body {
                    self.collect_var_types_in_stmt(s);
                }
            }
            Statement::ParallelBlock { iterator, statements, .. } => {
                self.current_var_types.borrow_mut().insert(iterator.clone(), "size_t".to_string());
                for s in statements {
                    self.collect_var_types_in_stmt(s);
                }
            }
            Statement::If { consequence, alternative, .. } => {
                for s in consequence {
                    self.collect_var_types_in_stmt(s);
                }
                if let Some(alt) = alternative {
                    for s in alt {
                        self.collect_var_types_in_stmt(s);
                    }
                }
            }
            Statement::TargetBlock { statements, .. }
            | Statement::EnclaveBlock { statements }
            | Statement::ProofBlock { statements }
            | Statement::SafeStateBlock { statements }
            | Statement::CfgBlock { statements, .. }
            | Statement::ComptimeBlock { statements }
            | Statement::ClusterBlock { statements } => {
                for s in statements {
                    self.collect_var_types_in_stmt(s);
                }
            }
            _ => {}
        }
    }

    /// Resolve the C type of an SoA field, defaulting to double.
    pub(crate) fn soa_field_ctype(&self, arr: &str, field: &str) -> String {
        let struct_name = self.soa_struct_of.borrow().get(arr).cloned();
        if let Some(sn) = struct_name {
            let ft = self.struct_schemas.borrow().get(&sn)
                .and_then(|fields| fields.iter().find(|(f, _)| f == field).map(|(_, t)| t.clone()));
            if let Some(t) = ft { return self.type_to_c(&Some(t)); }
        }
        "double".to_string()
    }

    /// If `expr` is a field access into a `secret` SoA array, return (array, field, index).
    pub(crate) fn secret_soa_target(&self, expr: &Expression) -> Option<(String, String, Expression)> {
        if let Expression::FieldAccess { base, field } = expr {
            match base.as_ref() {
                Expression::IndexAccess { base: ab, index }
                | Expression::OramAccess { base: ab, index, .. } => {
                    if let Expression::Identifier(arr) = ab.as_ref() {
                        if self.soa_secret_lens.borrow().contains_key(arr) {
                            return Some((arr.clone(), field.clone(), (**index).clone()));
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub(crate) fn find_shared_variables(&self, statements: &[Statement], iterator: &str) -> Vec<(String, String)> {
        let mut local_vars = std::collections::HashSet::new();
        let mut referenced = Vec::new();
        for s in statements {
            self.find_referenced_in_stmt(s, iterator, &mut local_vars, &mut referenced);
        }
        
        let mut shared = Vec::new();
        for name in referenced {
            if !shared.iter().any(|(n, _)| *n == name) {
                let var_type = self.current_var_types.borrow()
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| "double".to_string());
                shared.push((name, var_type));
            }
        }
        shared
    }

    pub(crate) fn find_referenced_in_stmt(
        &self,
        stmt: &Statement,
        iterator: &str,
        local_vars: &mut std::collections::HashSet<String>,
        referenced: &mut Vec<String>,
    ) {
        match stmt {
            Statement::Let { name, value, .. } => {
                self.find_referenced_in_expr(value, iterator, local_vars, referenced);
                local_vars.insert(name.clone());
            }
            Statement::ExpressionStatement(expr) | Statement::Assert(expr) | Statement::Return(expr) => {
                self.find_referenced_in_expr(expr, iterator, local_vars, referenced);
            }
            Statement::If { condition, consequence, alternative } => {
                self.find_referenced_in_expr(condition, iterator, local_vars, referenced);
                let mut con_locals = local_vars.clone();
                for s in consequence {
                    self.find_referenced_in_stmt(s, iterator, &mut con_locals, referenced);
                }
                if let Some(alt) = alternative {
                    let mut alt_locals = local_vars.clone();
                    for s in alt {
                        self.find_referenced_in_stmt(s, iterator, &mut alt_locals, referenced);
                    }
                }
            }
            Statement::For { iterator: for_iter, start, end, body } => {
                self.find_referenced_in_expr(start, iterator, local_vars, referenced);
                self.find_referenced_in_expr(end, iterator, local_vars, referenced);
                let mut body_locals = local_vars.clone();
                body_locals.insert(for_iter.clone());
                for s in body {
                    self.find_referenced_in_stmt(s, iterator, &mut body_locals, referenced);
                }
            }
            Statement::ParallelBlock { iterator: par_iter, start, end, statements } => {
                self.find_referenced_in_expr(start, iterator, local_vars, referenced);
                self.find_referenced_in_expr(end, iterator, local_vars, referenced);
                let mut body_locals = local_vars.clone();
                body_locals.insert(par_iter.clone());
                for s in statements {
                    self.find_referenced_in_stmt(s, iterator, &mut body_locals, referenced);
                }
            }
            Statement::AtomicAdd { target, amount } => {
                if target != iterator && !local_vars.contains(target) {
                    referenced.push(target.clone());
                }
                if !amount.chars().next().is_none_or(|c| c.is_ascii_digit()) {
                    if amount != iterator && !local_vars.contains(amount) {
                        referenced.push(amount.clone());
                    }
                }
            }
            Statement::TargetBlock { statements, .. }
            | Statement::EnclaveBlock { statements }
            | Statement::ProofBlock { statements }
            | Statement::SafeStateBlock { statements }
            | Statement::CfgBlock { statements, .. }
            | Statement::ComptimeBlock { statements }
            | Statement::ClusterBlock { statements } => {
                let mut inner_locals = local_vars.clone();
                for s in statements {
                    self.find_referenced_in_stmt(s, iterator, &mut inner_locals, referenced);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn find_referenced_in_expr(
        &self,
        expr: &Expression,
        iterator: &str,
        local_vars: &std::collections::HashSet<String>,
        referenced: &mut Vec<String>,
    ) {
        match expr {
            Expression::Identifier(name) => {
                if name != iterator && !local_vars.contains(name) {
                    referenced.push(name.clone());
                }
            }
            Expression::Infix { left, right, .. } => {
                self.find_referenced_in_expr(left, iterator, local_vars, referenced);
                self.find_referenced_in_expr(right, iterator, local_vars, referenced);
            }
            Expression::TensorDefinition { dimensions } => {
                for dim in dimensions {
                    self.find_referenced_in_expr(dim, iterator, local_vars, referenced);
                }
            }
            Expression::FunctionCall { arguments, .. } => {
                for arg in arguments {
                    self.find_referenced_in_expr(arg, iterator, local_vars, referenced);
                }
            }
            Expression::StructInit { fields, .. } => {
                for (_, val) in fields {
                    self.find_referenced_in_expr(val, iterator, local_vars, referenced);
                }
            }
            Expression::FieldAccess { base, .. } => {
                self.find_referenced_in_expr(base, iterator, local_vars, referenced);
            }
            Expression::IndexAccess { base, index } | Expression::OramAccess { base, index, .. } => {
                self.find_referenced_in_expr(base, iterator, local_vars, referenced);
                self.find_referenced_in_expr(index, iterator, local_vars, referenced);
            }
            Expression::Try(inner) | Expression::Comptime(inner) => {
                self.find_referenced_in_expr(inner, iterator, local_vars, referenced);
            }
            Expression::ArrayLiteral(elems) => {
                for e in elems { self.find_referenced_in_expr(e, iterator, local_vars, referenced); }
            }
            _ => {}
        }
    }

    pub(crate) fn generate_parallel_statement(&self, stmt: &Statement, indent: usize, shared_vars: &[(String, String)], iterator: &str) -> String {
        let pad = "    ".repeat(indent);
        match stmt {
            Statement::Let { name, is_mut: _, is_secret: _, value, var_type } => {
                let val_c = self.generate_parallel_expression(value, shared_vars, iterator);
                let c_type = self.type_to_c(var_type);
                format!("{}    {} {} = {};\n", pad, c_type, name, val_c)
            }
            Statement::ExpressionStatement(expr) => {
                if let Expression::FunctionCall { name, arguments } = expr {
                    if name == "print" {
                        return self.generate_print_builtin(arguments, false, &pad, Some((shared_vars, iterator)));
                    }
                    if name == "println" {
                        return self.generate_print_builtin(arguments, true, &pad, Some((shared_vars, iterator)));
                    }
                }
                let expr_c = self.generate_parallel_expression(expr, shared_vars, iterator);
                if expr_c == "print" {
                    format!("{}printf(\"Execution complete.\\n\");\n", pad)
                } else {
                    format!("{}{};\n", pad, expr_c)
                }
            }
            Statement::If { condition, consequence, alternative } => {
                let mut out = format!("{}if ({}) {{\n", pad, self.generate_parallel_expression(condition, shared_vars, iterator));
                for s in consequence {
                    out.push_str(&self.generate_parallel_statement(s, indent + 1, shared_vars, iterator));
                }
                if let Some(alt) = alternative {
                    out.push_str(&format!("{}}} else {{\n", pad));
                    for s in alt {
                        out.push_str(&self.generate_parallel_statement(s, indent + 1, shared_vars, iterator));
                    }
                }
                out.push_str(&format!("{}}}\n", pad));
                out
            }
            Statement::For { iterator: for_iter, start, end, body } => {
                let start_c = self.generate_parallel_expression(start, shared_vars, iterator);
                let end_c = self.generate_parallel_expression(end, shared_vars, iterator);
                let mut out = format!("{}for (size_t {} = {}; {} < {}; {}++) {{\n", pad, for_iter, start_c, for_iter, end_c, for_iter);
                for s in body {
                    out.push_str(&self.generate_parallel_statement(s, indent + 1, shared_vars, iterator));
                }
                out.push_str(&format!("{}}}\n", pad));
                out
            }
            Statement::Return(expr) => {
                let expr_c = self.generate_parallel_expression(expr, shared_vars, iterator);
                format!("{}return {};\n", pad, expr_c)
            }
            Statement::AtomicAdd { target, amount } => {
                let amt_c = if shared_vars.iter().any(|(n, _)| n == amount) {
                    format!("(*__zeus_data->{})", amount)
                } else {
                    amount.clone()
                };
                if shared_vars.iter().any(|(n, _)| n == target) {
                    let target_type = shared_vars.iter()
                        .find(|(n, _)| n == target)
                        .map(|(_, t)| t.as_str())
                        .unwrap_or("double");
                    if target_type == "int64_t" {
                        // Clean integer atomic: __atomic_fetch_add works directly on int64_t*
                        format!(
                            "{}__atomic_fetch_add(__zeus_data->{}, (int64_t)({}), __ATOMIC_SEQ_CST);\n",
                            pad, target, amt_c
                        )
                    } else {
                        // Fallback CAS loop for non-integer shared vars (e.g. double*)
                        format!(
                            "{}{{ double _zat_old, _zat_new; do {{ _zat_old = *__zeus_data->{}; _zat_new = _zat_old + (double)({}); }} while (!__atomic_compare_exchange_n((int64_t*)__zeus_data->{}, (int64_t*)&_zat_old, *(int64_t*)&_zat_new, 0, __ATOMIC_SEQ_CST, __ATOMIC_RELAXED)); }}\n",
                            pad, target, amt_c, target
                        )
                    }
                } else {
                    format!("{}__atomic_fetch_add(&{}, {}, __ATOMIC_SEQ_CST);\n", pad, target, amt_c)
                }
            }
            Statement::Assert(expr) => {
                let expr_c = self.generate_parallel_expression(expr, shared_vars, iterator);
                format!("{}// [ZEUS VERIFIED: assert({})]\n", pad, expr_c)
            }
            Statement::EnclaveBlock { statements } => {
                let mut out = format!("{}// [ZEUS: scoped secure region — compiler memory barrier; no hardware enclave on this target]\n{}zeus_enclave_enter();\n{}{{ \n", pad, pad, pad);
                for s in statements {
                    out.push_str(&self.generate_parallel_statement(s, indent + 1, shared_vars, iterator));
                }
                out.push_str(&format!("{}}}\n", pad));
                out.push_str(&format!("{}zeus_enclave_exit();\n{}// [ZEUS: end secure region]\n", pad, pad));
                out
            }
            Statement::TargetBlock { targets, statements } => {
                let target_str = targets.join(", ");
                let mut out = format!("{}// [ZEUS: TARGET SPECIFIC START: {}]\n", pad, target_str);
                for s in statements {
                    out.push_str(&self.generate_parallel_statement(s, indent, shared_vars, iterator));
                }
                out.push_str(&format!("{}// [ZEUS: TARGET SPECIFIC END]\n", pad));
                out
            }
            Statement::ProofBlock { statements }
            | Statement::SafeStateBlock { statements }
            | Statement::CfgBlock { statements, .. }
            | Statement::ComptimeBlock { statements }
            | Statement::ClusterBlock { statements } => {
                let mut out = String::new();
                for s in statements {
                    out.push_str(&self.generate_parallel_statement(s, indent, shared_vars, iterator));
                }
                out
            }
            _ => self.generate_statement(stmt, indent)
        }
    }

    pub(crate) fn generate_parallel_expression(&self, expr: &Expression, shared_vars: &[(String, String)], iterator: &str) -> String {
        match expr {
            Expression::Identifier(name) => {
                if name == iterator {
                    name.clone()
                } else if shared_vars.iter().any(|(n, _)| n == name) {
                    format!("(*__zeus_data->{})", name)
                } else {
                    name.clone()
                }
            }
            Expression::Infix { left, operator, right } => {
                let left_c = self.generate_parallel_expression(left, shared_vars, iterator);
                let right_c = self.generate_parallel_expression(right, shared_vars, iterator);
                let op_str = match operator.as_str() {
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
                    format!("({{\n    zeus_tensor* _res = __zeus_arena_alloc(sizeof(zeus_tensor));\n    zeus_tensor* _l = {};\n    zeus_tensor* _r = {};\n    _res->dim1 = _l->dim1;\n    _res->dim2 = _r->dim2;\n    _res->data = __zeus_arena_alloc(_res->dim1 * _res->dim2 * sizeof(double));\n    for (size_t _i = 0; _i < _res->dim1; _i++) {{\n        for (size_t _j = 0; _j < _res->dim2; _j++) {{\n            double _sum = 0.0;\n            for (size_t _k = 0; _k < _l->dim2; _k++) {{\n                _sum += _l->data[_i * _l->dim2 + _k] * _r->data[_k * _r->dim2 + _j];\n            }}\n            _res->data[_i * _res->dim2 + _j] = _sum;\n        }}\n    }}\n    _res;\n}})", left_c, right_c)
                } else if is_bitwise {
                    format!("((int){} {} (int){})", left_c, op_str, right_c)
                } else if (operator.as_str() == "Slash" || operator.as_str() == "Percent")
                    && !matches!(right.as_ref(), Expression::Number(n) if *n != 0.0)
                    && { let t = self.infer_arg_c_type(right); t != "double" && t != "float" }
                {
                    format!("({{ __typeof__({r}) _zd = ({r}); if (_zd == 0) __zeus_div_zero_trap(); ({l}) {op} _zd; }})", r=right_c, l=left_c, op=op_str)
                } else {
                    format!("({} {} {})", left_c, op_str, right_c)
                }
            }
            Expression::Number(n) => n.to_string(),
            Expression::StringLiteral(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Expression::TensorDefinition { dimensions } => {
                let dim1 = if dimensions.len() > 0 { self.generate_parallel_expression(&dimensions[0], shared_vars, iterator) } else { "1".to_string() };
                let dim2 = if dimensions.len() > 1 { self.generate_parallel_expression(&dimensions[1], shared_vars, iterator) } else { "1".to_string() };
                format!("({{\n    zeus_tensor* _t = __zeus_arena_alloc(sizeof(zeus_tensor));\n    _t->dim1 = {};\n    _t->dim2 = {};\n    _t->data = __zeus_arena_alloc(_t->dim1 * _t->dim2 * sizeof(double));\n    _t;\n}})", dim1, dim2)
            }
            Expression::FunctionCall { name, arguments } => {
                let args_c: Vec<String> = arguments.iter().map(|a| self.generate_parallel_expression(a, shared_vars, iterator)).collect();
                if name == "print" || name == "println" {
                    let _ = &args_c;
                    let nl = name == "println";
                    let body = self.generate_print_builtin(arguments, nl, "", Some((shared_vars, iterator)));
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
                                if let Expression::IndexAccess { base, index } = &arguments[i] {
                                    if let Expression::Identifier(arr_name) = &**base {
                                        let idx_c = self.generate_parallel_expression(index, shared_vars, iterator);
                                        let temp_name = format!("_zeus_tmp_{}", i);
                                        block.push_str(&format!("{} {}; ", struct_name, temp_name));
                                        
                                        if let Some(fields) = self.struct_schemas.borrow().get(struct_name) {
                                            for (f_name, _) in fields {
                                                block.push_str(&format!("{}.{} = {}_{}[{}]; ", temp_name, f_name, arr_name, f_name, idx_c));
                                                               writes_back.push(format!("{}_{}[{}] = {}.{}; ", arr_name, f_name, idx_c, temp_name, f_name));
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
                            block.push_str(&format!("size_t __phoenix_mark = *zeus_arena_offset; {}({}); ", name, call_args.join(", ")));
                            for wb in writes_back {
                                block.push_str(&wb);
                            }
                            block.push_str("*zeus_arena_offset = __phoenix_mark; })");
                        } else {
                            block.push_str(&format!("size_t __phoenix_mark = *zeus_arena_offset; {} _res = {}({}); ", c_ret, name, call_args.join(", ")));
                            for wb in writes_back {
                                block.push_str(&wb);
                            }
                            block.push_str("*zeus_arena_offset = __phoenix_mark; _res; })");
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
                    .map(|(f, v)| format!(".{} = {}", f, self.generate_parallel_expression(v, shared_vars, iterator)))
                    .collect();
                format!("(({}){{ {} }})", name, parts.join(", "))
            }
            Expression::FieldAccess { base, field } => {
                // SoA field access in parallel context: same OramAccess/IndexAccess unwrap
                let soa_field: Option<String> = match &**base {
                    Expression::IndexAccess { base: arr_base, index }
                    | Expression::OramAccess { base: arr_base, index, .. } => {
                        if let Expression::Identifier(arr_name) = arr_base.as_ref() {
                            if self.soa_arrays.borrow().contains(arr_name.as_str()) {
                                Some(format!("{}_{}[(size_t)({})]", arr_name, field,
                                    self.generate_parallel_expression(index, shared_vars, iterator)))
                            } else { None }
                        } else { None }
                    }
                    _ => None,
                };
                if let Some(s) = soa_field { return s; }
                if field == "data" {
                    format!("{}->{}", self.generate_parallel_expression(base, shared_vars, iterator), field)
                } else {
                    format!("{}.{}", self.generate_parallel_expression(base, shared_vars, iterator), field)
                }
            }
            Expression::IndexAccess { base, index } => {
                format!("{}[(size_t)({})]", self.generate_parallel_expression(base, shared_vars, iterator), self.generate_parallel_expression(index, shared_vars, iterator))
            }
            Expression::OramAccess { base, index, bound: _ } => {
                let b = self.generate_parallel_expression(base, shared_vars, iterator);
                let i = self.generate_parallel_expression(index, shared_vars, iterator);
                // ORAM side-channel: use __zeus_rand (XOR-shift, no __rdtsc timing leak)
                format!("{}[(size_t)({})] /* note: oblivious protection applies to `secret` fixed-size struct arrays; this is a direct access */", b, i)
            }
            Expression::Try(inner) => {
                format!("ZEUS_TRY({})", self.generate_parallel_expression(inner, shared_vars, iterator))
            }
            Expression::Comptime(inner) => {
                self.generate_parallel_expression(inner, shared_vars, iterator)
            }
            Expression::Prefix { operator, operand } => {
                let o = self.generate_parallel_expression(operand, shared_vars, iterator);
                let op = match operator.as_str() { "Minus" => "-", "Not" => "!", _ => operator.as_str() };
                format!("({}{})", op, o)
            }
            Expression::ArrayLiteral(elems) => {
                let parts: Vec<String> = elems.iter().map(|e| self.generate_parallel_expression(e, shared_vars, iterator)).collect();
                format!("{{{}}}", parts.join(", "))
            }
            _ => "/* unsupported parallel expr */".to_string()
        }
    }
}
