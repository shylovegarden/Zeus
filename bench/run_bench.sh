#!/usr/bin/env bash
# =============================================================================
# Zeus benchmark harness.
#
# Measures, on THIS machine, with REAL numbers:
#   1) SoA (Zeus-emitted) vs naive AoS (hand C) for a tight unit-stride loop.
#   2) Zeus arena bump-allocator vs malloc/free.
#
# IMPORTANT CAVEAT (read RESULTS.md): Zeus is source-to-source. All performance
# here is GCC's performance on the emitted C. `zeus build` itself compiles at
# -O0; to expose the aligned/ivdep vectorization the emitted .c is recompiled at
# -O3 -march=native, the same flags applied to the naive C counterpart. The
# comparison is therefore "same compiler, same flags, SoA layout vs AoS layout".
# =============================================================================
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$HERE"

ZEUSC="${ZEUSC:-/tmp/zeus_target/release/zeus_compiler}"
CC="${CC:-gcc}"
OPT="-O3 -march=native -fno-math-errno"

echo "============================================================"
echo " Zeus benchmark harness"
echo " compiler : $ZEUSC"
echo " cc       : $($CC --version | head -1)"
echo " flags    : $OPT"
echo " cpu      : $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //')"
echo "============================================================"

# --- repeat helper: run a binary K times, report best wall ms -----------------
best_ms() {
    local bin="$1"; local k="${2:-7}"; local best=999999999
    for _ in $(seq "$k"); do
        local t0 t1 ms
        t0=$(date +%s%N)
        "$bin" >/dev/null
        t1=$(date +%s%N)
        ms=$(( (t1 - t0) / 1000000 ))
        if [ "$ms" -lt "$best" ]; then best="$ms"; fi
    done
    echo "$best"
}

# =============================================================================
# 1. SoA vs AoS
# =============================================================================
echo
echo "[1/2] SoA throughput: Zeus-emitted SoA  vs  naive AoS C"
echo "------------------------------------------------------------"

# Build the .zs so the Zeus compiler emits soa_throughput.c (the SoA form).
echo " - zeus build soa_throughput.zs (emits SoA C; zeus's own gcc call is -O0)"
"$ZEUSC" build soa_throughput.zs >/dev/null 2>&1 || true
if [ ! -f soa_throughput.c ]; then
    echo "ERROR: zeus did not emit soa_throughput.c" >&2; exit 1
fi

# Recompile the EMITTED Zeus C at -O3 so vectorization can apply.
echo " - recompile emitted Zeus SoA C at $OPT"
$CC $OPT soa_throughput.c -o soa_zeus_o3

# Compile the naive AoS counterpart at the SAME flags.
echo " - compile naive AoS C at $OPT"
$CC $OPT soa_naive.c -o soa_naive_o3

# Element-iterations performed by the hot loop (must match the sources).
N=131072
STEPS=512
ELEMS=$(( N * STEPS ))

soa_ms=$(best_ms ./soa_zeus_o3 7)
aos_ms=$(best_ms ./soa_naive_o3 7)

soa_ns_per=$(awk "BEGIN{printf \"%.4f\", ($soa_ms*1e6)/$ELEMS}")
aos_ns_per=$(awk "BEGIN{printf \"%.4f\", ($aos_ms*1e6)/$ELEMS}")
ratio=$(awk "BEGIN{printf \"%.2f\", $aos_ms/$soa_ms}")

echo
printf " %-28s %8s ms   %10s ns/elem\n" "Zeus SoA (-O3):"   "$soa_ms" "$soa_ns_per"
printf " %-28s %8s ms   %10s ns/elem\n" "Naive AoS (-O3):"  "$aos_ms" "$aos_ns_per"
printf " %-28s %8s\n" "Speedup (AoS/SoA):" "${ratio}x"

# =============================================================================
# 2. Arena vs malloc
# =============================================================================
echo
echo "[2/2] Arena bump-allocator  vs  malloc/free"
echo "------------------------------------------------------------"
$CC $OPT arena_vs_malloc.c -o arena_vs_malloc
./arena_vs_malloc

echo
echo "Done. See RESULTS.md for recorded numbers + caveats."
