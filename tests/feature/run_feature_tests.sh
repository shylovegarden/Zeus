#!/usr/bin/env bash
# =============================================================================
# Zeus FEATURE test suite -- hardens the new capabilities against the REAL
# compiler binary. Every check runs the actual `zeus_compiler` / `wasmtime` /
# agent-loop and asserts behavior that was observed empirically.
#
#   ZEUS_BIN=/path/to/zeus_compiler bash tests/feature/run_feature_tests.sh
#
# Optional env:
#   ZEUS_BIN  path to the zeus_compiler binary   (required)
#   WT        path to a wasmtime binary          (check 3 is SKIPPED if absent)
#   PY        python3 interpreter                 (default: python3)
#
# Prints PASS/FAIL per check, a final `RESULT: X passed, Y failed`, and exits
# non-zero if any check FAILs.
#
# Artifact hygiene: `zeus build/run/cert/wasm` and the agent loop emit files
# (.c/.h/.zcert/.wat/binary/.work.*) into the CURRENT working directory and,
# for `zeus wasm`, next to the SOURCE. This script therefore copies every
# source it must compile into a private scratch dir under /tmp and runs all
# emitting commands from there, so the repo is never littered.
# =============================================================================
set -u

# --- locate things ----------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FIXTURES="$SCRIPT_DIR/fixtures"
# repo root = three levels up from tests/feature
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SHOWCASE="$REPO_ROOT/showcase"

ZEUS_BIN="${ZEUS_BIN:-}"
WT="${WT:-}"
PY="${PY:-python3}"

if [ -z "$ZEUS_BIN" ] || [ ! -x "$ZEUS_BIN" ]; then
    echo "FATAL: ZEUS_BIN is not set or not executable (got '${ZEUS_BIN:-<unset>}')." >&2
    echo "       Run:  ZEUS_BIN=/path/to/zeus_compiler bash $0" >&2
    exit 2
fi
if ! command -v "$PY" >/dev/null 2>&1; then
    echo "FATAL: python3 (PY='$PY') not found; needed for JSON parsing." >&2
    exit 2
fi

# private scratch dir for all file-emitting commands
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/zeus_ftest.XXXXXX")"
cleanup() { rm -rf "$SCRATCH" 2>/dev/null || true; }
trap cleanup EXIT

PASS=0
FAIL=0
pass() { printf 'PASS  %s\n' "$1"; PASS=$((PASS+1)); }
fail() { printf 'FAIL  %s\n' "$1"; FAIL=$((FAIL+1)); }

# -----------------------------------------------------------------------------
# Check 1: audit v2 structured JSON -- wcet_exceeded
# -----------------------------------------------------------------------------
c1() {
    local desc="audit v2 JSON: under-budget @wcet -> wcet_exceeded finding (gap>0, fixable)"
    local pyf="$SCRATCH/c1.py"
    cat > "$pyf" <<'PYEOF'
import sys, json
line = next((l for l in sys.stdin if l.strip().startswith("{")), None)
assert line, "no JSON object on stdout"
d = json.loads(line)
assert d.get("audit") == "v2", "audit field != v2: %r" % d.get("audit")
fs = d.get("findings_structured", [])
hits = [f for f in fs if f.get("kind") == "wcet_exceeded"]
assert hits, "no wcet_exceeded finding in findings_structured"
f = hits[0]
assert isinstance(f.get("gap"), int) and f["gap"] > 0, "gap not a positive int: %r" % f.get("gap")
assert f.get("fixable") is True, "fixable != true: %r" % f.get("fixable")
PYEOF
    if "$ZEUS_BIN" audit "$FIXTURES/wcet_low.zs" --json 2>/dev/null | "$PY" "$pyf"; then
        pass "$desc"
    else
        fail "$desc"
    fi
}

# -----------------------------------------------------------------------------
# Check 2: audit v2 structured JSON -- secret_branch (not fixable)
# -----------------------------------------------------------------------------
c2() {
    local desc="audit v2 JSON: @constant_time fn branching on secret -> secret_branch (fixable=false)"
    local pyf="$SCRATCH/c2.py"
    cat > "$pyf" <<'PYEOF'
import sys, json
line = next((l for l in sys.stdin if l.strip().startswith("{")), None)
assert line, "no JSON object on stdout"
d = json.loads(line)
assert d.get("audit") == "v2", "audit field != v2"
fs = d.get("findings_structured", [])
hits = [f for f in fs if f.get("kind") == "secret_branch"]
assert hits, "no secret_branch finding in findings_structured"
assert hits[0].get("fixable") is False, "fixable != false: %r" % hits[0].get("fixable")
PYEOF
    if "$ZEUS_BIN" audit "$FIXTURES/secret_branch.zs" --json 2>/dev/null | "$PY" "$pyf"; then
        pass "$desc"
    else
        fail "$desc"
    fi
}

# -----------------------------------------------------------------------------
# Check 3: WASM round-trip -- neuron4(3,1,2,1) == 12 via wasmtime
#   (also asserts the native build+run of the same source prints 12)
# -----------------------------------------------------------------------------
c3() {
    local desc="WASM round-trip: zeus wasm math.zs -> wasmtime --invoke neuron4 3 1 2 1 == 12"
    if [ -z "$WT" ] || [ ! -x "$WT" ]; then
        printf 'SKIP  %s (WT not set/executable)\n' "$desc"
        return
    fi
    local d="$SCRATCH/wasm"; mkdir -p "$d"
    # copy source so the emitted .wat (written next to the source) lands in scratch
    cp "$SHOWCASE/wasm/math.zs" "$d/math.zs"
    ( cd "$d" && "$ZEUS_BIN" wasm "$d/math.zs" >/dev/null 2>&1 )
    local wat="$d/math.wat"
    if [ ! -f "$wat" ]; then fail "$desc (no .wat emitted)"; return; fi
    local got
    got="$("$WT" run --invoke neuron4 "$wat" 3 1 2 1 2>/dev/null | tr -d '[:space:]')"
    if [ "$got" != "12" ]; then fail "$desc (wasmtime got '$got', want 12)"; return; fi
    # cross-check: native build + run of the same source also prints 12
    local nat
    nat="$( cd "$d" && "$ZEUS_BIN" run "$d/math.zs" 2>/dev/null | tr -d '[:space:]' )"
    case "$nat" in
        *12*) pass "$desc" ;;
        *)    fail "$desc (native run got '$nat', want 12)" ;;
    esac
}

# -----------------------------------------------------------------------------
# Check 4: edge-AI -- build+run mlp_infer.zs prints 16
# -----------------------------------------------------------------------------
c4() {
    local desc="edge-AI: zeus build+run mlp_infer.zs prints 16"
    local d="$SCRATCH/edge"; mkdir -p "$d"
    cp "$SHOWCASE/edge_ai/mlp_infer.zs" "$d/mlp_infer.zs"
    local out
    out="$( cd "$d" && "$ZEUS_BIN" run "$d/mlp_infer.zs" 2>/dev/null | tr -d '[:space:]' )"
    case "$out" in
        *16*) pass "$desc" ;;
        *)    fail "$desc (got '$out', want 16)" ;;
    esac
}

# -----------------------------------------------------------------------------
# Check 5: edge-AI certificate -- reproducible + constant-time + bounded PROVEN
# -----------------------------------------------------------------------------
c5() {
    local desc="edge-AI cert: mlp_infer reproducible+constant-time+bounded all PROVEN"
    local d="$SCRATCH/cert"; mkdir -p "$d"
    cp "$SHOWCASE/edge_ai/mlp_infer.zs" "$d/mlp_infer.zs"
    local out
    out="$( cd "$d" && "$ZEUS_BIN" cert "$d/mlp_infer.zs" 2>&1 )"
    # Verdict line: reproducible: PROVEN  constant-time: PROVEN  fully-bounded: PROVEN
    local v
    v="$(printf '%s' "$out" | grep -i 'Verdict:' | head -1)"
    if printf '%s' "$v" | grep -qi 'reproducible:.*PROVEN' \
       && printf '%s' "$v" | grep -qi 'constant-time:.*PROVEN' \
       && printf '%s' "$v" | grep -qi 'bounded:.*PROVEN'; then
        pass "$desc"
    else
        fail "$desc (verdict line: $v)"
    fi
}

# -----------------------------------------------------------------------------
# Check 6: The Lens (LLVM) -- multi_fn has BOTH a PROVED-SAFE and a NOT-PROVEN
#   function and exits 1.
# -----------------------------------------------------------------------------
c6() {
    local desc="Lens multi_fn.ll: both PROVED-SAFE and NOT-PROVEN function, exit 1"
    local out rc
    out="$("$ZEUS_BIN" audit "$SHOWCASE/llvm/multi_fn.ll" 2>&1)"; rc=$?
    if [ "$rc" -ne 1 ]; then fail "$desc (exit $rc, want 1)"; return; fi
    # require a PROVED-SAFE function line and a NOT-PROVEN function line
    if printf '%s' "$out" | grep -q 'fn .*PROVED-SAFE' \
       && printf '%s' "$out" | grep -q 'fn .*NOT-PROVEN'; then
        pass "$desc"
    else
        fail "$desc (missing PROVED-SAFE and/or NOT-PROVEN function verdict)"
    fi
}

# -----------------------------------------------------------------------------
# Check 7: The Lens (LLVM) -- interproc.ll exits 1 and reports the cross-fn
#   taint flow "into @inner".
# -----------------------------------------------------------------------------
c7() {
    local desc="Lens interproc.ll: exit 1 and finding mentions 'into @inner'"
    local out rc
    out="$("$ZEUS_BIN" audit "$SHOWCASE/llvm/interproc.ll" 2>&1)"; rc=$?
    if [ "$rc" -ne 1 ]; then fail "$desc (exit $rc, want 1)"; return; fi
    if printf '%s' "$out" | grep -q 'into @inner'; then
        pass "$desc"
    else
        fail "$desc (no finding mentioning 'into @inner')"
    fi
}

# -----------------------------------------------------------------------------
# Check 8: The Lens (LLVM) -- public_add.ll is clean and exits 0.
# -----------------------------------------------------------------------------
c8() {
    local desc="Lens public_add.ll: PROVED-SAFE, exit 0"
    local out rc
    out="$("$ZEUS_BIN" audit "$SHOWCASE/llvm/public_add.ll" 2>&1)"; rc=$?
    if [ "$rc" -eq 0 ] && printf '%s' "$out" | grep -q 'PROVED-SAFE'; then
        pass "$desc"
    else
        fail "$desc (exit $rc; expected 0 + PROVED-SAFE)"
    fi
}

# -----------------------------------------------------------------------------
# Check 9: agent loop -- repair_demo.zs converges (exit 0).
# -----------------------------------------------------------------------------
c9() {
    local desc="agent loop: repair_demo.zs converges to a signed cert (exit 0)"
    local loop="$SHOWCASE/agent_loop/zeus_agent_loop.py"
    if [ ! -f "$loop" ]; then fail "$desc (agent loop script missing)"; return; fi
    local d="$SCRATCH/agent_repair"; mkdir -p "$d"
    cp "$SHOWCASE/agent_loop/repair_demo.zs" "$d/repair_demo.zs"
    local rc
    ( cd "$d" && ZEUS_BIN="$ZEUS_BIN" "$PY" "$loop" "$d/repair_demo.zs" >/dev/null 2>&1 )
    rc=$?
    if [ "$rc" -eq 0 ]; then pass "$desc"; else fail "$desc (exit $rc, want 0)"; fi
}

# -----------------------------------------------------------------------------
# Check 10: agent loop -- leak_demo.zs escalates (exit 2).
# -----------------------------------------------------------------------------
c10() {
    local desc="agent loop: leak_demo.zs (secret branch) escalates to human (exit 2)"
    local loop="$SHOWCASE/agent_loop/zeus_agent_loop.py"
    if [ ! -f "$loop" ]; then fail "$desc (agent loop script missing)"; return; fi
    local d="$SCRATCH/agent_leak"; mkdir -p "$d"
    cp "$SHOWCASE/agent_loop/leak_demo.zs" "$d/leak_demo.zs"
    local rc
    ( cd "$d" && ZEUS_BIN="$ZEUS_BIN" "$PY" "$loop" "$d/leak_demo.zs" >/dev/null 2>&1 )
    rc=$?
    if [ "$rc" -eq 2 ]; then pass "$desc"; else fail "$desc (exit $rc, want 2)"; fi
}

# --- run all -----------------------------------------------------------------
echo "== Zeus FEATURE test suite =="
echo "   ZEUS_BIN = $ZEUS_BIN"
echo "   WT       = ${WT:-<unset, check 3 will SKIP>}"
echo "   scratch  = $SCRATCH"
echo

c1; c2; c3; c4; c5; c6; c7; c8; c9; c10

echo
echo "RESULT: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
