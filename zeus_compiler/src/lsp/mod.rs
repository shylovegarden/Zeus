// Language Server Protocol (LSP) Implementation for Zeus
// Provides IDE features: autocomplete, goto-definition, real-time errors

use lsp_types::*;
use lsp_server::{Connection, Message, Notification, Request, RequestId};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod parser;
mod completion;
mod diagnostics;

use parser::{parse_document, AstCache};
use completion::CompletionProvider;
use diagnostics::DiagnosticEngine;

pub struct ZeusLanguageServer {
    connection: Connection,
    documents: Arc<Mutex<HashMap<Url, String>>>,
    ast_cache: Arc<Mutex<AstCache>>,
    completion_provider: CompletionProvider,
    diagnostic_engine: DiagnosticEngine,
}

impl ZeusLanguageServer {
    pub fn new(connection: Connection) -> Self {
        ZeusLanguageServer {
            connection,
            documents: Arc::new(Mutex::new(HashMap::new())),
            ast_cache: Arc::new(Mutex::new(AstCache::new())),
            completion_provider: CompletionProvider::new(),
            diagnostic_engine: DiagnosticEngine::new(),
        }
    }
    
    /// Run the LSP server
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize
        let (id, params) = self.connection.initialize_start()?;
        
        let init_params: InitializeParams = serde_json::from_value(params).unwrap();
        
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::FULL
            )),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                work_done_progress_options: Default::default(),
                all_commit_characters: None,
                completion_item: None,
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            execute_command_provider: Some(ExecuteCommandOptions {
                commands: vec![
                    "zeus.build".to_string(),
                    "zeus.verify".to_string(),
                    "zeus.run".to_string(),
                ],
                work_done_progress_options: Default::default(),
            }),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(
                    SemanticTokensOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        legend: SemanticTokensLegend {
                            token_types: vec![
                                SemanticTokenType::KEYWORD,
                                SemanticTokenType::FUNCTION,
                                SemanticTokenType::VARIABLE,
                                SemanticTokenType::TYPE,
                                SemanticTokenType::COMMENT,
                                SemanticTokenType::STRING,
                                SemanticTokenType::NUMBER,
                                SemanticTokenType::OPERATOR,
                            ],
                            token_modifiers: vec![
                                SemanticTokenModifier::DECLARATION,
                                SemanticTokenModifier::DEFINITION,
                                SemanticTokenModifier::READONLY,
                                SemanticTokenModifier::STATIC,
                            ],
                        },
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                        range: Some(false),
                        document_selector: None,
                    }
                )
            ),
            ..ServerCapabilities::default()
        };
        
        let server_info = ServerInfo {
            name: "Zeus Language Server".to_string(),
            version: Some("0.1.0".to_string()),
        };
        
        let result = InitializeResult {
            capabilities,
            server_info: Some(server_info),
        };
        
        self.connection.initialize_finish(id, &result)?;
        
        // Main loop
        while let Some(msg) = self.connection.receiver.recv()? {
            match msg {
                Message::Request(req) => self.handle_request(req)?,
                Message::Notification(not) => self.handle_notification(not)?,
                Message::Response(_) => {}
            }
        }
        
        Ok(())
    }
    
    fn handle_request(&mut self, req: Request) -> Result<(), Box<dyn std::error::Error>> {
        match req.method.as_str() {
            "textDocument/completion" => self.handle_completion(req.id, req.params)?,
            "textDocument/hover" => self.handle_hover(req.id, req.params)?,
            "textDocument/definition" => self.handle_definition(req.id, req.params)?,
            "textDocument/documentSymbol" => self.handle_document_symbol(req.id, req.params)?,
            "workspace/executeCommand" => self.handle_execute_command(req.id, req.params)?,
            _ => {}
        }
        Ok(())
    }
    
    fn handle_notification(&mut self, not: Notification) -> Result<(), Box<dyn std::error::Error>> {
        match not.method.as_str() {
            "textDocument/didOpen" => self.handle_did_open(not.params)?,
            "textDocument/didChange" => self.handle_did_change(not.params)?,
            "textDocument/didSave" => self.handle_did_save(not.params)?,
            "textDocument/didClose" => self.handle_did_close(not.params)?,
            _ => {}
        }
        Ok(())
    }
    
    fn handle_completion(&self, id: RequestId, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let params: CompletionParams = serde_json::from_value(params)?;
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        
        let documents = self.documents.lock().unwrap();
        let content = documents.get(&uri).cloned().unwrap_or_default();
        
        // Get completions
        let completions = self.completion_provider.get_completions(
            &content,
            position.line as usize,
            position.character as usize,
        );
        
        let items: Vec<CompletionItem> = completions.into_iter().map(|c| CompletionItem {
            label: c.label,
            kind: Some(c.kind),
            detail: c.detail,
            documentation: c.documentation.map(|d| Documentation::String(d)),
            insert_text: c.insert_text,
            ..CompletionItem::default()
        }).collect();
        
        let result = CompletionResponse::Array(items);
        
        self.connection.sender.send(Message::Response(lsp_server::Response {
            id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        }))?;
        
        Ok(())
    }
    
    fn handle_hover(&self, id: RequestId, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let params: HoverParams = serde_json::from_value(params)?;
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        
        let documents = self.documents.lock().unwrap();
        let content = documents.get(&uri).cloned().unwrap_or_default();
        
        // Get hover info
        let hover_info = self.get_hover_info(&content, position);
        
        let result = hover_info.map(|info| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: info,
            }),
            range: None,
        });
        
        self.connection.sender.send(Message::Response(lsp_server::Response {
            id,
            result: serde_json::to_value(result)?,
            error: None,
        }))?;
        
        Ok(())
    }
    
    fn handle_definition(&self, id: RequestId, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let params: GotoDefinitionParams = serde_json::from_value(params)?;
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        
        let documents = self.documents.lock().unwrap();
        let content = documents.get(&uri).cloned().unwrap_or_default();
        
        // Find definition
        let definitions = self.find_definitions(&content, position);
        
        let result = if definitions.is_empty() {
            None
        } else {
            Some(GotoDefinitionResponse::Array(definitions))
        };
        
        self.connection.sender.send(Message::Response(lsp_server::Response {
            id,
            result: serde_json::to_value(result)?,
            error: None,
        }))?;
        
        Ok(())
    }
    
    fn handle_document_symbol(&self, id: RequestId, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let params: DocumentSymbolParams = serde_json::from_value(params)?;
        let uri = params.text_document.uri;
        
        let documents = self.documents.lock().unwrap();
        let content = documents.get(&uri).cloned().unwrap_or_default();
        
        // Get document symbols
        let symbols = self.get_document_symbols(&content, uri.clone());
        
        let result: DocumentSymbolResponse = DocumentSymbolResponse::Nested(symbols);
        
        self.connection.sender.send(Message::Response(lsp_server::Response {
            id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        }))?;
        
        Ok(())
    }
    
    fn handle_execute_command(&self, id: RequestId, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let params: ExecuteCommandParams = serde_json::from_value(params)?;
        
        let result = match params.command.as_str() {
            "zeus.build" => {
                // Execute zeus build
                Some(Value::String("Building...".to_string()))
            }
            "zeus.verify" => {
                // Execute zeus verify
                Some(Value::String("Verifying...".to_string()))
            }
            "zeus.run" => {
                // Execute zeus run
                Some(Value::String("Running...".to_string()))
            }
            _ => None,
        };
        
        self.connection.sender.send(Message::Response(lsp_server::Response {
            id,
            result,
            error: None,
        }))?;
        
        Ok(())
    }
    
    fn handle_did_open(&self, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let params: DidOpenTextDocumentParams = serde_json::from_value(params)?;
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        
        {
            let mut documents = self.documents.lock().unwrap();
            documents.insert(uri.clone(), content.clone());
        }
        
        // Parse and cache AST
        {
            let mut ast_cache = self.ast_cache.lock().unwrap();
            ast_cache.parse_document(uri.clone(), &content);
        }
        
        // Publish diagnostics
        self.publish_diagnostics(uri, &content)?;
        
        Ok(())
    }
    
    fn handle_did_change(&self, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let params: DidChangeTextDocumentParams = serde_json::from_value(params)?;
        let uri = params.text_document.uri;
        
        // Apply changes
        if let Some(change) = params.content_changes.into_iter().next() {
            let content = change.text;
            
            {
                let mut documents = self.documents.lock().unwrap();
                documents.insert(uri.clone(), content.clone());
            }
            
            // Re-parse
            {
                let mut ast_cache = self.ast_cache.lock().unwrap();
                ast_cache.parse_document(uri.clone(), &content);
            }
            
            // Publish updated diagnostics
            self.publish_diagnostics(uri, &content)?;
        }
        
        Ok(())
    }
    
    fn handle_did_save(&self, _params: Value) -> Result<(), Box<dyn std::error::Error>> {
        // Trigger verification on save
        Ok(())
    }
    
    fn handle_did_close(&self, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let params: DidCloseTextDocumentParams = serde_json::from_value(params)?;
        let uri = params.text_document.uri;
        
        {
            let mut documents = self.documents.lock().unwrap();
            documents.remove(&uri);
        }
        
        {
            let mut ast_cache = self.ast_cache.lock().unwrap();
            ast_cache.remove_document(&uri);
        }
        
        Ok(())
    }
    
    fn publish_diagnostics(&self, uri: Url, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        let diagnostics = self.diagnostic_engine.check(content);
        
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        };
        
        self.connection.sender.send(Message::Notification(Notification {
            method: "textDocument/publishDiagnostics".to_string(),
            params: serde_json::to_value(params)?,
        }))?;
        
        Ok(())
    }
    
    fn get_hover_info(&self, content: &str, position: Position) -> Option<String> {
        // Extract word at position
        let line = content.lines().nth(position.line as usize)?;
        let word = self.extract_word_at_position(line, position.character as usize)?;
        
        // Look up documentation
        self.completion_provider.get_documentation(&word)
    }
    
    fn find_definitions(&self, content: &str, position: Position) -> Vec<Location> {
        // Find where symbol is defined
        vec![]
    }
    
    fn get_document_symbols(&self, content: &str, uri: Url) -> Vec<DocumentSymbol> {
        // Extract all functions, structs, etc.
        vec![]
    }
    
    fn extract_word_at_position(&self, line: &str, col: usize) -> Option<String> {
        let chars: Vec<char> = line.chars().collect();
        let start = chars[..col].iter().rposition(|&c| !c.is_alphanumeric() && c != '_').map(|i| i + 1).unwrap_or(0);
        let end = chars[col..].iter().position(|&c| !c.is_alphanumeric() && c != '_').map(|i| col + i).unwrap_or(chars.len());
        Some(chars[start..end].iter().collect())
    }
}

/// Start the LSP server
pub fn start_lsp_server() -> Result<(), Box<dyn std::error::Error>> {
    // Set up connection
    let (connection, io_threads) = Connection::stdio();
    
    // Create and run server
    let mut server = ZeusLanguageServer::new(connection);
    server.run()?;
    
    // Wait for threads
    io_threads.join()?;
    
    Ok(())
}
