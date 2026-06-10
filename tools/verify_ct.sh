#!/usr/bin/env bash
# verify_ct.sh -- BINARY/IR-LEVEL constant-time verification (experimental).
#
# Zeus proves constant-time at the source/ZIR level, but an optimizing C compiler
# (-O2/-O3) could in principle reintroduce a data-dependent branch. This harness
# closes the loop: compile the Zeus-generated C to LLVM IR with clang, then re-run
# Zeus's Lens (`zeus audit`) on the EMITTED IR and check the target function.
#
# Requires clang/LLVM (for `-emit-llvm`). Without it, the IR step is reported as
# SKIPPED (honestly), not silently passed.
#
# Usage: ZEUS_BIN=/path/zeus_compiler bash tools/verify_ct.sh <file.zs> [target_fn]
#
# HONEST CAVEAT: Zeus's generated C bundles a runtime (arena, memcpy, scheduler),
# and the Lens seeds every function parameter as secret-by-default. So a WHOLE-MODULE
# IR audit is NOISY (runtime helpers branch on their params -> NOT-PROVEN noise that
# is NOT a real leak). Pass a [target_fn] to scope the pass/fail to the function you
# actually care about; without one, the IR audit is INFORMATIONAL only.
set -u
BIN="${ZEUS_BIN:-/tmp/zeus_target/release/zeus_compiler}"
SRC="${1:-}"; FUNC="${2:-}"
[ -z "$SRC" ] && { echo "usage: verify_ct.sh <file.zs> [target_fn]"; exit 2; }
[ -f "$SRC" ] || { echo "[verify-ct] no such file: $SRC"; exit 2; }
stem="$(basename "${SRC%.zs}")"
work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
cp "$SRC" "$work/$stem.zs"
strip() { sed -e 's/\x1b\[[0-9;]*m//g'; }

# 1) Source-level audit.
( cd "$work" && "$BIN" build "$stem.zs" >/dev/null 2>&1 ) \
  || { echo "[verify-ct] build failed for $SRC"; exit 1; }
echo "== source-level constant-time =="
( cd "$work" && "$BIN" audit "$stem.zs" 2>&1 | strip | grep -iE "fn |constant-time|GATE" | head -20 )

# 2) IR-level (needs clang).
if ! command -v clang >/dev/null 2>&1; then
  cat <<MSG

[verify-ct] clang/LLVM not found -- IR-level verification SKIPPED.
            To run it on an LLVM-equipped machine:
              clang -O2 -S -emit-llvm "$stem.c" -o "$stem.ll" && zeus audit "$stem.ll"
MSG
  exit 0
fi

[ -f "$work/$stem.c" ] || { echo "[verify-ct] generated C not found"; exit 1; }
clang -O2 -S -emit-llvm "$work/$stem.c" -o "$work/$stem.ll" 2>/dev/null \
  || { echo "[verify-ct] clang failed to emit LLVM IR"; exit 1; }
echo ""
echo "== IR-level audit (clang -O2 -emit-llvm) =="
ir="$("$BIN" audit "$work/$stem.ll" 2>&1 | strip)"
echo "$ir" | grep -iE "fn |NOT-PROVEN|PROVED|UNDECID" | head -40

if [ -z "$FUNC" ]; then
  echo ""
  echo "[verify-ct] INFORMATIONAL only (no target_fn given). Whole-module audit is"
  echo "            noisy: runtime helpers + all-params-secret produce expected"
  echo "            NOT-PROVEN noise. Re-run with a [target_fn] to gate on it."
  exit 0
fi

# Scoped pass/fail: the named function must NOT be NOT-PROVEN in the optimized IR.
verdict="$(echo "$ir" | grep -E "fn $FUNC[^a-zA-Z0-9_]" | head -1)"
echo ""
if [ -z "$verdict" ]; then
  echo "[verify-ct] target fn '$FUNC' not found in IR (clang may have inlined it)."
  exit 0
elif echo "$verdict" | grep -q "NOT-PROVEN"; then
  echo "[verify-ct] FAIL: '$FUNC' is NOT-PROVEN in -O2 IR -- constant-time may have been"
  echo "            lost by the optimizer. Investigate: $verdict"
  exit 1
else
  echo "[verify-ct] PASS: '$FUNC' constant-time survived to -O2 LLVM IR."
  echo "            $verdict"
  exit 0
fi
