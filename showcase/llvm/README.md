# The Lens on LLVM-IR (multi-block)

Zeus audits code it did NOT write by ingesting textual LLVM IR
(`clang -O1 -S -emit-llvm foo.c -o foo.ll`, then `zeus audit foo.ll`).
Add `--sarif` to emit SARIF 2.1.0 for CI; `--strict` to also fail on UNDECIDABLE.

How it works: the function is parsed into basic blocks; secret-taint is computed
with a MONOTONE FIXPOINT over all instructions, so it is sound through `phi` nodes
and across loop back-edges. A small alloca-memory model tracks secrets through
`store`/`load` on direct stack slots; a secret written to a non-alloca pointer, a
loop, or an unknown opcode degrades to UNDECIDABLE -- it never reports PROVED-SAFE
on code it could not fully reason about.

Taint seed: parameters are secret by default; `; zeus.public: %a %b` marks params public.

Demos (`zeus audit showcase/llvm/<file>.ll`):
- sbox.ll         -> NOT-PROVEN  (secret-indexed table lookup: the AES S-box cache-timing leak)
- insecure_cmp.ll -> NOT-PROVEN  (multi-block loop w/ phi; secret bytes reach a branch: non-constant-time compare)
- route_pub.ll    -> PROVED-SAFE (multi-block branch + phi join; public inputs, no timing channel)
- counter.ll      -> UNDECIDABLE (a loop back-edge cannot be bounded; --strict fails it)

The first two are exactly the shape `clang -O1` emits for real C -- this is the
"audit AI-generated C in CI" path: findings -> SARIF -> a build that fails on a leak.
