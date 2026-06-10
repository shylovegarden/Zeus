// The Zeus Anti-Bloat Enforcer
// Physically prevents legacy paradigms from creeping into the Zeus ecosystem.

use crate::ast::Statement;

pub fn enforce_zero_bloat(program: &crate::ast::Program, c_source: &str) {
    let banned_symbols = vec![
        "malloc",
        "calloc",
        "free",
        "pthread_create",
        "pthread_mutex_lock",
    ];

    // AST-level enforcement: Catch malicious aliasing of banned OS functions
    for stmt in &program.statements {
        if let Statement::ExternFunctionDeclaration { name, .. } = stmt {
            for banned in &banned_symbols {
                if name.contains(banned) {
                    eprintln!("\n\x1b[31m[ZEUS ENFORCER PANIC]: 💀 CRITICAL MANIFESTO VIOLATION DETECTED 💀\x1b[0m");
                    eprintln!("The Anti-Bloat Enforcer detected a banned legacy software paradigm via AST inspection.");
                    eprintln!("Banned Symbol Detected: '{}'", banned);
                    eprintln!("Zeus Architecture strictly prohibits dynamic memory allocation and OS-level threads.");
                    eprintln!("Build Assassinated.\n");
                    std::process::exit(1);
                }
            }
        }
    }

    // Textual-level enforcement against the generated C source.
    // These patterns must NEVER appear in Zeus output.
    // Bare `malloc` / `calloc` / `free` calls are a Zero-Heap violation.
    // `#include <pthread.h>` violates the NO-PTHREADS mandate.
    // `#include <stdlib.h>` pulls in malloc/free declarations (MISRA 21.3).
    let banned_patterns: &[(&str, &str)] = &[
        ("malloc(",         "MISRA 21.3 / Zero-Heap: dynamic allocation banned"),
        ("calloc(",         "MISRA 21.3 / Zero-Heap: dynamic allocation banned"),
        ("free(",           "MISRA 21.3 / Zero-Heap: deallocation banned"),
        ("#include <pthread.h>", "MANIFESTO sec.69: OS-level threads banned -- use ucontext fibers"),
        ("#include <stdlib.h>", "MANIFESTO / MISRA 21.3: stdlib.h exposes malloc/calloc/free -- use stddef.h"),
    ];

    for (pattern, reason) in banned_patterns {
        if c_source.contains(pattern) {
            let context_line = c_source.lines()
                .find(|l| l.contains(pattern))
                .unwrap_or("");
            eprintln!("\n\x1b[31m[ZEUS ENFORCER PANIC]: 💀 CRITICAL MANIFESTO VIOLATION DETECTED 💀\x1b[0m");
            eprintln!("The Anti-Bloat Enforcer found a banned pattern in the generated C output.");
            eprintln!("  Pattern : '{}'", pattern);
            eprintln!("  Reason  : {}", reason);
            eprintln!("  Context : '{}'", context_line.trim());
            eprintln!("Build Assassinated.\n");
            std::process::exit(1);
        }
    }
}
