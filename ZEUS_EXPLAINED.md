# Zeus, Explained Simply

**In one sentence:** Zeus is a tool that makes code *prove* it's safe and fast before it ships — and hands you a receipt that proves it.

## What it does (two modes)
- **Build (the Shield):** You write code and tag the important parts ("must finish in time," "must not leak secrets"). The compiler either **proves the promise and stamps a certificate**, or **refuses to build** and tells you why. You can't ship a promise you can't keep.
- **Audit (the Lens):** Point Zeus at code and it reports — and can **prove** — where there's a secret leak or a part that could run forever. (`zeus audit`, working today.)

## How it works (4 steps)
1. Write a program; tag a function: `@wcet(2000)` (time limit) or `@constant_time` (no secret leak).
2. Run `zeus build`. It proves the promise — or rejects the program.
3. You get a **certificate** listing exactly what was proven.
4. `zeus run --require ...` (or a `zeus.policy` file) refuses to run anything whose certificate doesn't match your rule.

## Who uses it
| User | Their pain | What Zeus gives |
|------|-----------|-----------------|
| Crypto / security engineers | "Does this leak the key through timing?" | Proof of no timing leak |
| Car / medical / aerospace / defense | "Does this always finish in time, with no surprise memory?" | Provable time/stack bounds = the audit evidence they already pay for |
| Blockchain teams | "Can this contract loop forever / drain fees?" | Proof of determinism + bounded execution |
| Teams running AI-written code | "Can I trust code an AI wrote?" | A gate that only runs code that proves it's safe |

## Why pick it over the alternatives
- **vs normal languages (Rust/Go/C++):** they let you *hope* it's safe. Zeus *refuses to build* until it's proven, and gives a receipt.
- **vs scanners (CodeQL/Semgrep):** they say "possible bug here." Zeus can say "**proved** no leak here" — it proves *absence*, not just suspicion.
- **vs research tools (Jasmin/HACL\*):** powerful but crypto-only and hard to use. Zeus aims to be usable, broad, and able to audit other people's code.

## Is it ready for consumers? (honest)
**Not yet — it's a working prototype, not a downloadable product.**
- Works today: compiles & runs real programs; the proof stack works; `zeus audit` works; 41 sample programs + 17 tests pass.
- Missing for a v1.0: one-command installer, docs + tutorial site, a real standard library, and **cryptographically signed** certificates (today they're content-hashed).

### Path to a consumer-ready v1.0
1. One-command install (`curl ... | sh`) + prebuilt binaries.
2. A tutorial + docs site (the simple version of this page, expanded).
3. Pick ONE audience and make that one demo bulletproof end-to-end.
4. Cryptographically sign the certificate (provenance, not just integrity).

**The honest pitch:** *with other tools you hope your code is safe; with Zeus the compiler won't build it until it proves the promises you made — and gives you a receipt that proves it.*
