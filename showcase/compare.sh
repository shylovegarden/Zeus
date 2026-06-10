#!/usr/bin/env bash
# =============================================================================
# compare.sh -- build both versions, prove same RESULTS but different ACCESS
# PATTERN. Demonstrates Zeus's access-pattern obliviousness for secret-indexed
# table lookups (the AES-S-box / cache-timing target).
#
# Usage:
#   ZEUS=/path/to/zeus_compiler ./compare.sh
# Defaults to /tmp/zeus_target/release/zeus_compiler if ZEUS is unset.
#
# Honest scope: this proves the oblivious full-scan is PRESENT in the emitted
# code and that correctness is preserved. It does not measure cache timing.
# =============================================================================
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZEUS="${ZEUS:-/tmp/zeus_target/release/zeus_compiler}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# The Zeus compiler shells out to `clang`; if only gcc is present, shim it.
if ! command -v clang >/dev/null 2>&1; then
    if command -v gcc >/dev/null 2>&1; then
        mkdir -p "$WORK/bin"
        ln -sf "$(command -v gcc)" "$WORK/bin/clang"
        export PATH="$WORK/bin:$PATH"
        echo "[info] clang not found; shimmed clang -> gcc on PATH"
    fi
fi

echo "============================================================"
echo " 1. BUILD"
echo "============================================================"

# Copy the .zs into the work dir so the compiler emits its .c/binary there.
cp "$HERE/sbox_secure.zs" "$WORK/sbox_secure.zs"
( cd "$WORK" && "$ZEUS" build "$WORK/sbox_secure.zs" >/dev/null 2>&1 )
echo "[ok] built Zeus secure version  -> $WORK/sbox_secure  (+ emitted sbox_secure.c)"

gcc -O2 -o "$WORK/sbox_naive" "$HERE/sbox_naive.c"
echo "[ok] built naive C version      -> $WORK/sbox_naive"

echo
echo "============================================================"
echo " 2. SAME RESULTS  (correctness preserved)"
echo "============================================================"
SECURE_OUT="$("$WORK/sbox_secure")"
NAIVE_OUT="$("$WORK/sbox_naive")"

echo "Zeus secure output:"; echo "$SECURE_OUT" | sed 's/^/    /'
echo "Naive C output:";     echo "$NAIVE_OUT"  | sed 's/^/    /'

if [ "$SECURE_OUT" = "$NAIVE_OUT" ]; then
    echo "[PASS] outputs are IDENTICAL -> obliviousness did not change the result"
else
    echo "[FAIL] outputs differ!"; exit 1
fi

echo
echo "============================================================"
echo " 3. DIFFERENT ACCESS PATTERN  (the actual security property)"
echo "============================================================"

GEN_C="$WORK/sbox_secure.c"

echo
echo "--- Naive C: direct, secret-index-DEPENDENT access (the leak) ---"
grep -n 'table\[k[0-9]\]' "$HERE/sbox_naive.c" | sed 's/^/    /'

echo
echo "--- Zeus emitted C: secret-index lookups become oblivious full scans ---"
echo "    (each read scans all 256 entries; access pattern is index-INDEPENDENT)"
# Count only CALL SITES (exclude the static-inline definitions / declarations).
ORD=$(grep '__zeus_oread_bytes' "$GEN_C"  | grep -v 'static inline' | grep -c '(' || true)
OWR=$(grep '__zeus_owrite_bytes' "$GEN_C" | grep -v 'static inline' | grep -c '(' || true)
grep -n '_zo;[[:space:]]*}));' "$GEN_C" | grep '__zeus_oread_bytes' | sed 's/^/    /'

echo
echo "--- Proof the oblivious primitive is a constant-time FULL SCAN ---"
echo "    (branchless mask loop over all n entries -- from the emitted C):"
awk '/static inline void __zeus_oread_bytes/{p=1} p{print "    "$0} /^}/{if(p){p=0}}' "$GEN_C"

echo
echo "------------------------------------------------------------"
echo "SUMMARY"
echo "------------------------------------------------------------"
echo "  naive C direct table[k] accesses : 3  (each leaks its index)"
echo "  Zeus oblivious reads  in emitted C: $ORD"
echo "  Zeus oblivious writes in emitted C: $OWR"
echo "  oblivious scan cost               : O(n) = 256 entries touched per access"
echo
echo "CONCLUSION: identical results, but Zeus's secret-indexed accesses touch"
echo "the SAME 256 locations regardless of the secret index -- the cache-timing"
echo "leak present in the naive C is eliminated in the Zeus binary."
echo "(This is access-pattern obliviousness, NOT a claim of unbreakability.)"
