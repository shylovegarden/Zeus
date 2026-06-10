# Zeus -- Getting Started

**Prove your code is safe and fast before it ships -- and keep the receipt.**

Zeus is a high-assurance compiler for the people who cannot accept "it
probably works." You write `.zs` source and tag the parts that matter
(`@constant_time`, `@wcet`, `@stack`); Zeus compiles to readable C and a native
binary, and either proves the promise and stamps a machine-checkable
certificate (`.zcert`) -- or refuses to build. The same proof engine also runs
outward as an auditor (`zeus audit`) over Zeus source. A `zeus run --require`
flag or a `zeus.policy` file then refuses to execute any binary whose
certificate does not prove the properties you demand. With other tools you
hope your code is safe; with Zeus the compiler will not let it ship until the
promises are proven, and hands you a receipt that proves it.

## Start here

- QUICKSTART.md -- install prerequisites and reach your first certificate and
  policy gate in under five minutes.
- TUTORIAL.md -- a guided walk from "what is a certificate" to gating
  execution on `constant-time`.
- install.sh -- a POSIX script that checks for Rust and a C compiler, builds
  the release binary, and points you at the next step (Linux and macOS).
- ZEUS_EXPLAINED.md -- the plain-language overview of what Zeus does and why.

The flagship demos live in `showcase/flagship/` (one proof engine, four
audiences: crypto, smart contracts, safety-critical embedded, and AI
supply-chain). See `showcase/flagship/README.md`.

## Honest status

Zeus is a working prototype, not a downloadable product. It introduces no new
cryptography; it packages established techniques (taint analysis, WCET bounds,
proof-carrying code) into one usable toolchain. Properties are
machine-checked under a stated trusted base: the Zeus compiler, the C
compiler, and any solver are trusted and unverified, and source-level
constant-time can in principle be disturbed by aggressive backend
optimization. The certificate is content-hashed (SHA-256) for integrity, not
yet cryptographically signed for provenance. See ZEUS_MISSION.md for the full
honesty ledger.
