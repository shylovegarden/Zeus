# Fuzz Testing Setup

## Overview

Fuzz testing has been set up for the Zeus lexer and parser using `cargo-fuzz` to detect security vulnerabilities and crashes from malformed input.

## Setup

### Installation

```bash
cargo install cargo-fuzz
```

### Initialization

```bash
cd zeus_compiler
cargo fuzz init
```

### Fuzz Target

The fuzz target is located at `zeus_compiler/fuzz/fuzz_targets/fuzz_target_1.rs`.

It tests:
- Lexer tokenization with arbitrary input
- Parser parsing with arbitrary input
- Security limits (input size, recursion depth, AST node count)
- Panic prevention on malformed input

## Running Fuzz Tests

### Prerequisites

Fuzz testing requires the **nightly** Rust compiler:

```bash
rustup install nightly
rustup default nightly
```

### Running the Fuzzer

```bash
cd zeus_compiler
cargo fuzz run fuzz_target_1
```

### Running with Specific Corpus

```bash
cargo fuzz run fuzz_target_1 fuzz/corpus/fuzz_target_1
```

### Running for Limited Time

```bash
cargo fuzz run fuzz_target_1 -- -max_total_time=60
```

## Security Limits Tested

The fuzzer validates that the following security limits prevent DoS attacks:

- **MAX_INPUT_SIZE**: 10MB (lexer panic on exceed)
- **MAX_LINE_LENGTH**: 10,000 characters (lexer error on exceed)
- **MAX_RECURSION_DEPTH**: 1000 (parser error on exceed)
- **MAX_AST_NODES**: 100,000 (parser error on exceed)

## Expected Behavior

- **Valid UTF-8**: Input is tokenized and parsed
- **Invalid UTF-8**: Input is skipped (ignored)
- **Large input**: Should hit security limits and error gracefully
- **Malformed input**: Should error without panicking
- **Deeply nested input**: Should hit recursion limit and error gracefully

## Integration with CI/CD

To add fuzz testing to CI/CD:

```yaml
- name: Install nightly Rust
  run: rustup install nightly && rustup default nightly

- name: Run fuzz tests
  run: cd zeus_compiler && cargo fuzz run fuzz_target_1 -- -max_total_time=60
```

## Notes

- Fuzz testing is resource-intensive (CPU and memory)
- Recommended to run nightly or weekly, not on every commit
- Consider using OSS-Fuzz for continuous fuzzing
- Corpus can be seeded with interesting test cases

## References

- [cargo-fuzz documentation](https://github.com/rust-fuzz/cargo-fuzz)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
- [OSS-Fuzz](https://github.com/google/oss-fuzz)
