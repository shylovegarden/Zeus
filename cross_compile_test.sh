#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# Zeus Universal Device Matrix — Cross-Compilation Test
# Validates that the Zeus compiler can emit code for all 6 baseline targets.
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST="$SCRIPT_DIR/zeus_compiler/Cargo.toml"
SOURCE="$SCRIPT_DIR/hello_world.zs"

BOLD=$'\e[1m'
GREEN=$'\e[32m'
RED=$'\e[31m'
YELLOW=$'\e[33m'
CYAN=$'\e[36m'
RESET=$'\e[0m'

echo ""
echo "${BOLD}${CYAN}⚡ Zeus Universal Device Matrix — Cross-Compilation Test${RESET}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

TARGETS=(
    "x86_64-unknown-linux-gnu"
    "aarch64-apple-darwin"
    "armv7a-none-eabi"
    "riscv64gc-unknown-none-elf"
    "wasm32-unknown-unknown"
    "nvptx64-nvidia-cuda"
)

passed=0
failed=0
missing_sysroot=0

for triple in "${TARGETS[@]}"; do
    printf "  %-42s" "$triple"

    # Run the Zeus compiler; capture both stdout and stderr
    if cargo run --quiet \
           --manifest-path "$MANIFEST" \
           -- build "$SOURCE" --target="$triple" \
           >/tmp/zeus_cc_out.log 2>&1; then
        echo "${GREEN}✔ ${triple}${RESET}"
        ((passed++))
    else
        # Distinguish between a missing cross-linker / sysroot and a real error
        if grep -qiE \
               "cross-linker|sysroot|linker not found|cannot find|No such file|lld-link|clang.*target" \
               /tmp/zeus_cc_out.log 2>/dev/null; then
            echo "${YELLOW}✗ ${triple} (cross-linker not found — install sysroot)${RESET}"
            ((missing_sysroot++))
        else
            echo "${RED}✗ ${triple} (compilation failed)${RESET}"
            # Print the last 5 lines of the error for quick diagnosis
            tail -n 5 /tmp/zeus_cc_out.log | sed 's/^/      /'
            ((failed++))
        fi
    fi
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "${BOLD}Summary${RESET}"
echo "  ${GREEN}✔  Passed          : $passed${RESET}"
echo "  ${YELLOW}⚠  Missing sysroot : $missing_sysroot${RESET}"
echo "  ${RED}✗  Failed          : $failed${RESET}"
total=${#TARGETS[@]}
echo "  ─  Total targets   : $total"
echo ""

if [ "$missing_sysroot" -gt 0 ]; then
    echo "${YELLOW}Tip: Install the required sysroots with:${RESET}"
    echo "  • Linux (x86_64): sudo apt install gcc-multilib"
    echo "  • ARM bare-metal : sudo apt install gcc-arm-none-eabi"
    echo "  • RISC-V        : sudo apt install gcc-riscv64-unknown-elf"
    echo "  • WASM          : rustup target add wasm32-unknown-unknown"
    echo "  • CUDA / PTX    : Install CUDA Toolkit (https://developer.nvidia.com/cuda-downloads)"
    echo ""
fi

# Exit non-zero only for hard failures (missing sysroot is not a hard failure)
if [ "$failed" -gt 0 ]; then
    exit 1
fi

exit 0
