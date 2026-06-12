# Z3 Embedding Design

## Current State

Z3 is currently invoked via subprocess (`std::process::Command::new("z3")`) in `formal_verifier.rs`. This causes:
- Massive filesystem overhead (writing SMT2 files to temp directory)
- Process scheduler overhead (spawning new process for each verification)
- Slow compilation (subprocess latency)

## Proposed Solution

Embed libz3 via C-API using the `z3` crate for direct in-process verification.

## Implementation Plan

### Phase 1: Add Dependency

Add to `zeus_compiler/Cargo.toml`:
```toml
[dependencies]
z3 = { version = "0.12", features = ["static-link-z3"] }
```

### Phase 2: Replace Subprocess Calls

Current code in `formal_verifier.rs`:
```rust
let output = std::process::Command::new("z3")
    .arg("-T:2")
    .arg(tmp.to_str().unwrap())
    .output()
    .map_err(|e| format!("z3 invocation failed: {}", e))?;
```

Replace with:
```rust
use z3::{Config, Context, Solver, SatResult};

let cfg = Config::new();
cfg.set_timeout_msec(2000); // 2 second timeout
let ctx = Context::new(&cfg);
let solver = Solver::new(&ctx);

// Parse SMT2 string directly
solver.assert(&ctx.parse_smt2_string(&smt, None, None, None, None, None));

match solver.check() {
    SatResult::Sat => Err("Counterexample found".to_string()),
    SatResult::Unsat => Ok(()),
    SatResult::Unknown => Ok(()), // Timeout
}
```

### Phase 3: Optimize Context Reuse

Instead of creating a new context for each verification:
- Create a single Z3 context at program startup
- Reuse the context for all verifications
- Reset solver state between verifications
- This eliminates context creation overhead

### Phase 4: Add Caching

The existing cache (`.zeus_verify_cache`) should be enhanced:
- Cache Z3 AST nodes, not just results
- Reuse AST nodes for common subexpressions
- This reduces Z3 parsing overhead

## Performance Benefits

Expected improvements:
- **Subprocess overhead**: Eliminated (no process spawning)
- **Filesystem overhead**: Eliminated (no temp file I/O)
- **Parsing overhead**: Reduced (direct AST construction)
- **Overall**: 10-100x faster verification

## Build Considerations

### Static Linking

Using `static-link-z3` feature:
- Compiles Z3 from source during build
- Increases build time (5-10 minutes)
- Produces standalone binary
- No runtime Z3 dependency

### Dynamic Linking

Without static linking:
- Requires libz3 installed on system
- Faster build time
- Smaller binary
- Runtime dependency on libz3

### Recommendation

Use static linking for distribution, dynamic for development:
```toml
[dependencies]
z3 = { version = "0.12", default-features = false }

[features]
static-z3 = ["z3/static-link-z3"]
```

## Migration Strategy

1. Add feature flag `--use-embedded-z3` to enable new implementation
2. Keep subprocess as fallback for compatibility
3. Benchmark both implementations
4. Switch default to embedded after validation
5. Remove subprocess code after stable release

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_embedded_z3_simple() {
        let ctx = Context::new(&Config::new());
        let solver = Solver::new(&ctx);
        
        let x = ctx.int_const("x");
        solver.assert(&x.gt(&ctx.int_from_i64(0)));
        
        assert_eq!(solver.check(), SatResult::Sat);
    }
    
    #[test]
    fn test_embedded_z3_unsat() {
        let ctx = Context::new(&Config::new());
        let solver = Solver::new(&ctx);
        
        let x = ctx.int_const("x");
        solver.assert(&x.gt(&ctx.int_from_i64(0)));
        solver.assert(&x.lt(&ctx.int_from_i64(0)));
        
        assert_eq!(solver.check(), SatResult::Unsat);
    }
}
```

## Compatibility Notes

- Z3 C-API is stable across versions
- SMT2 syntax parsing is supported
- All existing SMT2 queries should work unchanged
- Timeout handling needs manual implementation

## Alternatives Considered

### 1. Z3 API via FFI
- Pro: More control
- Con: More complex, unsafe code
- Decision: Use `z3` crate (safe wrapper)

### 2. Keep Subprocess with Pool
- Pro: Simpler migration
- Con: Still has overhead
- Decision: Full embedding for maximum performance

### 3. Alternative SMT Solvers (CVC5, Yices)
- Pro: Different tradeoffs
- Con: Not battle-tested with Zeus
- Decision: Stick with Z3 for now

## Status

- **Design**: Complete
- **Implementation**: Not started
- **Blocker**: LLVM build dependency needs resolution first
- **Priority**: High (PERF-001)

## References

- [z3 crate documentation](https://docs.rs/z3/)
- [Z3 C-API documentation](https://z3prover.github.io/api/html/index.html)
- [Z3 SMT2 format](https://z3prover.github.io/api/html/z3__z3__c_8h.html)
