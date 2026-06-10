#!/bin/sh
# zeus-audit.sh -- Zeus Audit Gate for AI-generated code (LLVM-IR .ll files)
#
# Runs `zeus audit` on each .ll file, captures per-file SARIF 2.1.0, merges all
# results into one SARIF file, prints a summary table, and exits non-zero if any
# file is NOT-PROVEN (or, under --strict, also UNDECIDABLE).
#
# This is a thin, honest wrapper around the real Zeus binary. The binary is the
# source of truth: this script never overrides its verdict or its exit code, it
# only aggregates them. Verdicts come from `zeus audit`:
#
#   PROVED-SAFE  : proven safe on the modeled subset       (binary exit 0)
#   UNDECIDABLE  : outside the modeled subset, NEVER false-safe
#                  (binary exit 0 normally, exit 1 under --strict)
#   NOT-PROVEN   : a real finding (secret branch/index/division/return)
#                  (binary exit 1)
#
# Usage:
#   ci/zeus-audit.sh [options] <file.ll | directory> ...
#
# Options:
#   --strict              Treat UNDECIDABLE as a gate failure (exit non-zero).
#   --sarif-out PATH      Write the merged SARIF 2.1.0 report to PATH.
#                         Default: zeus-audit.sarif in the current directory.
#   --bin PATH            Path to the zeus binary. Default: looks for
#                         $ZEUS_BIN, then `zeus`, then `zeus_compiler` on PATH.
#   --no-sarif            Skip writing the merged SARIF file.
#   -h, --help            Show this help.
#
# Exit codes:
#   0  all audited files PASSED the gate
#   1  at least one file FAILED the gate (NOT-PROVEN, or UNDECIDABLE in --strict)
#   2  usage / environment error (no files, missing binary, etc.)
#
# Portability: POSIX sh only. Uses jq if available for SARIF merging, otherwise
# falls back to a pure-sh/awk merge. No bashisms.

set -u

PROG="zeus-audit.sh"

# ---------------------------------------------------------------------------
# Defaults / argument parsing
# ---------------------------------------------------------------------------
STRICT=0
WRITE_SARIF=1
SARIF_OUT="zeus-audit.sarif"
ZEUS="${ZEUS_BIN:-}"
INPUTS=""

usage() {
    sed -n '2,40p' "$0" | sed 's/^# \{0,1\}//'
}

err() {
    printf '%s: %s\n' "$PROG" "$*" >&2
}

while [ $# -gt 0 ]; do
    case "$1" in
        --strict)      STRICT=1 ;;
        --no-sarif)    WRITE_SARIF=0 ;;
        --sarif-out)
            [ $# -ge 2 ] || { err "--sarif-out requires a path"; exit 2; }
            SARIF_OUT="$2"; shift ;;
        --sarif-out=*) SARIF_OUT="${1#*=}" ;;
        --bin)
            [ $# -ge 2 ] || { err "--bin requires a path"; exit 2; }
            ZEUS="$2"; shift ;;
        --bin=*)       ZEUS="${1#*=}" ;;
        -h|--help)     usage; exit 0 ;;
        --)            shift; while [ $# -gt 0 ]; do INPUTS="$INPUTS
$1"; shift; done; break ;;
        -*)            err "unknown option: $1"; exit 2 ;;
        *)             INPUTS="$INPUTS
$1" ;;
    esac
    shift
done

# ---------------------------------------------------------------------------
# Locate the zeus binary
# ---------------------------------------------------------------------------
if [ -z "$ZEUS" ]; then
    if command -v zeus >/dev/null 2>&1; then
        ZEUS="zeus"
    elif command -v zeus_compiler >/dev/null 2>&1; then
        ZEUS="zeus_compiler"
    else
        err "could not find the zeus binary on PATH."
        err "Set ZEUS_BIN=/path/to/zeus_compiler or pass --bin /path/to/zeus."
        exit 2
    fi
fi
# Verify it is runnable (resolve to absolute if a bare command name).
if ! command -v "$ZEUS" >/dev/null 2>&1 && [ ! -x "$ZEUS" ]; then
    err "zeus binary not found or not executable: $ZEUS"
    exit 2
fi

# ---------------------------------------------------------------------------
# Expand inputs (files + directories) into a list of .ll files
# ---------------------------------------------------------------------------
FILELIST=$(mktemp 2>/dev/null || echo "/tmp/zeus_audit_files.$$")
: > "$FILELIST"

# Iterate over the newline-separated INPUTS safely.
OLDIFS=$IFS
IFS='
'
for item in $INPUTS; do
    [ -n "$item" ] || continue
    if [ -d "$item" ]; then
        # Find all .ll files in the directory (recursive).
        find "$item" -type f -name '*.ll' 2>/dev/null | sort >> "$FILELIST"
    elif [ -f "$item" ]; then
        printf '%s\n' "$item" >> "$FILELIST"
    else
        # Allow shell globs that the caller already expanded, and warn on misses.
        err "not a file or directory (skipping): $item"
    fi
done
IFS=$OLDIFS

if [ ! -s "$FILELIST" ]; then
    err "no .ll files to audit."
    err "Usage: $PROG [--strict] [--sarif-out PATH] <file.ll | dir> ..."
    rm -f "$FILELIST"
    exit 2
fi

# ---------------------------------------------------------------------------
# Run the audit per file
# ---------------------------------------------------------------------------
WORKDIR=$(mktemp -d 2>/dev/null || { d="/tmp/zeus_audit.$$"; mkdir -p "$d"; echo "$d"; })
RESULTS="$WORKDIR/results.tsv"   # file<TAB>verdict<TAB>findings<TAB>passfail
: > "$RESULTS"
SARIF_PARTS="$WORKDIR/parts"
mkdir -p "$SARIF_PARTS"

STRICT_FLAG=""
[ "$STRICT" -eq 1 ] && STRICT_FLAG="--strict"

GATE_EXIT=0
N_PROVED=0
N_UNDECIDABLE=0
N_NOTPROVEN=0
idx=0

while IFS= read -r f; do
    [ -n "$f" ] || continue
    idx=$((idx + 1))
    sarif_file="$SARIF_PARTS/part_$idx.sarif"

    # Run the audit. The binary emits SARIF to stdout with --sarif and reflects
    # the gate verdict in its exit code. IMPORTANT: flag order matters -- the
    # real binary stops scanning options after --sarif, so --strict (when used)
    # MUST precede --sarif, otherwise --strict is silently ignored.
    # (The real binary has no --sarif-out flag, so we capture stdout ourselves.)
    "$ZEUS" audit "$f" $STRICT_FLAG --sarif > "$sarif_file" 2>/dev/null
    rc=$?

    # Derive the human-readable verdict from the SARIF + exit code.
    # Mapping discovered from the real binary:
    #   empty results + rc 0                   -> PROVED-SAFE
    #   ZEUS-UNDECIDABLE present (only notes)  -> UNDECIDABLE
    #   any error-level finding                -> NOT-PROVEN
    n_results=$(grep -o '"ruleId"' "$sarif_file" 2>/dev/null | wc -l | tr -d ' ')
    has_error=$(grep -o '"level":"error"' "$sarif_file" 2>/dev/null | wc -l | tr -d ' ')
    has_undec=$(grep -o 'ZEUS-UNDECIDABLE' "$sarif_file" 2>/dev/null | wc -l | tr -d ' ')

    if [ "$has_error" -gt 0 ]; then
        verdict="NOT-PROVEN"
        N_NOTPROVEN=$((N_NOTPROVEN + 1))
    elif [ "$has_undec" -gt 0 ]; then
        verdict="UNDECIDABLE"
        N_UNDECIDABLE=$((N_UNDECIDABLE + 1))
    elif [ "$n_results" -eq 0 ]; then
        verdict="PROVED-SAFE"
        N_PROVED=$((N_PROVED + 1))
    else
        # Unexpected shape: be conservative, never report false-safe.
        verdict="UNDECIDABLE"
        N_UNDECIDABLE=$((N_UNDECIDABLE + 1))
    fi

    # The gate result is the binary's own exit code -- the source of truth.
    if [ "$rc" -eq 0 ]; then
        passfail="PASS"
    else
        passfail="FAIL"
        GATE_EXIT=1
    fi

    printf '%s\t%s\t%s\t%s\n' "$f" "$verdict" "$n_results" "$passfail" >> "$RESULTS"
done < "$FILELIST"

# ---------------------------------------------------------------------------
# Merge per-file SARIF into one report
# ---------------------------------------------------------------------------
if [ "$WRITE_SARIF" -eq 1 ]; then
    if command -v jq >/dev/null 2>&1; then
        # Concatenate all results arrays into a single run.
        jq -s '
          {
            version: "2.1.0",
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            runs: [
              {
                tool: { driver: { name: "zeus", version: "0.1.0", informationUri: "https://zeus-lang.dev" } },
                results: [ .[].runs[0].results[] ]
              }
            ]
          }
        ' "$SARIF_PARTS"/part_*.sarif > "$SARIF_OUT" 2>/dev/null \
            && SARIF_OK=1 || SARIF_OK=0
    else
        SARIF_OK=0
    fi

    if [ "${SARIF_OK:-0}" -ne 1 ]; then
        # Pure-sh fallback: extract each file's results[...] inner content and
        # concatenate. Each part is a single-line SARIF object from the binary.
        {
            printf '%s' '{"version":"2.1.0","$schema":"https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json","runs":[{"tool":{"driver":{"name":"zeus","version":"0.1.0","informationUri":"https://zeus-lang.dev"}},"results":['
            first=1
            for p in "$SARIF_PARTS"/part_*.sarif; do
                [ -f "$p" ] || continue
                # Pull out the substring between the first `"results":[` and its
                # matching trailing `]}]}` produced by the binary.
                inner=$(sed -n 's/.*"results":\[\(.*\)\]}\]}.*/\1/p' "$p")
                [ -n "$inner" ] || continue
                if [ "$first" -eq 1 ]; then
                    printf '%s' "$inner"; first=0
                else
                    printf ',%s' "$inner"
                fi
            done
            printf '%s' ']}]}'
        } > "$SARIF_OUT"
    fi
fi

# ---------------------------------------------------------------------------
# Summary table
# ---------------------------------------------------------------------------
TOTAL=$idx

echo
echo "================================ ZEUS AUDIT GATE ================================"
[ "$STRICT" -eq 1 ] && echo "mode: STRICT (UNDECIDABLE fails the gate)" || echo "mode: default (UNDECIDABLE passes with caveats)"
echo "binary: $ZEUS"
echo "--------------------------------------------------------------------------------"
printf '%-40s  %-12s  %-8s  %s\n' "FILE" "VERDICT" "FINDINGS" "GATE"
echo "--------------------------------------------------------------------------------"
while IFS="	" read -r f verdict nfind passfail; do
    # Trim long paths from the left for readability but keep them unambiguous.
    disp="$f"
    if [ "${#disp}" -gt 40 ]; then
        disp="...$(printf '%s' "$f" | tail -c 37)"
    fi
    printf '%-40s  %-12s  %-8s  %s\n' "$disp" "$verdict" "$nfind" "$passfail"
done < "$RESULTS"
echo "--------------------------------------------------------------------------------"
printf 'totals: %d file(s)  |  PROVED-SAFE %d  UNDECIDABLE %d  NOT-PROVEN %d\n' \
    "$TOTAL" "$N_PROVED" "$N_UNDECIDABLE" "$N_NOTPROVEN"
[ "$WRITE_SARIF" -eq 1 ] && echo "merged SARIF -> $SARIF_OUT"
echo "--------------------------------------------------------------------------------"

if [ "$GATE_EXIT" -eq 0 ]; then
    echo "[ZEUS AUDIT GATE] PASSED -- all $TOTAL file(s) cleared the gate."
else
    echo "[ZEUS AUDIT GATE] FAILED -- at least one file did not pass."
fi
echo "================================================================================"

# Cleanup temp scratch (keep the SARIF output).
rm -rf "$WORKDIR" "$FILELIST"

exit $GATE_EXIT
