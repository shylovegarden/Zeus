# Zeus — Market, Competitive & Regulatory Assessment (mid-2026)

> Audit doc 06. Grounded, current, honest. Builds on `ZEUS_STRATEGY.md`, `ZEUS_MISSION.md`, and `ZEUS_WHITEPAPER.md` (§2–4) — it does not restate the thesis; it pressure-tests the *market* around it with 2026 facts. Every competitive and regulatory claim is sourced inline.

---

## Executive verdict

There is a real, defensible *wedge* here, but not yet a real *business* — and the distance between the two is the entire risk. The defensible idea is genuine: nobody today productizes a **sound proof of a non-functional safety envelope (constant-time / bounded WCET+stack / determinism / zero-heap) bound to a signed, gateable certificate, applied to foreign LLVM-IR.** The rigorous tools that can prove these properties (Jasmin, HACL\*, SPARK, CompCert+absint) are expert-only and language-bound; the ergonomic tools that everyone uses (CodeQL, Semgrep, Snyk) pattern-match and structurally *cannot say "proved absent"*; the sandboxes (E2B, Modal, Daytona, Wassette) contain code and prove nothing about it. That gap is real and the regulatory tailwind (FIPS-140-3, DO-178C/DO-333/DO-330, ISO 26262, IEC 62304, EU CRA) is real and dated. **But Zeus is a single-implementer, trusted-not-verified tool with no ecosystem, no third-party-confirmed soundness, and no design partner — the same three weaknesses that have kept every academic constant-time tool out of production for 15 years.** The single highest-leverage move is therefore *not* to chase the big AI-CI volume market; it is to **win one FIPS-140-3 / constant-time crypto design partner and get a CST-lab or crypto-engineering team to independently confirm one Zeus constant-time certificate against a real primitive.** That single act converts "interesting prototype" into "tool an auditor will accept," which is the only currency that matters in every downstream regulated market. Everything else follows from that proof point.

---

## 1. Competitive map (2026 facts)

The honest frame from `ZEUS_STRATEGY.md` holds: read the field by *what each category proves* and *what it structurally cannot*. Below, for each, what it does, maturity, who uses it, the gap Zeus fills — **and where it is ahead of Zeus** (this is the part that survives expert reading).

### Verified-code / formal-methods tools (the only category that touches code *properties*)

**SPARK / Ada (AdaCore).** A mature subset of Ada with contract-based deductive proof; AdaCore explicitly markets SPARK as letting "unit proof replace unit testing" under DO-178C and its DO-333 formal-methods supplement ([AdaCore, DO-178C technologies](https://www.adacore.com/books/do-178c-tech); [SPARK in practice](https://docs.adacore.com/spark2014-docs/html/ug/en/usage_scenarios.html)). Used in avionics, rail, defense for decades. **Gap Zeus fills:** SPARK proves *functional* contracts and absence of runtime errors; it does not give a first-class, signed *constant-time* or *WCET-bound* certificate on foreign IR, and it requires you to (re)write in Ada with heavy annotation. **Where SPARK is far ahead:** 20+ years of certification evidence, qualified tooling, a real customer base, an ecosystem, and — decisively — it is *trusted by certifiers today*. Zeus has none of that.

**Jasmin.** Low-level assembly-like language with a **Coq-verified compiler that provably preserves constant-time**, plus automated memory-safety and constant-time checkers ([Jasmin, ACM CCS'17](https://acmccs.github.io/papers/p1807-almeidaA.pdf)); 2024 work extends preservation to *speculative* constant-time (Spectre-v1) ([Preservation of Speculative Constant-Time, ePrint 2024/1203](https://eprint.iacr.org/2024/1203.pdf)). Used by crypto researchers (formosa-crypto, parts of libjade). **Gap Zeus fills:** Jasmin makes you hand-write in Jasmin assembly for one primitive at a time; Zeus aims to audit *clang-emitted IR from code you didn't write* and emit an auditor-facing certificate without rewriting. **Where Jasmin is far ahead:** its constant-time guarantee is *machine-checked down to assembly with a verified compiler* — a strictly stronger claim than Zeus's source/IR-level analysis on a trusted-unverified base (Zeus itself flags in `ZEUS_MISSION.md` that `-O2` can undo source-level constant-time). For the narrowest, highest-assurance crypto, Jasmin is simply more trustworthy than Zeus is today.

**HACL\* / F\*.** Verified C crypto library written in F\* and extracted to readable C; ships in Firefox (NSS), the Linux kernel, and elsewhere ([Jasmin/HACL\* overview](https://www.semanticscholar.org/paper/Jasmin:-High-Assurance-and-High-Speed-Cryptography-Almeida-Barbosa/18307d7fea0fed1067a5704f9aa13c93541e0142)). **Gap Zeus fills:** HACL\* is a *fixed library of primitives*, not a tool you point at arbitrary code; Zeus is a checker, not a library. **Where HACL\* is ahead:** it is *deployed in production at billions-of-users scale*, fully verified, and the reference answer for "I need a constant-time primitive." Zeus does not produce primitives; it can only attest someone else's.

**FaCT.** A domain-specific constant-time language (PLDI'19) that lets you write crypto in a C-like syntax and compiles to constant-time code. Research-grade, low adoption. **Gap Zeus fills:** same — bespoke language, crypto-only, not an auditor of foreign IR. **Where it's ahead:** purpose-built compiler guarantees vs. Zeus's analysis-after-the-fact.

**CompCert (AbsInt).** Formally (Coq-)verified optimizing C compiler. The 2026 landmark: CompCert was **qualified for the Multi-Function Computer New Generation (MFC_NG) of the ATR 42/72 aircraft in March 2026 — the first time DO-178C/DO-333/DO-330 certification credit has been claimed from compiler usage on critical avionics** ([Aerospace Innovations, 2026](https://aerospace-innovations.com/successful-qualification-of-compcert-for-the-multi-function-computer-new-generation-mfc_ng-of-atr-42-72-aircraft/); [AbsInt release 26-03-20](https://www.absint.com/releases/260320.htm)); Airbus has used CompCert in Toulouse for over a decade ([CompCert main page](https://compcert.org/)). **Gap Zeus fills:** CompCert proves *semantic preservation* (object code matches source), not the non-functional envelope (constant-time, WCET, leakage). They are complementary — CompCert could be Zeus's *trusted base* rather than a competitor. **Where it's ahead:** it is verified *and now formally qualified for airborne software*. This is the bar Zeus's "DO-178C evidence pack" ambition will be measured against, and Zeus is years from it.

**Frama-C.** Open-source C analysis platform (abstract interpretation + deductive verification via WP/Eva). Used in aerospace, nuclear, defense. **Gap Zeus fills:** Frama-C proves functional/runtime-error properties but has no first-class constant-time or signed-certificate story and demands heavy ACSL annotation. **Where it's ahead:** mature, multi-plugin, broadly trusted on safety-critical C.

**Dafny.** Microsoft's verification-aware language (SMT-backed) for functional correctness. Used internally and in teaching/research. **Gap Zeus fills:** functional correctness, not the non-functional envelope; requires writing in Dafny. **Where it's ahead:** mature toolchain, strong IDE, real industrial users.

**Kani (Rust, AWS).** Bit-precise bounded model checker for Rust; AWS uses it to verify parts of the Rust standard library and ships regular releases (v0.61, nightly-2025-04-03 toolchain) ([Kani GitHub](https://github.com/model-checking/kani); [AWS: Verify the Rust std library](https://aws.amazon.com/blogs/opensource/verify-the-safety-of-the-rust-standard-library/)). The Rust Foundation is *expanding* the formal-verification ecosystem (welcomed ESBMC in 2025) ([Rust Foundation](https://rustfoundation.org/media/expanding-the-rust-formal-verification-ecosystem-welcoming-esbmc/)). **Gap Zeus fills:** Kani checks panics/UB/assertions, *not* constant-time/WCET/leakage, and it is Rust-only and *bounded* (no unbounded soundness). **Where it's ahead:** big-sponsor (AWS) backing, growing ecosystem, real production use on the Rust std lib — exactly the ecosystem credibility Zeus lacks.

### Pattern scanners (ergonomic, ubiquitous, *not* proof)

**CodeQL & Semgrep.** Industry-standard SAST. Both are widely deployed but suffer the same structural ceiling: high false-positive rates and, critically, **they pattern-match for known-bad shapes — they find evidence of bugs, they never prove absence** ([Semgrep vs GitHub Advanced Security](https://semgrep.dev/resources/semgrep-vs-github/); [AI code security tool comparison, 2025](https://sanj.dev/post/ai-code-security-tools-comparison/)). Research confirms traditional scanners "weren't designed with AI-generated code patterns in mind, creating blind spots," and that up to ~40% of AI-generated code carries vulnerabilities ([IssueByte tool testing](https://issuebyte.com/after-testing-8-ai-code-security-tools/)). **Gap Zeus fills:** the "proved-absent, not pattern-found" claim from `ZEUS_MISSION.md` is the genuine differentiator — a Zeus green is a proof within its stated envelope. **Where they are ahead:** ubiquity, IDE/CI integration, ecosystem, breadth of bug classes, and *Semgrep has raised >$1B and CodeQL ships inside GitHub* ([Contrary Research: Semgrep](https://research.contrary.com/company/semgrep)). They cover 100% of repos at shallow depth; Zeus covers a narrow envelope at proof depth. Buyers default to breadth.

### AI-agent sandboxes (containment, not correctness)

As of 2026, four platforms own AI-agent code execution: **E2B** (Firecracker microVMs), **Modal** (only one that can hold a GPU in-sandbox), **Daytona** (sub-90ms cold starts; **raised a $24M Series A in February 2026**), and **Vercel Sandbox** ([Daytona vs E2B vs Modal vs Vercel, 2026](https://www.startuphub.ai/ai-news/artificial-intelligence/2026/daytona-vs-e2b-vs-modal-vs-vercel-sandbox-2026); [Northflank comparison](https://northflank.com/blog/daytona-vs-e2b-ai-code-execution-sandboxes)). **Microsoft Wassette** (announced Aug 6, 2025, on Wasmtime) gives AI tools a **deny-by-default** Wasm sandbox with no filesystem/network/env access unless granted ([Microsoft Open Source Blog](https://opensource.microsoft.com/blog/2025/08/06/introducing-wassette-webassembly-based-tools-for-ai-agents/); [The New Stack](https://thenewstack.io/wassette-microsofts-rust-powered-bridge-between-wasm-and-mcp/)). **Gap Zeus fills:** all of them *contain* untrusted code; **none can tell you the contained code is good** — a sandbox cheerfully runs a timing leak or an unbounded loop. Zeus is the trust gate *on top of* the box. **Where they are ahead:** they are funded, shipping, integrated, and solve a problem buyers *already feel acutely today*; Zeus solves a problem most buyers don't yet know they have. They are the GTM channel, not the rival.

### Supply-chain trust (provenance, not properties)

**SLSA / Sigstore / in-toto.** Now operational standards: Sigstore's cosign/Fulcio/Rekor are production and adopted across npm, PyPI, Kubernetes; GitHub ships native SLSA provenance via `actions/attest-build-provenance`; in-toto (ITE-6) is the common envelope ([InfoQ, 2025](https://www.infoq.com/news/2025/08/provenance/); [AquilaX](https://aquilax.ai/blog/supply-chain-artifact-signing-slsa)). Post-GhostAction (2025), "provenance verification is no longer optional," and supply-chain attacks **more than doubled in 2025** ([Practical DevSecOps, 2026](https://www.practical-devsecops.com/slsa-framework-guide-software-supply-chain-security/)). **Gap Zeus fills:** these prove *where an artifact came from*, not *whether the function leaks a secret through a branch*. Provenance ≠ properties. **Zeus is complementary** — it *emits* SLSA/in-toto provenance and carries the properties certificate as additional attested evidence. This is the right relationship: Zeus rides the rail, it doesn't fight it.

**Net competitive read:** Zeus's claimed white space is real and precisely stated. But in *every* cell, an incumbent is ahead on the axis buyers actually weigh first — verification depth (Jasmin/CompCert/SPARK), or breadth+ubiquity (CodeQL/Semgrep), or funded momentum (sandboxes). Zeus wins only where its *specific combination* (sound non-functional envelope + signed gateable cert + foreign-IR) is the buying criterion. That combination only becomes the buying criterion under regulatory pressure — which is §2.

---

## 2. Regulatory / standards hooks that create real buying pressure

The thesis only pays off where a standard *forces* someone to produce evidence Zeus already emits. Ranked by directness of fit.

**FIPS-140-3 (constant-time crypto) — strongest direct fit.** FIPS-140-3 is the mandatory bar for crypto modules sold to the US/Canada governments; the CMVP management manual is actively maintained (latest 2026) ([CMVP FIPS 140-3 Management Manual](https://csrc.nist.gov/projects/cryptographic-module-validation-program/cmvp-fips-140-3-management-manual)). Constant-time execution is treated as a **baseline requirement** for crypto implementations because timing side channels are remotely exploitable ([Towards Efficient Verification of Constant-Time, arXiv 2402.13506](https://arxiv.org/pdf/2402.13506.pdf); [SafeLogic, 2025 FIPS-140-3 review](https://www.safelogic.com/blog/2025-in-review-fips-140-3-post-quantum-readiness-crypto-agility)). The post-quantum migration (ML-KEM/ML-DSA) is *re-opening every validated module*, creating a wave of new constant-time evidence demand. **This maps directly onto what Zeus already proves** (constant-time + zero-heap + determinism), and the buyer (a crypto-module vendor or a CST lab) already pays for this analysis manually. Caveat: FIPS does not *name a tool*; Zeus's cert is only worth what an auditor will accept — see risk in §5.

**DO-178C + DO-333 (formal methods) / DO-330 (tool qualification) — strong fit, high bar.** DO-178C with the DO-333 formal-methods supplement explicitly lets formal proof *replace* certain testing; AdaCore markets exactly this ([AdaCore, Testing or Formal Verification](https://www.adacore.com/papers/testing-or-formal-verification-do-178c)). DO-330 governs *tool qualification* — and CompCert's March-2026 ATR qualification proves the door is open for a *tool* to earn certification credit ([AbsInt, 2026](https://www.absint.com/releases/260320.htm)). **Maps onto Zeus's bounded-WCET/stack and determinism proofs** — these are precisely the resource and behavioral evidence avionics demands. Caveat: to claim credit, Zeus itself would need DO-330 qualification (a multi-year, expensive effort). Near-term Zeus can be *supplementary evidence*, not a credit-bearing tool.

**ISO 26262 ASIL-D (automotive) — strong, concrete fit.** ISO 26262 §7.4.17 *explicitly requires* upper bounds on worst-case execution time and stack usage as architectural safety requirements ([AbsInt, safety standards](https://www.absint.com/qualification/safety.htm); [Rapita, ISO 26262](https://www.rapitasystems.com/iso26262)). AbsInt's aiT/StackAnalyzer are qualified up to ASIL-D for exactly this. **This is the single most literal match to a Zeus capability** (`@wcet`, `@stack`). Caveat: AbsInt/Rapita already own this with qualified tools and a measurement-based pedigree; Zeus's static bounds compete against an entrenched, trusted incumbent.

**IEC 62304 (medical) — moderate fit.** Class C (life-threatening) software demands the most comprehensive verification; tools like Axivion are TÜV-certified for use up to Class C ([Johner Institute](https://blog.johner-institute.com/iec-62304-medical-software/safety-class-iec-62304/); the draft IEC 62304:2026 reduces classes from three to two ([IntuitionLabs guide](https://intuitionlabs.ai/articles/iec-62304-medical-device-software-guide)). **Determinism and bounded-resource proofs map** onto Class C evidence, but the dominant 62304 concern is functional/risk-based, where Zeus is weaker. Secondary market.

**EU Cyber Resilience Act (CRA) — strong *timing* pressure, indirect *capability* fit.** The CRA is the clearest dated buying-pressure event: reporting obligations apply **11 September 2026**; full secure-by-design + machine-readable SBOM + conformity assessment obligations apply **11 December 2027**; fines up to **€15M or 2.5% of global turnover** ([EC Cyber Resilience Act](https://digital-strategy.ec.europa.eu/en/policies/cyber-resilience-act); [Mend.io CRA 2026 guide](https://www.mend.io/blog/eu-cyber-resilience-act-compliance-guide/); [Keysight one-year countdown](https://www.keysight.com/blogs/en/tech/nwvs/2025/09/11/one-year-countdown-to-eu-cra-compliance-september-11-2026-changes-everything)). The CRA's core demand is SBOM + vuln-handling + secure-by-design — which is *provenance/process*, closer to SLSA than to Zeus's properties. **Zeus rides this as evidence, not as a direct requirement.** Honest read: CRA creates budget and urgency for "attested software evidence," and Zeus's signed certificate fits that narrative, but Zeus does not satisfy a *named* CRA clause.

**EU AI Act — weaker/diffuse fit, and the deadline just slipped.** The Act applies broadly from 2 Aug 2026, but the **high-risk obligations were deferred by the Digital Omnibus (Nov 2025) to 2 December 2027**, confirmed by political agreement 7 May 2026 ([EC AI Act](https://digital-strategy.ec.europa.eu/en/policies/regulatory-framework-ai); [Latham & Watkins](https://www.lw.com/en/insights/ai-act-update-eu-resolves-to-change-rules-and-extend-deadlines)). The Act governs AI *systems/models*, not the *code AI generates* — so the fit to Zeus's "trust gate for AI-generated code" framing is thematic, not regulatory. Do not over-index on it.

**US executive orders / SBOM.** EO 14028 and successors drove SBOM into federal procurement and fed the SLSA/Sigstore wave ([Practical DevSecOps](https://www.practical-devsecops.com/slsa-framework-guide-software-supply-chain-security/)). Same posture as CRA: budget and narrative tailwind, provenance-shaped, Zeus rides it as supplementary evidence.

**Direct-fit ranking:** FIPS-140-3 (constant-time) > ISO 26262 (WCET/stack) > DO-178C/DO-333 (formal proof) > IEC 62304 > CRA/EO (timing pressure, provenance-shaped) > AI Act (diffuse, deferred). The capabilities Zeus *already* proves line up best with FIPS-140-3 and ISO 26262 — both crypto/embedded, both with an existing manual-spend Zeus can undercut.

---

## 3. The realistic beachhead — pick FIPS-140-3 / constant-time crypto

| Axis | **Beachhead A: FIPS-140-3 constant-time crypto tooling** | **Beachhead B: AI-code CI gating** |
|---|---|---|
| What's already built | Constant-time + zero-heap + determinism analysis + signed cert — **all REAL today** (`ZEUS_MISSION.md`) | Foreign-IR ingestion ("next keystone," not done); SARIF output (medium) |
| Speed-to-credibility | One CST-lab/crypto-team confirmation = instant credibility in a tiny, expert audience | Must beat CodeQL/Semgrep on breadth — slow, crowded, default-to-incumbent |
| Buyer | Crypto-module vendor / CST lab / a crypto-engineering team facing a FIPS validation or PQC migration | Platform/security/DevEx team — diffuse budget, "good enough" scanners already bought |
| Market size | Small but deep, well-funded, underserved; *expanding* due to PQC re-validation wave | Huge TAM but brutally contested (Semgrep >$1B, CodeQL in GitHub, Snyk 2.2M users) |
| Why now | PQC migration re-opens every validated module; constant-time evidence demand is rising | AI-code volume is rising, but containment (sandboxes) is the felt problem, not proof |
| Risk | Tiny audience; cert must be auditor-accepted | Competes on incumbents' axis (breadth); Zeus's depth is a hard sell here |

**Choose Beachhead A (FIPS-140-3 / constant-time crypto).** Justification:

1. **It uses only what already works.** Constant-time, zero-heap, determinism, and the signed `.zcert` are flagged REAL in `ZEUS_MISSION.md`. Beachhead B depends on the foreign-LLVM-IR front-end that `ZEUS_MISSION.md` itself calls "the next keystone, not done." You cannot beachhead on a capability you haven't shipped.
2. **Speed-to-credibility is the deciding factor for a single-implementer tool.** In a market where "an auditor will accept this" is the entire value, a *narrow expert audience* is an asset: one Jasmin/HACL\*-literate reviewer confirming one Zeus constant-time certificate against a real primitive (e.g., a Curve25519 or ML-KEM inner loop) is worth more than a thousand CI installs. The crypto community is small, vocal, and credentials travel fast.
3. **The competitor here is *un-usability*, not a funded rival.** Jasmin/HACL\*/FaCT *can* prove constant-time but are expert-only and primitive-specific; CodeQL/Semgrep *cannot prove it at all*. Zeus's wedge — "proof-of-constant-time as a CI check with an auditor-verifiable certificate" — has no ergonomic incumbent. In Beachhead B, every incumbent is funded and entrenched.
4. **It is the on-ramp to every later market.** A constant-time/FIPS credential is the credibility currency that opens ISO 26262 (WCET/stack), then DO-178C — the natural expansion path. Beachhead B does not build that currency.

TAM honesty: this is *not* a billion-dollar first market. It is a few hundred crypto-module vendors + CST labs + crypto teams globally — a design-partner and seed-stage beachhead, not a Series-B market. That is the *correct* first market for a single-implementer tool that must earn trust before scale.

---

## 4. Business model options (with comps)

**Recommended: open-core + cert/evidence packs for regulated buyers, sitting on top of sandboxes.** The three levers:

1. **Open-core engine.** The analysis + CLI open source to build the ecosystem and third-party scrutiny Zeus desperately needs (see §5 risks). *Comp:* **Semgrep** — OSS core, >$1B raised, monetizes enterprise features ([Contrary Research](https://research.contrary.com/company/semgrep)); **Snyk** — freemium dev tool, 2.2M users. Open-core is the proven path to *developer trust* and *adoption* for security tooling.

2. **Evidence/cert packs for regulated buyers (the revenue).** The money is in the *auditor-facing artifact*: signed certificate + provenance + a compliance-mapped evidence bundle (FIPS-140-3 constant-time evidence; ISO 26262 WCET/stack evidence; DO-178C/DO-333 supplementary evidence). *Comp:* **Chainguard** — sells *trust artifacts* (zero-CVE images + attestations), ARR grew 7× to ~$40M in FY2025, valued $3.5B, targeting $100M ARR ([Sacra](https://sacra.com/c/chainguard/); [FinTech Global](https://fintech.global/2025/04/24/chainguard-lands-356m-series-d-to-boost-global-software-supply-chain-security/)). Chainguard is the closest *business-model* comp: it monetizes "provable trustworthiness as a subscription," exactly Zeus's shape — except Zeus sells *properties* evidence where Chainguard sells *provenance/CVE* evidence. *Comp 2:* **AdaCore/AbsInt** — sell qualified tooling + Qualification Support Kits into DO-178C/ISO 26262 at high per-seat/per-project prices to a narrow, well-funded base. This is the *pricing* comp for the regulated packs.

3. **Sit on top of sandboxes (distribution, later).** Integrate as the trust-gate layer for E2B/Modal/Daytona/Wassette — Zeus emits the certificate the sandbox structurally can't. This is a *channel/partnership* play for the AI-CI expansion, not the first revenue line. *Comp:* SLSA/Sigstore's success came from being *embedded* in platforms (GitHub Actions) rather than sold standalone ([InfoQ](https://www.infoq.com/news/2025/08/provenance/)).

**Avoid:** a pure usage-metered SaaS (Zeus's value is the rare regulated certificate, not per-run volume) and a closed-source tool (kills the third-party-soundness scrutiny that is the only antidote to the single-implementer risk).

---

## 5. What must be true to be viable — and the biggest risks

**The "what must be true" list (in order):**

1. **A third party must independently confirm one Zeus certificate.** Until a CST lab or a Jasmin/HACL\*-literate reviewer confirms a Zeus constant-time proof against a real primitive, Zeus is "trust me," and no regulated buyer accepts "trust me." *This is the gating fact.*
2. **The trusted base must be stated, narrowed, and ideally verified.** `ZEUS_MISSION.md` honestly admits the Zeus compiler + Z3 + the C compiler are *trusted and unverified*, and that `-O2` can undo source-level constant-time. To be procurable, Zeus must either (a) validate at the IR/binary level (so optimization can't break the guarantee) or (b) sit *on top of CompCert* as its trusted base. A constant-time claim that an optimizer can silently void is not yet a sellable guarantee for FIPS.
3. **The foreign-LLVM-IR front-end must ship** for any AI-code story to exist at all (it is the "next keystone, not done").
4. **One paying/design-partner customer in the FIPS/crypto beachhead** — the proof that someone will pay for the certificate.
5. **An ecosystem signal** — open-sourcing the core, ≥1 external contributor, ≥1 external soundness review.

**The biggest risks (honest):**

- **Single implementer.** A verification tool whose soundness rests on one author's correctness is, to a certifier, an unqualified single point of failure. Every trusted competitor (CompCert, Jasmin, SPARK) earned trust through *years of external scrutiny and a verified or qualified base*. Zeus has neither — this is the central risk, and §5.1 is its only antidote.
- **Trusted-not-verified base.** Zeus proves properties on a foundation it trusts but hasn't verified, while its nearest credible neighbors (Jasmin's verified compiler, CompCert's verified semantics) verify *down to the machine*. Against them, Zeus's claim is structurally weaker — and buyers in this exact market are the ones who notice.
- **No ecosystem / no network effects.** SLSA, Sigstore, Kani, Semgrep all won via *adoption and sponsorship* (CNCF, AWS, GitHub). Zeus has no platform sponsor, no integrations beyond SARIF/SLSA emission, and no community. A trust tool with no one vouching for it is a contradiction.
- **The certifier-acceptance gap.** Even where capabilities map perfectly (FIPS constant-time, ISO 26262 WCET), *no standard names Zeus*, and incumbents (AbsInt, AdaCore) are already qualified. Zeus is supplementary evidence until it earns qualification — a long, costly road.
- **Beachhead-A market is small.** Winning crypto credibility is necessary but not sufficient for a venture-scale business; the path to scale (AI-CI, automotive) depends on capabilities not yet built and incumbents already entrenched.

---

## 12–24 month milestone path to a first paying / design-partner customer

**Months 0–3 — Harden the one real claim.** Lock down constant-time + zero-heap + determinism on the Zeus subset; finish Ed25519-signed `.zcert` (per `ZEUS_MISSION.md` move #2). Pick one real, recognizable primitive (e.g., an ML-KEM or Curve25519 inner loop) and produce a clean, reproducible constant-time certificate for it.

**Months 3–6 — Close the trusted-base hole enough to be defensible.** Add IR/binary-level constant-time validation *or* document Zeus-on-CompCert as the trusted base, so the certificate survives `-O2`. Publish a precise, honest "trusted base + threat model" statement. Open-source the core engine.

**Months 6–9 — Get one independent confirmation.** Hand the certificate + methodology to a Jasmin/HACL\*-literate reviewer or a CST lab and obtain a written, citable confirmation that the constant-time proof holds for that primitive. *This is the single highest-leverage milestone in the whole plan.*

**Months 9–15 — Productize the FIPS evidence pack + sign one design partner.** Wrap the certificate into a FIPS-140-3-mapped evidence bundle (constant-time + zero-heap + provenance). Target crypto-module vendors mid-PQC-migration and CST labs. Convert one into a *design partner* (paid or in-kind), pulled by the September 2026 CRA reporting deadline and the PQC re-validation wave as urgency.

**Months 15–24 — Ship the foreign-LLVM-IR front-end and prove it on real AI-generated crypto C.** Build the keystone front-end; demonstrate a constant-time certificate on `clang -emit-llvm` output from AI-generated crypto C — the bridge from Beachhead A (crypto) toward Beachhead B (AI-CI). Land the first *paying* contract on the FIPS evidence pack, and open the ISO 26262 WCET/stack conversation as the next expansion.

**Success at 24 months = one paying/design-partner crypto customer + one independently-confirmed constant-time certificate + an open-sourced, externally-reviewed core.** That trio, not revenue scale, is what converts Zeus from defensible idea into viable product.

---

### Sources

- [AdaCore — Technologies for DO-178C/ED-12C](https://www.adacore.com/books/do-178c-tech)
- [AdaCore — SPARK in Practice (unit proof for DO-178C)](https://docs.adacore.com/spark2014-docs/html/ug/en/usage_scenarios.html)
- [AdaCore — Testing or Formal Verification: DO-178C](https://www.adacore.com/papers/testing-or-formal-verification-do-178c)
- [Jasmin: High-Assurance and High-Speed Cryptography (ACM CCS'17)](https://acmccs.github.io/papers/p1807-almeidaA.pdf)
- [Preservation of Speculative Constant-Time by Compilation (ePrint 2024/1203)](https://eprint.iacr.org/2024/1203.pdf)
- [Jasmin / HACL\* overview (Semantic Scholar)](https://www.semanticscholar.org/paper/Jasmin:-High-Assurance-and-High-Speed-Cryptography-Almeida-Barbosa/18307d7fea0fed1067a5704f9aa13c93541e0142)
- [CompCert qualified for ATR 42/72 MFC_NG (Aerospace Innovations, 2026)](https://aerospace-innovations.com/successful-qualification-of-compcert-for-the-multi-function-computer-new-generation-mfc_ng-of-atr-42-72-aircraft/)
- [AbsInt — CompCert ATR qualification release (2026)](https://www.absint.com/releases/260320.htm)
- [CompCert main page](https://compcert.org/)
- [Kani Rust Verifier (GitHub)](https://github.com/model-checking/kani)
- [AWS — Verify the Safety of the Rust Standard Library](https://aws.amazon.com/blogs/opensource/verify-the-safety-of-the-rust-standard-library/)
- [Rust Foundation — Expanding the Rust Formal Verification Ecosystem (ESBMC)](https://rustfoundation.org/media/expanding-the-rust-formal-verification-ecosystem-welcoming-esbmc/)
- [Semgrep vs GitHub Advanced Security](https://semgrep.dev/resources/semgrep-vs-github/)
- [Best AI Code Security Tools 2025: Snyk vs Semgrep vs CodeQL](https://sanj.dev/post/ai-code-security-tools-comparison/)
- [After Testing 8 AI Code Security Tools (IssueByte)](https://issuebyte.com/after-testing-8-ai-code-security-tools/)
- [Contrary Research — Semgrep business breakdown](https://research.contrary.com/company/semgrep)
- [Daytona vs E2B vs Modal vs Vercel Sandbox (2026)](https://www.startuphub.ai/ai-news/artificial-intelligence/2026/daytona-vs-e2b-vs-modal-vs-vercel-sandbox-2026)
- [Daytona vs E2B (Northflank, 2026)](https://northflank.com/blog/daytona-vs-e2b-ai-code-execution-sandboxes)
- [Microsoft — Introducing Wassette](https://opensource.microsoft.com/blog/2025/08/06/introducing-wassette-webassembly-based-tools-for-ai-agents/)
- [The New Stack — Wassette](https://thenewstack.io/wassette-microsofts-rust-powered-bridge-between-wasm-and-mcp/)
- [InfoQ — Provenance Tools Becoming Standard (2025)](https://www.infoq.com/news/2025/08/provenance/)
- [AquilaX — Supply Chain Artifact Signing (SLSA/Sigstore)](https://aquilax.ai/blog/supply-chain-artifact-signing-slsa)
- [Practical DevSecOps — SLSA Framework Guide 2026](https://www.practical-devsecops.com/slsa-framework-guide-software-supply-chain-security/)
- [CMVP FIPS 140-3 Management Manual (NIST CSRC)](https://csrc.nist.gov/projects/cryptographic-module-validation-program/cmvp-fips-140-3-management-manual)
- [Towards Efficient Verification of Constant-Time (arXiv 2402.13506)](https://arxiv.org/pdf/2402.13506.pdf)
- [SafeLogic — 2025 in Review: FIPS 140-3, PQC, Crypto-Agility](https://www.safelogic.com/blog/2025-in-review-fips-140-3-post-quantum-readiness-crypto-agility)
- [AbsInt — Relation to Safety Standards (ISO 26262)](https://www.absint.com/qualification/safety.htm)
- [Rapita Systems — ISO 26262](https://www.rapitasystems.com/iso26262)
- [Johner Institute — IEC 62304 safety classes](https://blog.johner-institute.com/iec-62304-medical-software/safety-class-iec-62304/)
- [IntuitionLabs — IEC 62304 guide (incl. 2026 draft)](https://intuitionlabs.ai/articles/iec-62304-medical-device-software-guide)
- [European Commission — Cyber Resilience Act](https://digital-strategy.ec.europa.eu/en/policies/cyber-resilience-act)
- [Mend.io — EU CRA 2026 Compliance Guide](https://www.mend.io/blog/eu-cyber-resilience-act-compliance-guide/)
- [Keysight — One-Year Countdown to EU CRA (Sept 11, 2026)](https://www.keysight.com/blogs/en/tech/nwvs/2025/09/11/one-year-countdown-to-eu-cra-compliance-september-11-2026-changes-everything)
- [European Commission — AI Act](https://digital-strategy.ec.europa.eu/en/policies/regulatory-framework-ai)
- [Latham & Watkins — AI Act deadline deferral](https://www.lw.com/en/insights/ai-act-update-eu-resolves-to-change-rules-and-extend-deadlines)
- [Sacra — Chainguard revenue & growth](https://sacra.com/c/chainguard/)
- [FinTech Global — Chainguard $356M Series D](https://fintech.global/2025/04/24/chainguard-lands-356m-series-d-to-boost-global-software-supply-chain-security/)
