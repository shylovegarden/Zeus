use std::io::{self, BufRead, Read, Write};
use serde_json::Value;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::analyzer::SemanticAnalyzer;
use crate::energy_profiler::EnergyProfiler;

pub fn run_lsp() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    
    loop {
        // Read headers
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                return; // EOF
            }
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            if line.starts_with("Content-Length:") {
                let parts: Vec<&str> = line.split(':').collect();
                content_length = parts[1].trim().parse().unwrap();
            }
        }
        
        if content_length == 0 { continue; }
        
        // Read body
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        let body_str = String::from_utf8(body).unwrap();
        
        let req: Value = match serde_json::from_str(&body_str) {
            Ok(v) => v,
            Err(_) => continue,
        };
        
        let method = req["method"].as_str().unwrap_or("");
        
        if method == "initialize" {
            let id = req["id"].clone();
            let resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "capabilities": {
                        "textDocumentSync": 1, // Full sync
                        "completionProvider": {
                            "resolveProvider": false,
                            "triggerCharacters": ["."]
                        }
                    }
                }
            });
            send_response(resp);
        } else if method == "textDocument/didOpen" || method == "textDocument/didChange" {
            let uri = if method == "textDocument/didOpen" {
                req["params"]["textDocument"]["uri"].as_str().unwrap_or("").to_string()
            } else {
                req["params"]["textDocument"]["uri"].as_str().unwrap_or("").to_string()
            };
            
            let text = if method == "textDocument/didOpen" {
                req["params"]["textDocument"]["text"].as_str().unwrap_or("").to_string()
            } else {
                let changes = &req["params"]["contentChanges"];
                if let Some(arr) = changes.as_array() {
                    if !arr.is_empty() {
                        arr[0]["text"].as_str().unwrap_or("").to_string()
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                }
            };
            
            publish_diagnostics(&uri, &text);
        } else if method == "textDocument/completion" {
            let id = req["id"].clone();
            
            // For now, provide basic static autocomplete for the standard library
            let items = serde_json::json!([
                {
                    "label": "println",
                    "kind": 3, // Function
                    "detail": "fn println(val: f64) -> void",
                    "documentation": "Prints a floating point number to stdout."
                },
                {
                    "label": "sha256_init",
                    "kind": 3, // Function
                    "detail": "fn sha256_init() -> Sha256State",
                    "documentation": "Initializes a SHA-256 state struct."
                },
                {
                    "label": "chacha20_quarter_round",
                    "kind": 3, // Function
                    "detail": "fn chacha20_quarter_round(a: f64, b: f64, c: f64, d: f64) -> f64",
                    "documentation": "Simulates a ChaCha20 quarter round."
                },
                {
                    "label": "parse_ipv4_frame",
                    "kind": 3, // Function
                    "detail": "fn parse_ipv4_frame() -> IPv4Frame",
                    "documentation": "Parses an IPv4 frame from the static buffer."
                },
                {
                    "label": "crypto",
                    "kind": 9, // Module
                    "detail": "module zeus.crypto"
                },
                {
                    "label": "io",
                    "kind": 9, // Module
                    "detail": "module zeus.io"
                },
                {
                    "label": "net",
                    "kind": 9, // Module
                    "detail": "module zeus.net"
                }
            ]);

            let resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": items
            });
            send_response(resp);
        }
    }
}

fn publish_diagnostics(uri: &str, text: &str) {
    let mut diagnostics = Vec::new();
    
    // Parse
    let lexer = Lexer::new(text);
    let mut parser = Parser::new(lexer);
    let mut program = parser.parse_program();
    
    // Collect Syntax Errors
    for err in parser.errors() {
        diagnostics.push(serde_json::json!({
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 100 } },
            "severity": 1, // Error
            "message": err,
            "source": "zeus"
        }));
    }
    
    // Semantic Analysis
    let mut analyzer = SemanticAnalyzer::new();
    if let Err(e) = analyzer.analyze(&mut program) {
        diagnostics.push(serde_json::json!({
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 100 } },
            "severity": 1, // Error
            "message": e,
            "source": "zeus"
        }));
    }
    
    // Energy Profiler
    let (_, warnings) = EnergyProfiler::analyze_and_get_warnings(&program);
    for warning in warnings {
        diagnostics.push(serde_json::json!({
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 100 } },
            "severity": 2, // Warning
            "message": format!("[ENERGY ALERT] {}", warning),
            "source": "zeus"
        }));
    }
    
    // Vibe Autocomplete Hints (Information-level diagnostics)
    let zero_range = serde_json::json!({
        "start": { "line": 0, "character": 0 },
        "end": { "line": 0, "character": 0 }
    });
    if text.contains("for ") {
        diagnostics.push(serde_json::json!({
            "range": zero_range,
            "severity": 3, // Information
            "message": "⚡ Vibe Tip: Hit Tab on any 'for' loop to convert it to a parallel {} block and use all CPU cores.",
            "source": "zeus"
        }));
    }
    if text.contains(" = ") && !text.contains("mut") {
        diagnostics.push(serde_json::json!({
            "range": zero_range,
            "severity": 3, // Information
            "message": "🔒 Zeus locks memory by default. Add 'mut' if you want to mutate this value.",
            "source": "zeus"
        }));
    }
    if text.contains("0x7E8") {
        diagnostics.push(serde_json::json!({
            "range": zero_range,
            "severity": 3, // Information
            "message": "🚗 Zeus recognizes 0x7E8: this is the standard OBD-II Engine Control Module address.",
            "source": "zeus"
        }));
    }

    let resp = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": diagnostics
        }
    });
    send_response(resp);
}

fn send_response(msg: Value) {
    let s = serde_json::to_string(&msg).unwrap();
    print!("Content-Length: {}\r\n\r\n{}", s.len(), s);
    io::stdout().flush().unwrap();
}
