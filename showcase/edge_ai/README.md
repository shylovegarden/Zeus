# Verified Edge-AI Inference (the honest "micro-AI baked in")

`mlp_infer.zs` is a fixed-weight, quantized **2-layer perceptron** — 4 inputs ->
3 ReLU hidden neurons -> 1 linear output — written in Zeus. It is the realistic
form of "AI baked into the binary": **fixed weights + certified, bounded
inference**, not a self-modifying or self-learning binary (which would destroy
determinism, void the proofs, and add a poisoning attack surface). Retraining
happens offline; the new weights are recompiled and **re-certified**.

## Why this is different from PyTorch / ONNX at the edge
A normal inference stack has a heap, dynamic dispatch, and no timing guarantee —
unusable where you must *prove* worst-case latency (avionics, automotive ASIL-D,
medical, defense). Zeus compiles this network to zero-heap C and **proves**:

- **bounded WCET + stack** — every loop is constant-bounded, so the compiler
  emits a finite worst-case step count per function (`@wcet`).
- **reproducible** — no nondeterministic source touches the result (`@deterministic`):
  same inputs -> byte-identical output, on every node.
- **zero-heap** — no `malloc`; the arena/stack model is enforced at compile time.
- **constant-time** — no secret-dependent branch/index/division.

…and emits a **signed certificate** (`mlp_infer.zcert`, Ed25519) carrying those
facts, plus SLSA v1.0 in-toto provenance.

## Run it
```
zeus build showcase/edge_ai/mlp_infer.zs      # -> ./mlp_infer, prints 16
zeus cert  showcase/edge_ai/mlp_infer.zs       # human-readable PROVEN verdict
zeus run   showcase/edge_ai/mlp_infer.zs --require=zero-heap,reproducible,constant-time,bounded
```

## Verified output (this machine)
```
ZIR Analysis    [ 4 fns, 145 SSA values, 4/4 provably-deterministic ]
Resource Bounds [ 4/4 fns with provable WCET ]
Certificate verdict:
  zero-heap: PROVEN   reproducible: PROVEN   constant-time: PROVEN   fully-bounded: PROVEN
Per-function WCET (steps): relu=8  neuron=129  infer=441  main=451
infer(3,1,2,1) = 16        # h0=12, h1=1, h2=2 ; y = h0 + 2*h1 + h2 = 16
Policy gate: certificate satisfies [...] -- executing.
```

## Honest scope
WCET is a sound **abstract step count** (calibrate to nanoseconds per target CPU);
the stack figure is a conservative estimate. The certificate is content-hashed and
Ed25519-signed. The point isn't that this MLP is large — it's that the *entire
inference is certified bounded, deterministic, and heap-free*, which no mainstream
ML runtime can claim.
