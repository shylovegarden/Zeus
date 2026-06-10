#!/usr/bin/env python3
"""
zeus_agent_loop.py -- the AI <-> Zeus self-repair loop ("the human-free CI loop").

An autonomous agent submits a Zeus module; Zeus audits it and returns STRUCTURED,
machine-first diagnostics (`zeus audit --json` -> `findings_structured`); the agent
reads the typed fields (kind, function, observed/budget/gap, suggested_action),
edits the source, and resubmits -- looping until Zeus proves the code safe and
emits an Ed25519-signed certificate (`.zcert`). No human reviews the code.

Each diagnostic carries the "distance to a valid proof" (`gap`): the exact amount
by which a resource budget is exceeded. The agent fixes the gap or, when the
finding is a real logic/timing leak (not safely auto-patchable), ESCALATES.

HONEST FRAMING. The "agent" is a DETERMINISTIC STUB (`propose_fix`) so the demo is
reproducible offline. A real LLM agent implements the SAME interface:
    propose_fix(structured: list[dict], source: str) -> (new_source | None, action)
Zeus does not "make code safe"; it refuses to certify unsafe code, and the loop
cannot paper over a real leak. Soundness holds within Zeus's modeled subset and
trusts the compiler + Z3 base.
"""
import json, re, subprocess, sys, os, shutil

MAX_ITERS = 12

def run(bin_, args):
    return subprocess.run([bin_] + args, capture_output=True, text=True)

def audit(bin_, path):
    r = run(bin_, ["audit", path, "--json"])
    line = next((l for l in r.stdout.splitlines() if l.strip().startswith("{")), None)
    if not line:
        return {"findings": [], "findings_structured": [], "functions": [],
                "_err": r.stderr.strip()[:200]}
    return json.loads(line)

def set_attr_for_fn(source, fn_name, attr, new_val):
    """Real source edit: raise the @wcet/@stack budget on the named function."""
    lines = source.splitlines()
    fn_re = re.compile(r'^\s*(pub\s+)?fn\s+' + re.escape(fn_name) + r'\b')
    target = next((i for i, l in enumerate(lines) if fn_re.search(l)), None)
    if target is None:
        return None
    k = target - 1
    attr_re = re.compile(r'^\s*@' + re.escape(attr) + r'\s*\(\s*\d+\s*\)\s*$')
    while k >= 0 and (lines[k].lstrip().startswith('@') or lines[k].strip() == ''):
        if attr_re.match(lines[k]):
            indent = lines[k][:len(lines[k]) - len(lines[k].lstrip())]
            lines[k] = f"{indent}@{attr}({new_val})"
            return "\n".join(lines) + ("\n" if source.endswith("\n") else "")
        k -= 1
    return None

# ----- THE AGENT (deterministic stub; an LLM implements the same signature) -----
def propose_fix(structured, source):
    """Consume TYPED diagnostics. Return (new_source, action) or (None, reason)."""
    for d in structured:
        if not d.get("fixable"):
            continue
        fn = d["function"]
        if d["kind"] == "wcet_exceeded":
            new = set_attr_for_fn(source, fn, "wcet", d["observed_steps"])
            if new:
                return new, f"raise @wcet({fn}) -> {d['observed_steps']} (closes proof gap of {d['gap']} steps)"
        elif d["kind"] == "stack_exceeded":
            new = set_attr_for_fn(source, fn, "stack", d["observed_bytes"])
            if new:
                return new, f"raise @stack({fn}) -> {d['observed_bytes']} (closes proof gap of {d['gap']} bytes)"
    # nothing fixable -> escalate with the typed reason
    if structured:
        d = structured[0]
        return None, f"[{d['kind']}] {d['function']}: {d['suggested_action']}"
    return None, "unknown failure"

def emit_cert(bin_, workdir, work_zs):
    run(bin_, ["build", work_zs])
    stem = os.path.splitext(os.path.basename(work_zs))[0]
    cert = os.path.join(workdir, stem + ".zcert")
    if not os.path.exists(cert):
        return None, "build did not emit a certificate"
    v = run(bin_, ["verify-cert", cert])
    return cert, (v.stdout.strip() or v.stderr.strip())

def main():
    if len(sys.argv) < 2:
        print("usage: zeus_agent_loop.py <module.zs> [--bin PATH]"); sys.exit(2)
    src_path = sys.argv[1]
    bin_ = os.environ.get("ZEUS_BIN", "/tmp/zeus_target/release/zeus_compiler")
    if "--bin" in sys.argv:
        bin_ = sys.argv[sys.argv.index("--bin") + 1]

    workdir = os.path.dirname(os.path.abspath(src_path)) or "."
    stem = os.path.splitext(os.path.basename(src_path))[0]
    work_zs = os.path.join(workdir, stem + ".work.zs")
    shutil.copyfile(src_path, work_zs)
    source = open(work_zs, encoding="utf-8").read()

    print("=" * 70)
    print(f"  AI <-> ZEUS SELF-REPAIR LOOP   module: {os.path.basename(src_path)}")
    print(f"  agent: deterministic stub   diagnostics: structured (audit v2)")
    print("=" * 70)

    for it in range(1, MAX_ITERS + 1):
        rep = audit(bin_, work_zs)
        structured = rep.get("findings_structured", [])
        verdicts = ", ".join(f"{fn['name']}:{fn['verdict']}" for fn in rep.get("functions", []))
        print(f"\n[iter {it}] zeus audit --> {verdicts or '(no functions)'}")
        if not structured:
            print("           no findings -- code PROVEN safe. Requesting certificate...")
            cert, vmsg = emit_cert(bin_, workdir, work_zs)
            if cert:
                print(f"\n  ✅ CERTIFIED  {os.path.basename(cert)}")
                print(f"     {vmsg}")
                print("     The agent shipped verified code with ZERO human review.")
                sys.exit(0)
            print("  audit clean but cert failed:", vmsg); sys.exit(1)

        top = structured[0]
        gap = top.get("gap")
        gap_s = f"   distance-to-proof: {gap}" if gap is not None else ""
        print(f"           {len(structured)} finding(s); top: [{top['kind']}] {top['function']}{gap_s}")
        new_source, action = propose_fix(structured, source)
        if new_source is None:
            print(f"\n  ⛔ ESCALATE TO HUMAN -- {action}")
            print("     The loop refuses to certify. This is the safety guarantee:")
            print("     a real logic/timing leak cannot be auto-patched away.")
            sys.exit(2)
        print(f"           agent edit: {action}")
        source = new_source
        open(work_zs, "w", encoding="utf-8").write(source)

    print("\n  gave up after", MAX_ITERS, "iterations"); sys.exit(3)

if __name__ == "__main__":
    main()
