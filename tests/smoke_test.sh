#!/usr/bin/env bash
# Zeus end-to-end smoke test: exercises every CLI command + language/security
# feature and reports PASS/FAIL. Builds run in a private scratch dir so the repo
# is never littered. Usage: ZEUS_BIN=/path/to/zeus_compiler bash tests/smoke_test.sh
set -u
BIN="${ZEUS_BIN:-/tmp/zeus_target/release/zeus_compiler}"
WT="${WT:-}"
REPO="$(cd "$(dirname "$0")/.." && pwd)"
SCRATCH="$(mktemp -d)"; trap 'rm -rf "$SCRATCH"' EXIT
PASS=0; FAIL=0; FAILED=()
strip() { sed -e 's/\x1b\[[0-9;]*m//g'; }
ok()  { PASS=$((PASS+1)); printf '  \033[32mPASS\033[0m %s\n' "$1"; }
no()  { FAIL=$((FAIL+1)); FAILED+=("$1"); printf '  \033[31mFAIL\033[0m %s  -- %s\n' "$1" "${2:-}"; }

# Cross-platform timeout wrapper (macOS doesn't have timeout by default)
run_with_timeout() {
    local secs=$1; shift
    if command -v timeout >/dev/null 2>&1; then
        timeout "$secs" "$@"
    elif command -v gtimeout >/dev/null 2>&1; then
        gtimeout "$secs" "$@"
    else
        # No timeout available, run directly (risky but better than failing)
        "$@"
    fi
}

# write source $2 to scratch/$1.zs, echo the path
src() { printf '%b\n' "$2" > "$SCRATCH/$1.zs"; echo "$SCRATCH/$1.zs"; }

# build should succeed (exit 0)
build_ok() { local f; f=$(src "$1" "$2"); ( cd "$SCRATCH" && "$BIN" build "$f" >/dev/null 2>&1 ); [ $? -eq 0 ] && ok "$1" || no "$1" "build failed"; }
# build should FAIL (exit !=0) and stderr/stdout contain $3
build_fail() { local f out rc; f=$(src "$1" "$2"); out=$( cd "$SCRATCH" && "$BIN" build "$f" 2>&1 ); rc=$?; out=$(printf '%s' "$out" | strip); if [ $rc -eq 0 ]; then no "$1" "expected rejection, built OK"; elif [ -n "${3:-}" ] && ! echo "$out" | grep -qiF "$3"; then no "$1" "rejected but msg missing '$3'"; else ok "$1"; fi; }
# build + run, stdout must equal $3
run_eq() { local f out; f=$(src "$1" "$2"); ( cd "$SCRATCH" && "$BIN" build "$f" >/dev/null 2>&1 ) || { no "$1" "build failed"; return; }; out=$( cd "$SCRATCH" && "./$1" 2>/dev/null | tr -d '\n' ); [ "$out" = "$3" ] && ok "$1 (=$3)" || no "$1" "got '$out' want '$3'"; }
# audit .ll: verdict + exit code
audit_ll() { local f; f="$SCRATCH/$1.ll"; printf '%b\n' "$2" > "$f"; local out rc; out=$("$BIN" audit "$f" 2>&1 | strip); rc=$( "$BIN" audit "$f" >/dev/null 2>&1; echo $? ); if echo "$out" | grep -q "$3" && [ "$rc" = "$4" ]; then ok "$1 ($3 rc=$4)"; else no "$1" "verdict/rc mismatch (rc=$rc)"; fi; }

echo "================ ZEUS SMOKE TEST ================"
echo "binary: $BIN"
echo ""
echo "## Language core"
run_eq  hello        'pub fn main() { println(42); }' '42'
run_eq  arith_prec   'fn f()->i32{ return 2*3+4; } pub fn main(){ println(f()); }' '10'
run_eq  modulo       'fn f(a:i32,b:i32)->i32{ return a % b; } pub fn main(){ println(f(17,5)); }' '2'
run_eq  mod_prec     'fn f()->i32{ return 2 + 7 % 3; } pub fn main(){ println(f()); }' '3'
run_eq  ifelse       'fn s(n:i32)->i32{ if n>0 {return 1;} else if n<0 {return 2;} return 0; } pub fn main(){ println(s(0-4)); }' '2'
run_eq  whileloop    'pub fn main(){ let mut i:i32=0; while i<5 { i = i + 1; } println(i); }' '5'
run_eq  forloop      'pub fn main(){ let mut s:i32=0; for i in 0..10 { s = s + i; } println(s); }' '45'
run_eq  logic        'pub fn main(){ let a:i32=5; if a>0 && a<10 { println(1); } }' '1'
run_eq  neg_lit      'fn f()->i32{ return 0-7; } pub fn main(){ println(f()); }' '-7'
run_eq  compound     'pub fn main(){ let mut s:i32=3; s += 4; println(s); }' '7'
build_ok struct_soa  'struct P{x:i32,y:i32} pub fn main(){ let a=P[4]; a[0].x=9; a[0].y=1; println(a[0].x); }'
build_ok strings     'pub fn main(){ let s:str="hello"; println(0); }'
build_ok recursionfree 'fn g(x:i32)->i32{return x;} fn f(x:i32)->i32{return g(g(x));} pub fn main(){ println(f(5)); }'

echo ""
echo "## Type checker + diagnostics (must REJECT cleanly)"
build_fail type_mismatch   'fn add(a:i32,b:i32)->i32{return a+b;} pub fn main(){ let s:str=add(1,2); println(0); }' 'type mismatch'
build_fail ret_mismatch    'fn f()->i32{ return "x"; } pub fn main(){ println(0); }' 'return type mismatch'
build_fail wrong_arity     'fn add(a:i32,b:i32)->i32{return a+b;} pub fn main(){ println(add(1,2,3)); }' 'argument'
build_fail unknown_attr    '@wceet(5)\nfn f(n:i32)->i32{return n;}\npub fn main(){println(0);}' 'unknown'
build_fail int_overflow    'fn f()->i32{ let x:i32=99999999999999999999999999; return x; } pub fn main(){println(0);}' 'range'

echo ""
echo "## Robustness (must not crash; clean handling)"
build_fail bareblock   'pub fn main() {{}}' ''
build_fail typename_fn 'fn f32(n:i32)->i32{return n;}\npub fn main(){println(0);}' ''
# missing file
"$BIN" build "$SCRATCH/nope_xyz.zs" >/dev/null 2>&1; mrc=$?; if [ "$mrc" = "101" ]; then no "missing_file_clean" "panicked (101)"; elif [ "$mrc" != "0" ]; then ok "missing_file_clean (rc=$mrc)"; else no "missing_file_clean" "built a missing file?!"; fi

# variable divide-by-zero must trap CLEANLY (rc 136), not raw SIGFPE
DZ=$(src divzero 'fn f(a:i32,b:i32)->i32{ return a / b; } pub fn main(){ println(f(10,0)); }')
( cd "$SCRATCH" && "$BIN" build divzero.zs >/dev/null 2>&1 )
( cd "$SCRATCH" && run_with_timeout 3 ./divzero >/dev/null 2>&1 ); dzrc=$?
[ "$dzrc" = "136" ] && ok "divzero_clean_trap (rc=136)" || no "divzero_clean_trap" "rc=$dzrc (want clean trap not SIGFPE)"

echo ""
echo "## Security / proof"
build_ok  ct_safe     '@constant_time\nfn f(secret k:i32, a:i32)->i32{ return a + 1; }\npub fn main(){ println(0); }'
build_fail ct_leak    '@constant_time\nfn f(secret k:i32, a:i32, b:i32)->i32{ if k>0 {return b;} return a; }\npub fn main(){println(0);}' ''
build_fail wcet_exceed '@wcet(5)\nfn f(n:i32)->i32{ let mut s:i32=0; for i in 0..1000 { s=s+n; } return s; }\npub fn main(){println(0);}' ''
build_ok  wcet_ok     '@wcet(20000)\nfn f(n:i32)->i32{ let mut s:i32=0; for i in 0..10 { s=s+n; } return s; }\npub fn main(){println(0);}'
build_fail launder    'struct B{v:i32}\n@constant_time\nfn f(secret k:i32,a:i32,b:i32)->i32{ let x=B{v:k}; let c:i32=x.v; if c>0 {return b;} return a; }\npub fn main(){println(0);}' ''

echo ""
echo "## The Lens (audit non-Zeus LLVM IR)"
audit_ll  lens_safe   '; zeus.public: %x\ndefine i32 @g(i32 %x) {\nentry:\n  %r = add i32 %x, 1\n  ret i32 %r\n}' 'PROVED-SAFE' 0
audit_ll  lens_branch 'define i32 @f(i32 %s) {\nentry:\n  %c = icmp sgt i32 %s, 0\n  br i1 %c, label %a, label %b\na:\n  ret i32 1\nb:\n  ret i32 0\n}' 'NOT-PROVEN' 1
audit_ll  lens_shift  'define i32 @h(i32 %s) {\nentry:\n  %r = shl i32 1, %s\n  ret i32 %r\n}' 'NOT-PROVEN' 1
audit_ll  lens_loop   'define void @l(i32 %n) {\nentry:\n  br label %lp\nlp:\n  br label %lp\n}' 'UNDECIDABLE' 0

echo ""
echo "## Certificate + policy gate"
CF=$(src certgate '@wcet(9000)\nfn add(a:i32,b:i32)->i32{ return a+b; }\npub fn main(){ println(add(2,3)); }')
( cd "$SCRATCH" && "$BIN" build certgate.zs >/dev/null 2>&1 )
[ -f "$SCRATCH/certgate.zcert" ] && ok "cert_emitted" || no "cert_emitted" "no .zcert"
( cd "$SCRATCH" && "$BIN" verify-cert certgate.zcert >/dev/null 2>&1 ) && ok "verify_cert_valid" || no "verify_cert_valid" ""
cp "$SCRATCH/certgate.zcert" "$SCRATCH/tamper.zcert"; python3 -c "
import re;p='$SCRATCH/tamper.zcert';s=open(p).read();m=re.search(r'\"source_sha256\":\"([0-9a-f]{64})\"',s)
if m: h=m.group(1); s=s.replace(h, ('0' if h[0]!='0' else '1')+h[1:]); open(p,'w').write(s)"
( cd "$SCRATCH" && "$BIN" verify-cert tamper.zcert >/dev/null 2>&1 ) && no "verify_cert_tamper" "accepted tampered" || ok "verify_cert_tamper"
( cd "$SCRATCH" && "$BIN" run certgate.zs --require=zero-heap,bounded >/dev/null 2>&1 ) && ok "gate_pass" || no "gate_pass" ""
( cd "$SCRATCH" && "$BIN" run certgate.zs --require=teleportation >/dev/null 2>&1 ) && no "gate_unknown_failclosed" "ran unknown prop" || ok "gate_unknown_failclosed"

echo ""
echo "## WASM backend"
if [ -n "$WT" ] && [ -x "$WT" ]; then
  WF=$(src wmath 'fn neuron4(x0:i32,x1:i32,x2:i32,x3:i32)->i32{ return 2*x0 - x1 + 3*x2 + x3; } pub fn main(){ println(neuron4(3,1,2,1)); }')
  ( cd "$SCRATCH" && "$BIN" wasm wmath.zs >/dev/null 2>&1 )
  if [ -f "$SCRATCH/wmath.wat" ]; then
    WOUT=$("$WT" run --invoke neuron4 "$SCRATCH/wmath.wat" 3 1 2 1 2>/dev/null)
    [ "$WOUT" = "12" ] && ok "wasm_exec (=12)" || no "wasm_exec" "got '$WOUT'"
  else no "wasm_emit" "no .wat"; fi
else echo "  SKIP wasm (wasmtime not set)"; fi

echo ""
echo "## C-header import (FFI bindings)"
printf 'int do_thing(double x);\nconst char* name(void);\n' > "$SCRATCH/eng.h"
"$BIN" import "$SCRATCH/eng.h" 2>&1 | grep -q "extern fn" && ok "import_bindings" || no "import_bindings" "no extern fn emitted"

echo ""
echo "================================================================"
echo " SMOKE RESULT: $PASS passed, $FAIL failed"
[ $FAIL -gt 0 ] && printf ' Failed: %s\n' "${FAILED[*]}"
echo "================================================================"
[ $FAIL -eq 0 ]
