# Zeus Audit 04 -- Language Design & Developer Experience

Scope: Can a motivated developer write a *real*, non-trivial program in Zeus
today? What language features exist, what's missing, how good are the errors,
and what tooling is there? All findings below were produced by actually writing
small `.zs` programs and running the shipped compiler read-only.

Method: `BIN=/tmp/zeus_target/release/zeus_compiler` (v0.1.0). Every repro was
compiled with `zeus build`. Source was read but never modified; no `cargo build`
was run. Memory-limited runs used `ulimit -v 2000000` to convert the runaway
allocations into fast aborts.

---

## Executive summary

**A motivated developer cannot write a non-trivial program in Zeus today.**
Zeus is best understood not as a general-purpose language but as a *proof-and-
certificate demo* over a tiny C-like subset: typed functions, `let`/`let mut`,
`i32`/`u64`/`f64`/`bool`, `if/else`, `while`, **constant-bounded** `for`, calls,
recursion, fixed-size Structure-of-Arrays buffers, and `println`. Those work and
produce correct native binaries (verified at runtime: `fib(10)=55`, dot-product
neuron `=12`, etc.).

Everything past that "calculator + fixed arrays" tier is missing, and -- worse --
several missing features **crash the compiler instead of reporting an error.**
The four hardest blockers:

1. **No sum types / `match` / generics, and using them aborts the compiler.**
   `enum`, `match`, generic `<T>`, and closures are not even tokens. Writing
   `match x { ... }`, `fn id<T>(x:T)`, or a `|x| {...}` closure sends the parser
   into an unbounded allocation that aborts with *"memory allocation of
   2818572288 bytes failed"* and **no diagnostic** (exit 137/134). A beginner
   gets a 2.8 GB OOM kill, not "unexpected token."

2. **`Result`/`Ok`/`Err` exist in name only.** The AST/codegen knows
   `zeus_result_t`, but a real `-> Result<i32,i32>` function fails to compile
   because the generated C is internally inconsistent (conflicting types,
   `return Ok(...)` becomes a bare `int`). Error-handling-as-values is not
   usable.

3. **No real strings.** `str` is literally `const char*`. `"a" + "b"` fails with
   a leaked C error (*invalid operands to binary `+`*). No length, slicing,
   concat, formatting, or comparison. `println` is typed `f64` only; string
   printing happens to work but is unmodeled.

4. **The type checker barely exists, so wrong programs compile.** A type
   mismatch (`let s: str = f()` where `f()->i32`) builds with only a C *warning*.
   Undefined variables/functions surface as raw **gcc/ld** errors, not Zeus
   errors. A missing `}` produces a working binary with **exit 0**. The parser
   silently drops what it can't parse.

On top of this, three of the documented "gotchas" the team believed were fixed
are **still broken** (digit-leading filenames, functions named `double`), one is
genuinely fixed (non-ASCII), and the module/`import` system only resolves under a
specific scaffolded layout.

Bottom line: Zeus can run the demos in `examples/`. It cannot yet host a program
a developer would actually want to write (a parser, a CLI tool, a data
structure) without immediately hitting a missing feature or a compiler crash.

---

## 1. Language completeness -- have / partial / missing

Legend: **Have** = works + correct runtime. **Partial** = parses/builds but
broken or trivially limited. **Missing** = no support; in the worst cases it
*crashes the compiler*.

| Feature | Status | Repro (and what happens) |
|---|---|---|
| Typed fns, params, return | **Have** | `fn add(x:i32,y:i32)->i32 { return x+y; }` -> 7 |
| `let` / `let mut`, reassign | **Have** | `mutable_var.zs`; runtime correct |
| Scalar ints/floats/bool | **Have** | `i32 u64 f64 bool`; `let b:bool=true` builds & runs |
| `if`/`else` | **Have** | `if x>0 {return 1;} else {return 0;}` -> 1 |
| `while` | **Have** | `while i<5 {i=i+1;}` -> 5 (warns "unbounded energy") |
| `for` over **constant** range | **Have** | `for i in 0..4 {...}` (ex08) -> 12 |
| `for` over **variable** range | **Missing (CRASH)** | `for i in 0..n` -> aborts, *2.8 GB alloc failed*, no binary |
| Recursion | **Have** | `fib(10)` -> 55 |
| Logical `&&` `||` `!` | **Have** | `if a && !b {...}` -> works |
| Comparison ops | **Have** | `<`, `>`, `==` all work |
| Modulo `%` | **Missing** | `x % 3` -> leaked C: *expected expression before `%`* |
| Negative literals | **Have** | `let x:i32=-5` -> -5 |
| Fixed-size SoA buffers | **Have** | `Point[4]; pts[0].x=10` (ex07) -> 37 |
| Structs (multiple, fields) | **Partial** | Only as `Name[N]` SoA buffers + `arr[i].field`; no plain struct value / no struct literal `Point{x:1}` |
| C-style arrays `[i32;3]` | **Have (basic)** | `let a:[i32;3]=[1,2,3]; a[0]` -> 1 |
| Slices / dynamic arrays / `Vec` | **Missing** | no growable collection; SoA is fixed-size only |
| **Enums / sum types** | **Missing (CRASH)** | `enum Color{...}; Color::Red` -> *Cannot initialize unknown struct 'Color'*; even bare `enum` -> same error |
| **Pattern matching `match`** | **Missing (CRASH)** | `match x {0=>...}` -> aborts, 2.8 GB alloc, **no diagnostic** |
| **`Result`/`Ok`/`Err`** | **Partial/broken** | `-> Result<i32,i32>` -> leaked C: *conflicting types*, *incompatible return type* |
| Real strings + string ops | **Missing** | `str` == `const char*`; `"a"+"b"` -> C error; no len/concat/slice/cmp |
| **Generics `<T>`** | **Missing (CRASH)** | `fn id<T>(x:T)->T` -> aborts, 2.8 GB alloc, no diagnostic |
| **Closures / lambdas** | **Missing (CRASH)** | `let f = |x:i32| {...}` -> aborts, 2.8 GB alloc |
| Modules / `import` | **Partial/fragile** | works only from scaffolded root as `import zeus.hw`; `import std.io` -> *not found at std/std/io.zs* (path double-prefixes `std/`) |
| Traits / interfaces / methods | **Missing** | no `impl`/`trait` tokens; no method syntax |
| Tuples | **Missing** | not parsed |

### Notable repros

**`match` aborts the compiler with no error:**
```
$ zeus build t_match.zs        # fn classify(x:i32)->i32 { match x { 0=>100, _=>300 } }
[ZEUS BUILD] Compiling t_match.zs
memory allocation of 2818572288 bytes failed   # then SIGABRT, exit 134/137
```
Root cause (read-only source inspection): `match` and `enum` are **not lexer
tokens** at all (`grep "match"/"enum"` in `lexer.rs` -> nothing). They lex as
identifiers; the expression parser sees `identifier { ... }` and loops, growing
an allocation until the process is killed. Same mechanism for generics and
closures.

**`Result` is half-wired:**
```
t_result.zs:1: error: conflicting types for 'div'; have 'zeus_result_t(int32_t,int32_t)'
t_result.zs:3: error: incompatible types when returning type 'int' but 'zeus_result_t' was expected
```
The `zeus_result_t` machinery exists in codegen but `Ok(...)`/`Err(...)` don't
lower to it, so error-handling-as-values is unusable.

---

## 2. Error message quality

Verdict: **poor for anything outside the modeled subset.** When Zeus's own
analyzer catches a problem (arity), the message is excellent. For the most common
beginner mistakes, Zeus either (a) leaks the underlying **gcc/ld** error, (b)
emits only a *warning* and builds anyway, or (c) silently produces a binary.

| Mistake | What you get | Line #? | Beginner-clear? |
|---|---|---|---|
| Wrong arity | `[ZEUS ERROR] call to 'add' has 1 argument(s) but it is defined with 2` | n/a | **Yes -- excellent** |
| Undefined variable | raw gcc: `error: 'x' undeclared (first use in this function)` | yes (C line) | No -- it's a C error in disguise |
| Undefined function | gcc *warning* + `ld: undefined reference to 'foo'` | partial | **No** -- no Zeus error, just a linker failure |
| Type mismatch `str = i32` | only a C *warning* (`cast to pointer from integer`), **build succeeds** | yes | **No** -- wrong program compiles |
| Missing `}` | nothing; **binary produced, exit 0** | no | **No** -- silently wrong |
| Missing `;` | nothing (semicolons optional); builds | n/a | n/a |
| `match` / generics / closures | 2.8 GB OOM abort, no message | no | **No -- catastrophic** |
| `%` operator | leaked C: `expected expression before '%' token` | yes (C) | No |

Representative leaked-C error (undefined var):
```
e_undefvar.zs: In function 'main':
e_undefvar.zs:2:34: error: 'x' undeclared (first use in this function)
[ZEUS ERROR] Clang Compilation Failed.
```
The good arity error shows Zeus *can* produce excellent diagnostics -- the
analyzer just covers very little. Most errors fall through to the C backend,
which exposes the "Trojan Horse C-Bridge" implementation detail to the user.

**Severity for adoption: Critical.** Silent mis-compiles (type mismatch, missing
brace) and OOM-on-typo destroy the feedback loop a beginner depends on.

---

## 3. Tooling

| Tool | Exists? | Notes |
|---|---|---|
| Compiler `build`/`run` | **Yes** | Works; `-O2 -march=native`; emits native binary + `.zcert` + provenance |
| `zeus fmt` | **Yes (basic)** | `fmt examples/ex03_functions.zs` -> *Formatted ...* (in-place). Crashes (`unwrap` panic) if run on a path it can't open -- fragile error handling |
| `zeus test` | **Yes (stub)** | Runs "native test blocks"; on a file with none -> *No tests found*. No assertion library surfaced |
| `zeus init` (project scaffold) | **Yes** | Creates `src/main.zs`, `src/std/zeus/{core,hw}.zs`, `zeus.toml`. **Tells you to run `cargo run -- build`** -- leaks dev workflow |
| Package manager | **No** | `zeus.toml` has `[package]` + an `energy_high_score` but **no `[dependencies]`**. No fetch/resolve/registry |
| `zeus lsp` | **Present** | LSP daemon command exists (`lsp.rs`); started cleanly in a smoke test. Editor integration depth not verified |
| REPL | **No** | `zeus repl` is not a command (prints usage) |
| Debugger | **No** | No debug command; native binary is debuggable via gdb only as plain C output |
| `audit`/`cert`/`verify`/`wasm`/`doc` | **Yes** | The certificate/proof tooling is the mature part of the product (out of DX scope here) |

So: there *is* a formatter, an LSP, a test runner stub, and a project
scaffolder, but **no package manager, no REPL, no debugger**, and the scaffold
points users at `cargo` rather than `zeus`.

---

## 4. Ergonomics gaps that block real use (gotcha status verified)

| Gotcha | Handbook says | Verified status (today) |
|---|---|---|
| Digit-leading filename `01_foo.zs` | "fails to build" (known) | **STILL BROKEN** -- `01_foo.h:2:9: error: macro names must be identifiers` + `unknown type name 'zeus_tensor'`. Not fixed |
| Function named `double` | "collides in C bridge" (known) | **STILL BROKEN** -- `error: two or more data types in declaration specifiers`. Any C type name (`int`, `char`, `float`...) presumably collides |
| Non-ASCII bytes (em-dash, smart quote) | "now handled cleanly" | **FIXED** -- em-dash in a comment builds successfully, no crash |
| `--require` needs `=` | documented | not re-tested; design wart |
| Bare `3` is a float, must type ints | documented | confirmed in practice (`f64` printed `7.000000`); error budgets/counters need explicit `i32` |
| `import` path resolution | implied to work | **Fragile** -- resolves `std/<dotted-path-with-/>.zs` from the source dir, double-prefixing `std/`. Only the scaffold's `import zeus.hw` layout works |
| Silent parser drops | not documented | The parser has many `return None` paths and a hand-tuned `advance_after_statement` guard; unrecognized constructs are dropped silently or loop to OOM rather than erroring |

The two "C bridge name collision" classes (digit filenames, C-type-name
functions) are *leaky-abstraction* bugs: the user is punished for the compiler's
internal choice to generate C. They should be invisible (name-mangle / sanitize),
and they were believed fixed but are not.

---

## Gap register (severity for adoption . evidence . fix . effort)

| # | Gap | Severity | Evidence | Recommended fix | Effort |
|---|---|---|---|---|---|
| G1 | `match`/`enum`/generics/closures **OOM-abort** the compiler | **Critical** | 2.8 GB alloc abort, exit 134/137, no diagnostic; not lexer tokens | At minimum: add tokens + emit a clean "not yet supported" parse error so it never loops. Then implement enums+`match` | Crash-guard: S. Real enums/match: L |
| G2 | No sum types + pattern matching | **Critical** | `enum` -> *Cannot initialize unknown struct* | Tagged-union enums, `match` with exhaustiveness | L |
| G3 | `Result`/`Ok`/`Err` not usable | **Critical** | `-> Result<i32,i32>` -> conflicting C types | Finish lowering `Ok/Err` to `zeus_result_t`; add `?`/match on it | M |
| G4 | No real strings / string ops | **Critical** | `str`==`const char*`; `"a"+"b"` C error | Owned/borrowed string type + len/concat/slice/cmp/format | L |
| G5 | Type checker too weak; wrong code compiles | **Critical** | `str = i32` warns only; missing `}` builds, exit 0 | Real type/decl checking in `analyzer.rs` before C handoff; reject on error | M-L |
| G6 | Errors leak gcc/ld output; no line-mapped Zeus diagnostics | **High** | undefined var/fn -> raw gcc/ld | Resolve names + map source spans in Zeus; never surface C errors | M |
| G7 | `for` over a runtime bound aborts | **High** | `for i in 0..n` -> 2.8 GB abort | Support runtime loop bounds (degrade WCET to "unbounded", don't crash) | M |
| G8 | No dynamic collections (Vec/slice/map) | **High** | only fixed `Name[N]` SoA | Heap-backed growable types behind a feature/region model | L |
| G9 | `%` operator missing | **Medium** | `x % 3` -> C parse error | Add `%` to lexer/parser/codegen | S |
| G10 | C-bridge name collisions (digit files, `double`/type-name fns) | **Medium** | both repro today | Name-mangle/sanitize all generated C identifiers + guards | S |
| G11 | `import`/module resolution fragile + double-prefixes `std/` | **Medium** | `import std.io` -> *not found at std/std/io.zs* | Define one resolution rule; search project root + stdlib path | M |
| G12 | No package manager (`zeus.toml` has no deps) | **Medium** | manifest has only `[package]` | `[dependencies]` + fetch/lock/resolve | L |
| G13 | No REPL / debugger; scaffold tells users to run `cargo` | **Low** | `zeus repl` not a command; `init` prints `cargo run -- build` | REPL via existing VM; fix scaffold message | M |
| G14 | `fmt` panics on unreadable path | **Low** | `unwrap()` panic at main.rs:1303 | Handle IO errors gracefully | S |

Effort key: S = days, M = 1-3 weeks, L = 1+ month.

---

## Top-8 "build these to be usable" (ranked)

1. **Crash-guard the parser (G1).** No input should ever OOM the compiler. Add
   `match`/`enum` tokens and make every unsupported construct emit one clean,
   line-numbered "not yet supported" error. This single change converts the
   worst beginner experience (2.8 GB OOM) into a normal error. *Highest ROI.*
2. **Make the type checker actually reject wrong programs (G5).** Type
   mismatches and missing braces must fail the build, not produce a silent
   binary. Without this, no error message work matters.
3. **Real strings + basic ops (G4).** len, concat, slice, compare, format. You
   cannot write a useful program without strings.
4. **Sum types + `match` with exhaustiveness (G2).** The defining feature of a
   modern language; also unblocks idiomatic error handling.
5. **Finish `Result`/`Ok`/`Err` + a `?`/match story (G3).** Error-handling-as-
   values is half-built in the AST; complete the lowering.
6. **First-class Zeus diagnostics, never raw gcc/ld (G6).** Resolve
   names/types in Zeus and report against `.zs` spans. Hides the C bridge.
7. **Dynamic collections (G8) + runtime-bounded `for` (G7).** A growable list
   and loops over computed bounds are table stakes for "real programs."
8. **Fix the leaky C-bridge gotchas (G10) and module resolution (G11).**
   Mangle generated identifiers so filenames/function names never collide, and
   make `import` resolve predictably from the project root.

Everything in this list is about turning Zeus from a *certificate demo over a
calculator subset* into a language a developer can write a first non-trivial
program in. The proof/certificate machinery (the project's actual differentiator)
is comparatively mature; the **language and DX layer underneath it is the gating
constraint for adoption.**
