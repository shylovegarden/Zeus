# Learn Zeus by Example

A curated set of small, copy-paste programs that each teach one Zeus idea.
Every example here has been **compiled and run with the real `zeus` binary**;
the "Expected output" lines below are the actual printed results.

## How to run an example

From this `examples/` directory:

```sh
zeus build ./ex01_hello.zs    # compile to a native binary (prints "Build Success")
zeus run   ./ex01_hello.zs    # compile AND execute, printing program output
zeus audit ./ex01_hello.zs    # static assurance report (constant-time / WCET / leaks)
```

`build` produces a native executable plus a signed certificate and provenance
file. `run` does the same and then runs it. `audit` is the CI gate: it reports
which functions are proven safe, constant-time, and bounded.

> Note on file names: these examples are named `exNN_*.zs` (letter-leading).
> The compiler derives a C header guard from the file's base name, and a name
> that *starts with a digit* (e.g. `01_hello.zs`) produces an invalid C macro
> and fails to compile. Keep a leading letter in your `.zs` file names.

---

## The examples

### ex01_hello.zs -- your first program
A function that returns a value plus a `main` that prints it. The smallest
useful Zeus program.
```sh
zeus run ./ex01_hello.zs
```
Expected output: `42`

### ex02_arithmetic.zs -- int types & operator precedence
Shows that integer locals need an explicit type (`let a: i32 = ...`) for
integer behavior, that `*` binds tighter than `+`, and that parentheses
override precedence. Computes `10 + 14 + 20 + 3`.
```sh
zeus run ./ex02_arithmetic.zs
```
Expected output: `47`

### ex03_functions.zs -- calling functions
Typed parameters and return values; one function calling another and the
results composing like ordinary values (no recursion). `twice(add(3,4))`.
```sh
zeus run ./ex03_functions.zs
```
Expected output: `14`

### ex04_wcet_bounds.zs -- proven worst-case execution time
`@wcet(5000)` declares a step budget. Because the `for` loop is
constant-bounded (`0..8`), the compiler PROVES the function fits the budget.
Adds `5` eight times.
```sh
zeus run   ./ex04_wcet_bounds.zs
zeus audit ./ex04_wcet_bounds.zs   # shows the function as bounded / PROVED-SAFE
```
Expected output: `40`

### ex05_secret_wipe.zs -- the `secret` keyword
`let secret key` marks data the compiler tracks through derived expressions
and zeroizes when dead. Here a secret key is folded into a public result.
```sh
zeus run ./ex05_secret_wipe.zs
```
Expected output: `1235`

### ex06_constant_time.zs -- a constant-time function that PASSES
`@constant_time` proves there is no secret-dependent timing. The function
does pure arithmetic and never branches on the secret, so the proof passes.
`99*3 + 7`.
```sh
zeus run   ./ex06_constant_time.zs
zeus audit ./ex06_constant_time.zs   # VERDICT: 2 constant-time | GATE PASSED
```
Expected output: `304`

### ex07_struct_soa.zs -- struct + Structure-of-Arrays
`Point[4]` allocates a fixed, heap-free buffer decomposed into aligned
per-field arrays. Access fields with `arr[i].field`. Sums four fields.
```sh
zeus run ./ex07_struct_soa.zs
```
Expected output: `37`

### ex08_tiny_net.zs -- a fixed-weight neuron (dot product)
One neuron with baked-in integer weights stored in a small SoA buffer. The
loop is constant-bounded, so `@wcet` (worst-case time) and `@deterministic`
(reproducible result) are both proven. Computes `2*3 + (-1)*1 + 3*2 + 1*1`.
Negative literals use the `0 - 1` form.
```sh
zeus run   ./ex08_tiny_net.zs
zeus audit ./ex08_tiny_net.zs   # bounded WCET, GATE PASSED
```
Expected output: `12`

---

## Verified results summary

| Example                | `zeus build` | `zeus run` output |
|------------------------|--------------|-------------------|
| ex01_hello.zs          | Build Success | 42  |
| ex02_arithmetic.zs     | Build Success | 47  |
| ex03_functions.zs      | Build Success | 14  |
| ex04_wcet_bounds.zs    | Build Success | 40  |
| ex05_secret_wipe.zs    | Build Success | 1235 |
| ex06_constant_time.zs  | Build Success | 304 |
| ex07_struct_soa.zs     | Build Success | 37  |
| ex08_tiny_net.zs       | Build Success | 12  |

All eight build cleanly and `zeus audit` reports `GATE PASSED` for the
annotated examples (04, 06, 08).
