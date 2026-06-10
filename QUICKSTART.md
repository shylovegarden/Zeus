# Zeus Quickstart

Get from zero to a machine-checked safety certificate in under five minutes.

Zeus compiles `.zs` source to readable C, then to a native binary, and emits a
machine-checkable certificate (`.zcert`) recording the safety properties it
proved: zero-heap, constant-time, reproducible, and bounded worst-case
execution time / stack.

## 1. Prerequisites

You need two things on your PATH:

- A Rust toolchain (rustc + cargo). Install via https://rustup.rs.
- A C compiler. Zeus prefers `clang` and falls back to `gcc`. On macOS,
  the Xcode Command Line Tools provide clang (`xcode-select --install`).

Check them:

```
rustc --version
cargo --version
clang --version   # or: gcc --version
```

## 2. Build the compiler

The crate lives in `zeus_compiler/`. From the repo root:

```
cd zeus_compiler
cargo build --release
```

During development Zeus is driven through cargo, so the command form is:

```
cargo run --release -- <command> [file.zs]
```

Everywhere below, read `zeus` as shorthand for `cargo run --release --`.
If you prefer a bare binary, it is at
`zeus_compiler/target/release/zeus_compiler` (see `install.sh`).

## 3. Compile and run a sample

The flagship demos live under `showcase/flagship/`. Build and run the
bounded AI-task demo:

```
cargo run --release -- run showcase/flagship/ai_task_good.zs
```

You will see the build pipeline, then the program's output, then a line like:

```
Certificate: ai_task_good.zcert  [sha256 + per-fn reproducible/constant_time/wcet/stack]
```

## 4. The money moment: certificate + policy gate

Zeus does not just run code; it proves properties and can refuse to run code
that does not prove them.

First, inspect the certificate as a human-readable trust report for the
constant-time crypto demo:

```
cargo run --release -- cert showcase/flagship/crypto_sbox.zs
```

This prints the per-function verdict (zero-heap / reproducible / constant-time
/ fully-bounded). The certificate proves `sbox_mix` has no secret-dependent
timing channel.

Now watch the gate decide. The "good" AI task proves a worst-case execution
bound and is allowed to run; the "bad" one cannot, and Zeus refuses it before
execution:

```
# Bounded task: certificate satisfies the policy, so it runs.
cargo run --release -- run showcase/flagship/ai_task_good.zs --require=bounded

# Unbounded task: no provable WCET, so the gate refuses to run it.
cargo run --release -- run showcase/flagship/ai_task_bad.zs --require=bounded
```

The first prints `[ZEUS POLICY GATE] certificate satisfies [bounded] -- executing.`
and runs. The second prints `[ZEUS POLICY GATE] refusing to run ... certificate
does NOT satisfy: bounded` and exits non-zero. That refusal is the whole point:
a promise you cannot keep does not ship.

## 5. Where to go next

- TUTORIAL.md walks through what a certificate is and how to gate on
  `constant-time`, step by step.
- ZEUS_EXPLAINED.md is the plain-language overview.

## Property names you can require

Pass them to `--require=` (comma-separated) or list them one per line in a
`zeus.policy` file in the working directory:

- `zero-heap` (alias `zero_heap`)
- `constant-time` (alias `constant_time`)
- `reproducible` (alias `deterministic`)
- `bounded` (alias `wcet`)

## Honest status

Zeus is a working prototype, not a downloadable product. It compiles and runs
real programs and the proof stack works today. It introduces no new
cryptography; it packages established techniques (taint analysis, WCET bounds,
proof-carrying code). The certificate is content-hashed (SHA-256) for
integrity, not yet cryptographically signed. Properties are proven
"machine-checked under a stated trusted base": the Zeus compiler, the C
compiler, and any solver are trusted and unverified, and source-level
constant-time can in principle be disturbed by aggressive backend
optimization. See ZEUS_MISSION.md for the full honesty ledger.
