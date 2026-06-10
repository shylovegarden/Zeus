# Zeus 5-Minute Tutorial: From "what is a certificate" to gating execution

This walks a newcomer through the core idea and ends with you producing a
real certificate for the crypto demo and refusing to run code that does not
prove what you demand. Everything is copy-pasteable.

Throughout, `zeus` is shorthand for `cargo run --release --` run from the
`zeus_compiler/` directory (see QUICKSTART.md). Paths below are relative to
the repo root.

## Step 1 -- What is a certificate?

When Zeus builds a program it does more than emit a binary. It runs its proof
passes (secret-taint / non-leakage, worst-case execution time, stack bounds,
zero-heap, determinism) and writes the results to a `.zcert` file next to the
binary. That file is the certificate: a record of exactly which safety
properties were proven, for which functions, plus a SHA-256 of the source so
the claim is tied to the exact code.

A certificate is a receipt. It does not say "looks fine." It says "this
function was proven constant-time / bounded / zero-heap," function by
function.

## Step 2 -- Build the crypto demo and look at its certificate

The flagship cryptography demo is a constant-time secret table lookup:

`showcase/flagship/crypto_sbox.zs`
```
@constant_time
fn sbox_mix(round: i32) -> i32 {
    let secret sbox = Cell[256];
    let a = sbox[round].v;
    return a + round;
}
```

The `@constant_time` attribute is a promise. Build it and read the
certificate as a human-readable trust report:

```
cargo run --release -- cert showcase/flagship/crypto_sbox.zs
```

You will see the raw certificate (a small JSON document) followed by a
Verdict line. The certificate's `sbox_mix` entry shows
`"constant_time":true` -- Zeus proved there is no secret-dependent timing
channel. If the function had branched on the secret, the build would have
failed with a CONSTANT-TIME VIOLATION instead.

The certificate file itself is written as `crypto_sbox.zcert` in your working
directory.

## Step 3 -- Gate execution on constant-time

A certificate is only useful if something enforces it. `zeus run` can refuse
to execute a program unless its certificate proves the properties you require.

Require constant-time for the crypto demo. Its certificate proves it, so it
runs:

```
cargo run --release -- run showcase/flagship/crypto_sbox.zs --require=constant-time
```

You should see:

```
[ZEUS POLICY GATE] certificate satisfies [constant-time] -- executing.
```

You can require several properties at once (comma-separated):

```
cargo run --release -- run showcase/flagship/crypto_sbox.zs --require=constant-time,zero-heap,reproducible,bounded
```

## Step 4 -- See the gate refuse unsafe code

Now require a property the code cannot prove. The AI-task "bad" demo has an
unbounded loop, so it has no provable worst-case execution time:

```
cargo run --release -- run showcase/flagship/ai_task_bad.zs --require=bounded
```

Zeus builds it but refuses to run it:

```
[ZEUS POLICY GATE] refusing to run 'ai_task_bad' -- certificate does NOT satisfy: bounded
```

That refusal (and the non-zero exit code) is the safety guarantee in action:
code that cannot keep the promise you demanded never executes.

## Step 5 -- Make the policy org-wide

Instead of repeating `--require=` on every run, drop a `zeus.policy` file in
your working directory. One property per line; blank lines and `#` comments
are ignored:

```
# org-wide proof policy
bounded
constant-time
zero-heap
```

With that file present, plain `zeus run <file.zs>` enforces every listed
property automatically -- the gate is on by default for everyone on the team.

## Recap

- A certificate (`.zcert`) records proven properties per function, tied to a
  source hash.
- `zeus cert <file>` renders it as a trust report.
- `zeus run <file> --require=<props>` (or a `zeus.policy` file) refuses to run
  anything whose certificate does not prove what you demand.

Property names: `zero-heap`, `constant-time`, `reproducible`, `bounded`
(aliases: `zero_heap`, `constant_time`, `deterministic`, `wcet`).

Honest note: properties are machine-checked under a stated trusted base (the
Zeus compiler and C compiler are trusted and unverified), and the certificate
is content-hashed for integrity, not yet cryptographically signed. See
ZEUS_MISSION.md.
