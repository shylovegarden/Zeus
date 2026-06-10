#!/usr/bin/env python3
"""
fuzz_analyses.py -- METAMORPHIC / DIFFERENTIAL fuzzer for Zeus's analyses.

Generates many randomized programs whose GROUND-TRUTH property is known by
construction, runs `zeus audit --json`, and asserts the verdict matches. This is
the guard against the existential failure of a proof tool: a SILENT FALSE
PROVED-SAFE (a program that leaks but is reported constant-time/bounded).

Categories (each randomized over names/constants/shape):
  * secret reaches a TIMING SINK (branch / division / modulo, incl. compound /=,
    %=)  -> constant_time MUST be False
  * secret only in pure arithmetic (no branch/index/div)     -> constant_time True
  * `while` / recursion                                       -> wcet MUST be null
  * constant-bound `for`                                      -> wcet MUST be int
Plus a crash-fuzz pass: random malformed inputs MUST NOT crash (no 101/137/124).

Usage: ZEUS_BIN=/path/zeus_compiler python3 tests/fuzz_analyses.py [N]
Exit 0 iff every property holds.
"""
import json, os, random, subprocess, sys, tempfile

BIN = os.environ.get("ZEUS_BIN", "/tmp/zeus_target/release/zeus_compiler")
N = int(sys.argv[1]) if len(sys.argv) > 1 else 150
random.seed(1234)
fails = []
checks = 0

def audit(src):
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "f.zs")
        open(p, "w").write(src)
        r = subprocess.run([BIN, "audit", p, "--json"], capture_output=True, text=True, timeout=20)
        line = next((l for l in r.stdout.splitlines() if l.strip().startswith("{")), None)
        return json.loads(line) if line else None

def fn(rep, name, key):
    for f in rep.get("functions", []):
        if f["name"] == name:
            return f.get(key)
    return None

def rid(): return "".join(random.choice("abcdefghkmnpqrs") for _ in range(4))

def check(desc, src, name, key, want, pred):
    global checks
    checks += 1
    rep = audit(src)
    if rep is None:
        fails.append(f"{desc}: audit produced no JSON")
        return
    got = fn(rep, name, key)
    if not pred(got, want):
        fails.append(f"{desc}: {key}={got!r} (want {want!r})\n--- src ---\n{src}")

# ---- CT: secret reaches a timing sink -> constant_time MUST be False ----
def gen_leaky():
    a, b = rid(), rid()
    k = "k" + rid()
    forms = [
        f"if {k} > 0 {{ return {a}; }} return {b};",          # branch on secret
        f"return {a} / {k};",                                  # divide by secret
        f"return {a} % {k};",                                  # modulo by secret
        f"let mut x: i32 = {a}; x /= {k}; return x;",          # compound /= secret
        f"let mut x: i32 = {a}; x %= {k}; return x;",          # compound %= secret
    ]
    body = random.choice(forms)
    name = "leak_" + rid()
    return name, k, a, b, f"@constant_time\nfn {name}(secret {k}: i32, {a}: i32, {b}: i32) -> i32 {{ {body} }}\npub fn main() {{ println(0); }}\n"

# ---- CT clean: secret only in pure arithmetic -> constant_time True ----
def gen_clean():
    a, b = rid(), rid()
    k = "k" + rid()
    c = random.randint(1, 9)
    name = "clean_" + rid()
    return name, f"fn {name}(secret {k}: i32, {a}: i32, {b}: i32) -> i32 {{ return {a} + {k} * {c} - {b}; }}\npub fn main() {{ println(0); }}\n"

# ---- WCET unbounded: while / recursion -> wcet null ----
def gen_unbounded():
    n = rid(); name = "ub_" + rid()
    if random.random() < 0.5:
        src = f"fn {name}({n}: i32) -> i32 {{ let mut s: i32 = 0; while {n} > 0 {{ s = s + 1; }} return s; }}\npub fn main() {{ println(0); }}\n"
    else:
        src = f"fn {name}({n}: i32) -> i32 {{ if {n} > 0 {{ return {name}({n} - 1); }} return 0; }}\npub fn main() {{ println(0); }}\n"
    return name, src

# ---- WCET bounded: constant for -> wcet int ----
def gen_bounded():
    n = rid(); name = "bd_" + rid(); hi = random.randint(2, 40)
    src = f"fn {name}({n}: i32) -> i32 {{ let mut s: i32 = 0; for i in 0..{hi} {{ s = s + {n}; }} return s; }}\npub fn main() {{ println(0); }}\n"
    return name, src

print(f"== Zeus analysis fuzzer ==  binary: {BIN}  iters: {N}")
for _ in range(N):
    name, k, a, b, src = gen_leaky()
    check("leaky-secret-sink", src, name, "constant_time", False, lambda g, w: g is False)
    name, src = gen_clean()
    check("clean-arith", src, name, "constant_time", True, lambda g, w: g is True)
    name, src = gen_unbounded()
    check("unbounded-wcet", src, name, "wcet_steps", None, lambda g, w: g is None)
    name, src = gen_bounded()
    check("bounded-wcet", src, name, "wcet_steps", "int", lambda g, w: isinstance(g, int))

# ---- crash-fuzz: random garbage must never crash the compiler ----
toks = ["fn","let","mut","pub","return","if","while","for","{","}","(",")","[","]",
        "secret","struct",";",":","=","+","%","/","*","->","i32","main","x","0","42","@wcet","("]
crash = 0
for _ in range(N):
    src = " ".join(random.choice(toks) for _ in range(random.randint(3, 40)))
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "g.zs"); open(p, "w").write(src + "\n")
        try:
            r = subprocess.run([BIN, "build", p], capture_output=True, text=True, timeout=8)
            if r.returncode in (101, 132, 134, 136, 137, 139):  # panic / SIG*
                crash += 1; fails.append(f"crash-fuzz: rc={r.returncode} on: {src[:80]}")
        except subprocess.TimeoutExpired:
            crash += 1; fails.append(f"crash-fuzz: HANG on: {src[:80]}")
    checks += 1

print(f"\nchecks: {checks}   property failures: {len(fails)}   crashes/hangs: {crash}")
if fails:
    print("\nFAILURES (first 5):")
    for f in fails[:5]:
        print("  " + f.replace("\n", "\n    "))
    sys.exit(1)
print("RESULT: all properties held; no crashes. PASS")
