#!/bin/sh
# Zeus build/install helper.
# Checks for a Rust toolchain and a C compiler, builds the release binary,
# and prints where it landed plus a next step. Works on Linux and macOS.
#
# Usage:
#   ./install.sh
#
# Optional: set CARGO_TARGET_DIR to redirect build output, e.g.
#   CARGO_TARGET_DIR=/tmp/zeus-build ./install.sh

set -e

say()  { printf '%s\n' "$*"; }
err()  { printf 'error: %s\n' "$*" >&2; }
die()  { err "$*"; exit 1; }

# --- locate the crate ---------------------------------------------------------
# Resolve the directory this script lives in, so it works from any cwd.
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
CRATE_DIR="$SCRIPT_DIR/zeus_compiler"

if [ ! -f "$CRATE_DIR/Cargo.toml" ]; then
    die "could not find zeus_compiler/Cargo.toml next to this script (looked in $CRATE_DIR). Run install.sh from the Zeus repo root."
fi

# --- prerequisite checks ------------------------------------------------------
need() {
    command -v "$1" >/dev/null 2>&1
}

if ! need rustc || ! need cargo; then
    err "a Rust toolchain (rustc + cargo) is required but was not found on PATH."
    err "install it from https://rustup.rs and re-run this script."
    exit 1
fi
say "found Rust:  $(rustc --version)"

# Zeus prefers clang and falls back to gcc; require at least one.
CC_FOUND=""
if need clang; then
    CC_FOUND="clang"
elif need gcc; then
    CC_FOUND="gcc"
fi
if [ -z "$CC_FOUND" ]; then
    err "a C compiler (clang or gcc) is required but was not found on PATH."
    err "  macOS: run 'xcode-select --install' to get clang."
    err "  Linux: install clang or gcc via your package manager (e.g. apt install clang)."
    exit 1
fi
say "found C compiler:  $CC_FOUND ($($CC_FOUND --version 2>/dev/null | head -n 1))"

# --- build --------------------------------------------------------------------
say ""
say "building the Zeus compiler (release)... this may take a few minutes the first time."
( cd "$CRATE_DIR" && cargo build --release ) || die "cargo build failed (see output above)."

# --- locate the produced binary ----------------------------------------------
if [ -n "$CARGO_TARGET_DIR" ]; then
    TARGET_DIR="$CARGO_TARGET_DIR"
else
    TARGET_DIR="$CRATE_DIR/target"
fi

# The binary is named after the crate (zeus_compiler).
BIN="$TARGET_DIR/release/zeus_compiler"
if [ ! -x "$BIN" ]; then
    BIN=$(find "$TARGET_DIR/release" -maxdepth 1 -type f -perm -u+x 2>/dev/null | grep -v '\.' | head -n 1 || true)
fi

say ""
if [ -n "$BIN" ] && [ -x "$BIN" ]; then
    say "Build complete. Zeus binary:"
    say "  $BIN"
    say ""
    say "Next step -- prove and gate the constant-time crypto demo:"
    say "  $BIN cert showcase/flagship/crypto_sbox.zs"
    say "  $BIN run  showcase/flagship/crypto_sbox.zs --require=constant-time"
    say ""
    say "Or during development you can always use:"
    say "  ( cd zeus_compiler && cargo run --release -- <command> [file.zs] )"
    say ""
    say "See QUICKSTART.md and TUTORIAL.md to go further."
else
    say "Build completed, but the binary could not be located automatically."
    say "Look under: $TARGET_DIR/release/"
    say "You can always run Zeus via: ( cd zeus_compiler && cargo run --release -- <command> )"
fi
