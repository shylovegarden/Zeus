#!/usr/bin/env bash
# =============================================================================
# Zeus compiler regression test harness
# =============================================================================
# For each tests/cases/*.zs:
#   * Positive tests: build with the Zeus compiler, run the produced binary,
#     and compare the NORMALIZED compiler stdout markers (plus a clean run
#     exit code) against the sibling tests/cases/<name>.expected golden file.
#   * Negative tests (name starts with "neg_"): the compiler is EXPECTED to
#     fail to compile. A non-zero build exit code counts as PASS; a successful
#     build counts as FAIL. Their .expected file is ignored (may be empty).
#
# Why markers and not program output?  The Zeus `print` builtin is a stub that
# only ever prints "Execution complete.", and every produced binary currently
# exits 0, so program stdout cannot encode a result. Instead we golden the
# compiler's own deterministic stdout markers ([ZEUS VERIFIED], Build Success,
# etc.) after stripping ANSI colors and volatile timing/paths. See README.md.
#
# Usage:
#   tests/run_tests.sh                # run every case
#   tests/run_tests.sh foo bar        # run only cases named foo, bar
#   REFRESH=1 tests/run_tests.sh      # (re)generate .expected golden files
#   ZEUS_BIN=/path/to/zeus_compiler tests/run_tests.sh
#
# Env vars:
#   ZEUS_BIN   path to the zeus_compiler binary
#              (default: /tmp/zeus_target/release/zeus_compiler)
#   REFRESH    if set to 1, overwrite .expected files instead of comparing
# =============================================================================

set -u

# --- locate things ----------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CASES_DIR="$SCRIPT_DIR/cases"
ZEUS_BIN="${ZEUS_BIN:-/tmp/zeus_target/release/zeus_compiler}"
REFRESH="${REFRESH:-0}"

if [ ! -x "$ZEUS_BIN" ]; then
    echo "ERROR: Zeus compiler not found / not executable at: $ZEUS_BIN" >&2
    echo "       Set ZEUS_BIN to point at the zeus_compiler binary." >&2
    exit 2
fi

# --- gcc->clang shim --------------------------------------------------------
# The compiler shells out to `clang` for native compilation. If clang is not on
# PATH but gcc is, drop a tiny shim into a temp dir and prepend it to PATH so
# the build can proceed using gcc as the real backend.
SHIM_DIR=""
if ! command -v clang >/dev/null 2>&1; then
    if command -v gcc >/dev/null 2>&1; then
        SHIM_DIR="$(mktemp -d)"
        cat > "$SHIM_DIR/clang" <<'SHIM'
#!/usr/bin/env bash
exec gcc "$@"
SHIM
        chmod +x "$SHIM_DIR/clang"
        export PATH="$SHIM_DIR:$PATH"
        echo "[harness] clang not found; using gcc shim at $SHIM_DIR"
    else
        echo "[harness] WARNING: neither clang nor gcc found; builds may fail" >&2
    fi
fi

cleanup() {
    [ -n "$SHIM_DIR" ] && rm -rf "$SHIM_DIR"
}
trap cleanup EXIT

# --- normalization ----------------------------------------------------------
# Turn raw compiler stdout+stderr into a stable, comparable form:
#   * strip ANSI escape sequences
#   * keep only deterministic marker lines
#   * drop volatile timing ("(Total Time: ...)") and trailing whitespace
normalize() {
    sed -r 's/\x1b\[[0-9;]*[a-zA-Z]//g' \
      | grep -E '\[ZEUS VERIFIED\]|\[ZEUS SMT-SOLVER\]|Build Success|Compilation (failed|error)|error\[' \
      | sed -E 's/\(Total Time:[^)]*\)//; s/[[:space:]]+$//'
}

# --- discover cases ---------------------------------------------------------
shopt -s nullglob
ALL_CASES=()
for f in "$CASES_DIR"/*.zs; do
    ALL_CASES+=("$(basename "$f" .zs)")
done
shopt -u nullglob

if [ ${#ALL_CASES[@]} -eq 0 ]; then
    echo "ERROR: no .zs cases found in $CASES_DIR" >&2
    exit 2
fi

# optional filter from CLI args
SELECTED=()
if [ $# -gt 0 ]; then
    for want in "$@"; do
        found=0
        for c in "${ALL_CASES[@]}"; do
            [ "$c" = "$want" ] && { SELECTED+=("$c"); found=1; }
        done
        [ "$found" -eq 0 ] && echo "WARNING: no such case: $want" >&2
    done
else
    SELECTED=("${ALL_CASES[@]}")
fi

# --- run in an isolated build dir so we never touch the source tree ----------
WORK="$(mktemp -d)"
trap 'cleanup; rm -rf "$WORK"' EXIT

PASS=0
FAIL=0
FAILED_NAMES=()

printf '%s\n' "============================================================"
printf '%s\n' " Zeus test harness"
printf '%s\n' "   compiler : $ZEUS_BIN"
printf '%s\n' "   cases    : $CASES_DIR (${#SELECTED[@]} selected)"
[ "$REFRESH" = "1" ] && printf '%s\n' "   MODE     : REFRESH (regenerating golden files)"
printf '%s\n' "============================================================"

for name in "${SELECTED[@]}"; do
    src="$CASES_DIR/$name.zs"
    exp="$CASES_DIR/$name.expected"

    # fresh per-case build dir (compiler writes binary into cwd)
    bdir="$WORK/$name"
    mkdir -p "$bdir"
    cp "$src" "$bdir/$name.zs"

    # build
    build_out="$( cd "$bdir" && "$ZEUS_BIN" build "$name.zs" 2>&1 )"
    build_rc=$?

    # ----- negative test path -----
    case "$name" in
        neg_*)
            if [ "$build_rc" -ne 0 ]; then
                printf ' PASS  %-28s (neg: compile failed as expected, rc=%d)\n' "$name" "$build_rc"
                PASS=$((PASS+1))
            else
                printf ' FAIL  %-28s (neg: expected compile FAILURE but build succeeded)\n' "$name"
                FAIL=$((FAIL+1)); FAILED_NAMES+=("$name")
            fi
            continue
            ;;
    esac

    # ----- positive test path -----
    if [ "$build_rc" -ne 0 ]; then
        printf ' FAIL  %-28s (build failed, rc=%d)\n' "$name" "$build_rc"
        echo "$build_out" | sed -r 's/\x1b\[[0-9;]*[a-zA-Z]//g' | sed 's/^/        | /' | tail -n 8
        FAIL=$((FAIL+1)); FAILED_NAMES+=("$name")
        continue
    fi

    # run the produced binary; it must exist and exit 0 (current Zeus invariant)
    run_rc=0
    if [ -x "$bdir/$name" ]; then
        ( cd "$bdir" && "./$name" >/dev/null 2>&1 ); run_rc=$?
    else
        printf ' FAIL  %-28s (no binary produced)\n' "$name"
        FAIL=$((FAIL+1)); FAILED_NAMES+=("$name")
        continue
    fi

    actual="$(printf '%s\n' "$build_out" | normalize)"

    # refresh mode: write golden and continue
    if [ "$REFRESH" = "1" ]; then
        printf '%s\n' "$actual" > "$exp"
        printf ' WROTE %-28s (golden refreshed)\n' "$name"
        PASS=$((PASS+1))
        continue
    fi

    if [ ! -f "$exp" ]; then
        printf ' FAIL  %-28s (missing golden file; run REFRESH=1)\n' "$name"
        FAIL=$((FAIL+1)); FAILED_NAMES+=("$name")
        continue
    fi

    expected="$(cat "$exp")"

    if [ "$run_rc" -ne 0 ]; then
        printf ' FAIL  %-28s (binary exited non-zero: rc=%d)\n' "$name" "$run_rc"
        FAIL=$((FAIL+1)); FAILED_NAMES+=("$name")
        continue
    fi

    if [ "$actual" = "$expected" ]; then
        printf ' PASS  %-28s\n' "$name"
        PASS=$((PASS+1))
    else
        printf ' FAIL  %-28s (marker mismatch)\n' "$name"
        diff <(printf '%s\n' "$expected") <(printf '%s\n' "$actual") \
            | sed 's/^/        /' | head -n 20
        FAIL=$((FAIL+1)); FAILED_NAMES+=("$name")
    fi
done

printf '%s\n' "------------------------------------------------------------"
printf ' RESULT: %d passed, %d failed (of %d)\n' "$PASS" "$FAIL" "$((PASS+FAIL))"
if [ "$FAIL" -gt 0 ]; then
    printf ' Failed: %s\n' "${FAILED_NAMES[*]}"
    exit 1
fi
exit 0
