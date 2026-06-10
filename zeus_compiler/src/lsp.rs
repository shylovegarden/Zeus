#![allow(clippy::if_same_then_else, clippy::collapsible_if, clippy::collapsible_else_if, clippy::map_unwrap_or, clippy::needless_bool)]
use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use serde_json::Value;
use crate::ast::{Statement, Type};
use crate::lexer::{Lexer, Token};
use crate::parser::Parser;
use crate::analyzer::SemanticAnalyzer;
use crate::energy_profiler::EnergyProfiler;

// ── Symbol table ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    /// Human-readable type string shown on hover.
    pub type_str: String,
    /// Declaration position (LSP 0-based line).
    pub decl_line: u32,
    pub decl_col: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind { Variable, Function, Parameter, Struct, Field }

fn type_str(t: &Option<Type>) -> String {
    match t {
        None => "void".into(),
        Some(ty) => type_str_inner(ty),
    }
}

fn type_str_inner(t: &Type) -> String {
    match t {
        Type::I8 => "i8".into(),   Type::I32 => "i32".into(),
        Type::U64 => "u64".into(), Type::F32 => "f32".into(),
        Type::F64 => "f64".into(), Type::Bool => "bool".into(),
        Type::Struct(n) => n.clone(),
        Type::Array(b, _) => format!("[{}]", type_str_inner(b)),
        Type::Tensor { .. } => "tensor".into(),
        Type::Pointer(b) => format!("*{}", type_str_inner(b)),
        Type::Result(ok, err) => format!("Result<{}, {}>", type_str_inner(ok), type_str_inner(err)),
        Type::Unknown(n) => n.clone(),
        Type::TypeParam(n) => n.clone(),
    }
}

/// Collect every declaration from the AST. Positions come from the token stream
/// (re-scanned here) because AST nodes don't carry spans yet.
pub fn build_symbol_table(source: &str) -> Vec<Symbol> {
    let mut symbols: Vec<Symbol> = Vec::new();
    // Re-lex to collect token positions into a map: name -> first occurrence line/col
    let mut token_positions: HashMap<String, (u32, u32)> = HashMap::new();
    {
        let mut lex = Lexer::new(source);
        loop {
            let tok = lex.next_token();
            if tok == Token::Eof { break; }
            if let Token::Identifier(ref id) = tok {
                token_positions.entry(id.clone()).or_insert((
                    (lex.token_line.saturating_sub(1)) as u32,
                    (lex.token_col.saturating_sub(1)) as u32,
                ));
            }
        }
    }

    let get_pos = |name: &str| -> (u32, u32) {
        token_positions.get(name).copied().unwrap_or((0, 0))
    };

    // Walk the parsed AST for structural declarations.
    let lexer2 = Lexer::new(source);
    let mut parser = Parser::new(lexer2);
    let program = parser.parse_program();

    for stmt in &program.statements {
        collect_stmt_symbols(stmt, &mut symbols, &get_pos);
    }
    symbols
}

fn collect_stmt_symbols<F>(stmt: &Statement, out: &mut Vec<Symbol>, get_pos: &F)
where F: Fn(&str) -> (u32, u32)
{
    match stmt {
        Statement::Let { name, var_type, .. } => {
            let (l, c) = get_pos(name);
            out.push(Symbol {
                name: name.clone(),
                kind: SymbolKind::Variable,
                type_str: type_str(var_type),
                decl_line: l, decl_col: c,
            });
        }
        Statement::FunctionDeclaration { name, parameters, return_type, body, .. } => {
            let (l, c) = get_pos(name);
            let param_sig: Vec<String> = parameters.iter()
                .map(|(pn, pt)| format!("{}: {}", pn, type_str_inner(pt)))
                .collect();
            out.push(Symbol {
                name: name.clone(),
                kind: SymbolKind::Function,
                type_str: format!("fn({}) -> {}", param_sig.join(", "), type_str(return_type)),
                decl_line: l, decl_col: c,
            });
            for (pn, pt) in parameters {
                let (pl, pc) = get_pos(pn);
                out.push(Symbol {
                    name: pn.clone(),
                    kind: SymbolKind::Parameter,
                    type_str: type_str_inner(pt),
                    decl_line: pl, decl_col: pc,
                });
            }
            for s in body { collect_stmt_symbols(s, out, get_pos); }
        }
        Statement::StructDeclaration { name, fields, .. } => {
            let (l, c) = get_pos(name);
            out.push(Symbol {
                name: name.clone(),
                kind: SymbolKind::Struct,
                type_str: format!("struct {}", name),
                decl_line: l, decl_col: c,
            });
            for (fn_, ft) in fields {
                let (fl, fc) = get_pos(fn_);
                out.push(Symbol {
                    name: fn_.clone(),
                    kind: SymbolKind::Field,
                    type_str: type_str_inner(ft),
                    decl_line: fl, decl_col: fc,
                });
            }
        }
        Statement::If { consequence, alternative, .. } => {
            for s in consequence { collect_stmt_symbols(s, out, get_pos); }
            if let Some(alt) = alternative { for s in alt { collect_stmt_symbols(s, out, get_pos); } }
        }
        Statement::For { body, .. } | Statement::While { body, .. } => {
            for s in body { collect_stmt_symbols(s, out, get_pos); }
        }
        _ => {}
    }
}

/// Find the identifier word at (line, col) in the source (both 0-based LSP coords).
fn word_at(source: &str, line: u32, col: u32) -> Option<String> {
    let src_line = source.lines().nth(line as usize)?;
    let col = col as usize;
    if col > src_line.len() { return None; }
    let start = src_line[..col].rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + 1).unwrap_or(0);
    let end = src_line[col..].find(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + col).unwrap_or(src_line.len());
    if start >= end { return None; }
    let word = &src_line[start..end];
    if word.is_empty() { None } else { Some(word.to_string()) }
}

// ── Open-document cache ──────────────────────────────────────────────────────

use std::sync::Mutex;
static DOC_CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

fn cache_document(uri: &str, text: &str) {
    let mut lock = DOC_CACHE.lock().unwrap();
    if lock.is_none() { *lock = Some(HashMap::new()); }
    lock.as_mut().unwrap().insert(uri.to_string(), text.to_string());
}

fn get_document(uri: &str) -> Option<String> {
    let lock = DOC_CACHE.lock().unwrap();
    lock.as_ref()?.get(uri).cloned()
}

// ── LSP server loop ──────────────────────────────────────────────────────────

pub fn run_lsp() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 { return; }
            let line = line.trim();
            if line.is_empty() { break; }
            if line.starts_with("Content-Length:") {
                content_length = line.split(':').nth(1)
                    .and_then(|s| s.trim().parse().ok()).unwrap_or(0);
            }
        }
        if content_length == 0 { continue; }

        let mut body = vec![0u8; content_length];
        if reader.read_exact(&mut body).is_err() { return; }
        let body_str = match String::from_utf8(body) { Ok(s) => s, Err(_) => continue };
        let req: Value = match serde_json::from_str(&body_str) { Ok(v) => v, Err(_) => continue };
        let method = req["method"].as_str().unwrap_or("");

        match method {
            "initialize" => {
                let id = req["id"].clone();
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {
                            "textDocumentSync": 1,
                            "hoverProvider": true,
                            "definitionProvider": true,
                            "completionProvider": {
                                "resolveProvider": false,
                                "triggerCharacters": ["."]
                            }
                        }
                    }
                });
                send_response(resp);
            }
            "textDocument/didOpen" => {
                let uri  = req["params"]["textDocument"]["uri"].as_str().unwrap_or("").to_string();
                let text = req["params"]["textDocument"]["text"].as_str().unwrap_or("").to_string();
                cache_document(&uri, &text);
                publish_diagnostics(&uri, &text);
            }
            "textDocument/didChange" => {
                let uri = req["params"]["textDocument"]["uri"].as_str().unwrap_or("").to_string();
                let text = req["params"]["contentChanges"][0]["text"].as_str().unwrap_or("").to_string();
                cache_document(&uri, &text);
                publish_diagnostics(&uri, &text);
            }
            "textDocument/hover" => {
                let id   = req["id"].clone();
                let uri  = req["params"]["textDocument"]["uri"].as_str().unwrap_or("");
                let line = req["params"]["position"]["line"].as_u64().unwrap_or(0) as u32;
                let col  = req["params"]["position"]["character"].as_u64().unwrap_or(0) as u32;
                let resp = handle_hover(id, uri, line, col);
                send_response(resp);
            }
            "textDocument/definition" => {
                let id   = req["id"].clone();
                let uri  = req["params"]["textDocument"]["uri"].as_str().unwrap_or("");
                let line = req["params"]["position"]["line"].as_u64().unwrap_or(0) as u32;
                let col  = req["params"]["position"]["character"].as_u64().unwrap_or(0) as u32;
                let resp = handle_definition(id, uri, line, col);
                send_response(resp);
            }
            "textDocument/completion" => {
                let id = req["id"].clone();
                let uri = req["params"]["textDocument"]["uri"].as_str().unwrap_or("");
                let resp = handle_completion(id, uri);
                send_response(resp);
            }
            _ => {}
        }
    }
}

fn handle_hover(id: Value, uri: &str, line: u32, col: u32) -> Value {
    if let Some(text) = get_document(uri) {
        if let Some(word) = word_at(&text, line, col) {
            let symbols = build_symbol_table(&text);
            if let Some(sym) = symbols.iter().find(|s| s.name == word) {
                return serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "contents": {
                            "kind": "markdown",
                            "value": format!("```zeus\n{}\n```\n\n*{}*",
                                sym.type_str,
                                match sym.kind {
                                    SymbolKind::Function  => "function",
                                    SymbolKind::Variable  => "variable",
                                    SymbolKind::Parameter => "parameter",
                                    SymbolKind::Struct    => "struct",
                                    SymbolKind::Field     => "field",
                                })
                        }
                    }
                });
            }
        }
    }
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": null })
}

fn handle_definition(id: Value, uri: &str, line: u32, col: u32) -> Value {
    if let Some(text) = get_document(uri) {
        if let Some(word) = word_at(&text, line, col) {
            let symbols = build_symbol_table(&text);
            if let Some(sym) = symbols.iter().find(|s| s.name == word) {
                return serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "uri": uri,
                        "range": {
                            "start": { "line": sym.decl_line, "character": sym.decl_col },
                            "end":   { "line": sym.decl_line, "character": sym.decl_col + (sym.name.len() as u32) }
                        }
                    }
                });
            }
        }
    }
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": null })
}

fn handle_completion(id: Value, uri: &str) -> Value {
    let mut items: Vec<Value> = Vec::new();

    // Dynamic completions from the open document
    if let Some(text) = get_document(uri) {
        let symbols = build_symbol_table(&text);
        for sym in &symbols {
            let kind = match sym.kind {
                SymbolKind::Function  => 3,
                SymbolKind::Struct    => 7,
                SymbolKind::Variable | SymbolKind::Parameter => 6,
                SymbolKind::Field     => 5,
            };
            items.push(serde_json::json!({
                "label": sym.name,
                "kind": kind,
                "detail": sym.type_str
            }));
        }
    }

    // Static stdlib completions
    let stdlib = [
        ("println",  3, "fn println(val: f64)"),
        ("print",    3, "fn print(val: f64)"),
        ("read_file",3, "fn read_file(path: str) -> str"),
        ("write_file",3,"fn write_file(path: str, data: str) -> bool"),
        ("Ok",       3, "fn Ok<T>(val: T) -> Result<T, _>"),
        ("Err",      3, "fn Err<E>(val: E) -> Result<_, E>"),
        ("unwrap",   3, "fn unwrap<T>(r: Result<T,_>) -> T"),
        ("unwrap_or",3, "fn unwrap_or<T>(r: Result<T,_>, default: T) -> T"),
        ("abs",      3, "fn abs(x) -> T"),
        ("min",      3, "fn min(a, b) -> T"),
        ("max",      3, "fn max(a, b) -> T"),
        ("sqrt",     3, "fn sqrt(x: f64) -> f64"),
        ("pow",      3, "fn pow(base: f64, exp: f64) -> f64"),
    ];
    for (label, kind, detail) in &stdlib {
        items.push(serde_json::json!({ "label": label, "kind": kind, "detail": detail }));
    }

    serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": items })
}

fn publish_diagnostics(uri: &str, text: &str) {
    let mut diagnostics = Vec::new();

    let lexer = Lexer::new(text);
    let mut parser = Parser::new(lexer);
    let mut program = parser.parse_program();

    // Syntax errors — now with real line/col from the new parser
    for err in parser.errors() {
        let (line, col, msg) = parse_diag_lsp(err);
        diagnostics.push(serde_json::json!({
            "range": {
                "start": { "line": line, "character": col },
                "end":   { "line": line, "character": col + 1 }
            },
            "severity": 1,
            "message": msg,
            "source": "zeus"
        }));
    }

    // Semantic errors
    let mut analyzer = SemanticAnalyzer::new();
    if let Err(e) = analyzer.analyze(&mut program) {
        diagnostics.push(serde_json::json!({
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 100 } },
            "severity": 1,
            "message": e,
            "source": "zeus"
        }));
    }

    // Energy warnings
    let (_, warnings) = EnergyProfiler::analyze_and_get_warnings(&program);
    for warning in warnings {
        diagnostics.push(serde_json::json!({
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } },
            "severity": 2,
            "message": format!("[ENERGY ALERT] {}", warning),
            "source": "zeus"
        }));
    }

    let resp = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": { "uri": uri, "diagnostics": diagnostics }
    });
    send_response(resp);
}

/// Parse "L:C: msg" or "line L: msg" → (0-based line, 0-based col, message) for LSP.
fn parse_diag_lsp(e: &str) -> (u32, u32, String) {
    let parts: Vec<&str> = e.splitn(3, ':').collect();
    if parts.len() == 3 {
        if let (Ok(l), Ok(c)) = (parts[0].trim().parse::<u32>(), parts[1].trim().parse::<u32>()) {
            return (l.saturating_sub(1), c.saturating_sub(1), parts[2].trim().to_string());
        }
    }
    if let Some(rest) = e.strip_prefix("line ") {
        if let Some(colon) = rest.find(':') {
            let n: u32 = rest[..colon].trim().parse().unwrap_or(1);
            return (n.saturating_sub(1), 0, rest[colon+1..].trim().to_string());
        }
    }
    (0, 0, e.to_string())
}

fn send_response(msg: Value) {
    let s = serde_json::to_string(&msg).unwrap();
    print!("Content-Length: {}\r\n\r\n{}", s.len(), s);
    io::stdout().flush().unwrap();
}
