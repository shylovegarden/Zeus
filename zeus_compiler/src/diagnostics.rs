use std::fs;
use std::path::Path;
use std::cmp;

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub line: usize,   // 1-based
    pub column: usize, // 1-based
    pub length: usize, // length of the offending span
    pub source_path: Option<String>,
}

impl Diagnostic {
    pub fn new(severity: Severity, message: String, line: usize, column: usize, length: usize, source_path: Option<String>) -> Self {
        Self { severity, message, line, column, length, source_path }
    }

    pub fn print_pretty(&self) {
        let severity_str = match self.severity {
            Severity::Error => "\x1b[1;31merror\x1b[0m",
            Severity::Warning => "\x1b[1;33mwarning\x1b[0m",
            Severity::Note => "\x1b[1;36mnote\x1b[0m",
        };

        let file_prefix = if let Some(path) = &self.source_path {
            format!("{}:{}:{}", path, self.line, self.column)
        } else {
            format!("<unknown>:{}:{}", self.line, self.column)
        };

        eprintln!("{}: {}", severity_str, self.message);
        eprintln!("  \x1b[1;34m-->\x1b[0m {}", file_prefix);

        if let Some(path) = &self.source_path {
            if let Ok(source) = fs::read_to_string(path) {
                let lines: Vec<&str> = source.lines().collect();
                if self.line > 0 && self.line <= lines.len() {
                    let offending_line = lines[self.line - 1];
                    let line_num_str = format!("{}", self.line);
                    let padding = " ".repeat(line_num_str.len());

                    eprintln!(" {} \x1b[1;34m|\x1b[0m", padding);
                    eprintln!(" {} \x1b[1;34m|\x1b[0m {}", line_num_str, offending_line);
                    
                    let caret_padding = " ".repeat(self.column.saturating_sub(1));
                    let carets = "^".repeat(cmp::max(1, self.length));
                    let color = match self.severity {
                        Severity::Error => "\x1b[1;31m",
                        Severity::Warning => "\x1b[1;33m",
                        Severity::Note => "\x1b[1;36m",
                    };
                    eprintln!(" {} \x1b[1;34m|\x1b[0m {}{}{}\x1b[0m", padding, caret_padding, color, carets);
                    eprintln!(" {} \x1b[1;34m|\x1b[0m", padding);
                }
            }
        }
        eprintln!();
    }
}
