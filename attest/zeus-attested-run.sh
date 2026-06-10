#!/usr/bin/env bash
#
# zeus-attested-run.sh
#
# HONEST SOFTWARE SIMULATION of hardware-bound execution attestation
# ("PUF/TPM-style binding") layered on top of Zeus's existing
# certificate + policy gate.
#
# !!! READ THIS -- SCOPE AND HONESTY !!!
# This script is a SOFTWARE SIMULATION of a Physical Unclonable Function
# (PUF) / TPM machine-binding step. It exists to demonstrate the POLICY HOOK:
# "refuse to run unless this binary is bound to THIS machine token."
#
# What is REAL here:
#   * Zeus really proves properties and signs a certificate (Ed25519).
#   * verify-cert really rejects a tampered/invalid certificate.
#   * This wrapper really REFUSES to run (non-zero exit) when the machine
#     token does not match the expected --bind value.
#
# What is SIMULATED here:
#   * The "machine token" is derived IN SOFTWARE (sha256 of /etc/machine-id,
#     or hostname + a local salt file). It is NOT read from silicon.
#   * It is therefore NOT unclonable. Anyone who can read /etc/machine-id (or
#     the salt file) on this host, or who copies those values, can reproduce
#     the token on a different machine.
#
# A REAL hardware binding requires actual hardware: a TPM 2.0 quote over PCRs,
# or an SRAM-PUF challenge-response from the device's silicon. This wrapper
# does NOT provide that. Do not rely on it for hardware security.
#
# ---------------------------------------------------------------------------
# Usage:
#   zeus-attested-run.sh <file.zs> --bind <expected_token> [--require=props]
#   zeus-attested-run.sh --show-token
#
# Exit codes:
#   0   = attested AND ran (build ok, cert valid, token matched, gate passed)
#   1   = usage / environment error
#   2   = build failed
#   3   = certificate verification failed (invalid signature/hash)
#   4   = ATTESTATION FAILED (machine token mismatch -- "inert on other silicon")
#   5   = policy gate refused execution (a required property not proven)
# ---------------------------------------------------------------------------

set -u

# ----- locate this script's own directory (for the salt file) --------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
SALT_FILE="${SCRIPT_DIR}/.attest_salt"

# ----- locate the zeus binary ----------------------------------------------
# Honor an explicit ZEUS_BIN env var; otherwise look on PATH for `zeus`.
ZEUS_BIN="${ZEUS_BIN:-}"
if [ -z "${ZEUS_BIN}" ]; then
  if command -v zeus >/dev/null 2>&1; then
    ZEUS_BIN="$(command -v zeus)"
  fi
fi

# ----- colors (disabled if not a tty) --------------------------------------
if [ -t 1 ]; then
  C_RED=$'\033[1;31m'; C_GRN=$'\033[1;32m'; C_YEL=$'\033[1;33m'
  C_CYN=$'\033[1;36m'; C_DIM=$'\033[90m'; C_RST=$'\033[0m'
else
  C_RED=''; C_GRN=''; C_YEL=''; C_CYN=''; C_DIM=''; C_RST=''
fi

say()   { printf '%s\n' "$*" >&2; }
pass()  { printf '%s[ PASS ]%s %s\n' "${C_GRN}" "${C_RST}" "$*" >&2; }
fail()  { printf '%s[ FAIL ]%s %s\n' "${C_RED}" "${C_RST}" "$*" >&2; }
info()  { printf '%s[ .... ]%s %s\n' "${C_CYN}" "${C_RST}" "$*" >&2; }
warn()  { printf '%s[ NOTE ]%s %s\n' "${C_YEL}" "${C_RST}" "$*" >&2; }

# ----- sha256 helper (portable) --------------------------------------------
sha256_of_string() {
  # reads stdin, prints lowercase hex digest only
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{print $1}'
  else
    fail "no sha256sum/shasum available on this host"
    exit 1
  fi
}

# ----- compute THIS machine's SIMULATED token ------------------------------
# Honest: this is a software-derived identifier, NOT a silicon-bound secret.
compute_machine_token() {
  local seed=""
  if [ -r /etc/machine-id ] && [ -s /etc/machine-id ]; then
    seed="machine-id:$(cat /etc/machine-id)"
  else
    # fall back to hostname + a locally-generated salt (created once)
    if [ ! -f "${SALT_FILE}" ]; then
      # generate a random salt and store it (best-effort randomness)
      if command -v od >/dev/null 2>&1 && [ -r /dev/urandom ]; then
        head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > "${SALT_FILE}"
      else
        printf '%s' "fallback-salt-$(date +%s)-$$-${RANDOM:-0}" > "${SALT_FILE}"
      fi
      chmod 600 "${SALT_FILE}" 2>/dev/null || true
    fi
    seed="hostname:$(hostname):salt:$(cat "${SALT_FILE}")"
  fi
  printf '%s' "${seed}" | sha256_of_string
}

# ===========================================================================
# Argument parsing
# ===========================================================================
SHOW_TOKEN=0
ZS_FILE=""
BIND_TOKEN=""
REQUIRE_FLAG=""   # full "--require=..." string passed through to zeus, if any

usage() {
  cat >&2 <<EOF
${C_CYN}zeus-attested-run.sh${C_RST} -- SOFTWARE SIMULATION of machine-bound attested execution

Usage:
  $0 <file.zs> --bind <expected_token> [--require=props]
  $0 --show-token

Options:
  --bind <token>     Expected SIMULATED machine token. Execution is refused
                     unless THIS machine's token matches it.
  --require=<props>  Properties to gate on (passed to 'zeus run --require=').
                     e.g. --require=zero-heap,reproducible,constant-time,bounded
  --show-token       Print THIS machine's current simulated token and exit.

Honest scope: the token is derived in SOFTWARE (sha256 of /etc/machine-id, or
hostname+salt). It is NOT unclonable and does NOT provide hardware security.
A real PUF/TPM binding requires actual hardware. See attest/README.md.
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --show-token)
      SHOW_TOKEN=1; shift ;;
    --bind)
      shift
      if [ $# -eq 0 ]; then fail "--bind requires a token argument"; usage; exit 1; fi
      BIND_TOKEN="$1"; shift ;;
    --bind=*)
      BIND_TOKEN="${1#--bind=}"; shift ;;
    --require=*)
      REQUIRE_FLAG="$1"; shift ;;
    --require)
      shift
      if [ $# -eq 0 ]; then fail "--require requires a value"; usage; exit 1; fi
      REQUIRE_FLAG="--require=$1"; shift ;;
    -h|--help)
      usage; exit 0 ;;
    -*)
      fail "unknown option: $1"; usage; exit 1 ;;
    *)
      if [ -z "${ZS_FILE}" ]; then ZS_FILE="$1"; else
        fail "unexpected extra argument: $1"; usage; exit 1
      fi
      shift ;;
  esac
done

# ===========================================================================
# Mode: --show-token
# ===========================================================================
if [ "${SHOW_TOKEN}" -eq 1 ]; then
  TOKEN="$(compute_machine_token)"
  warn "This is a SIMULATED, software-derived machine token (not silicon-bound)."
  printf '%s\n' "${TOKEN}"
  exit 0
fi

# ===========================================================================
# Mode: attested run -- validate inputs
# ===========================================================================
if [ -z "${ZS_FILE}" ]; then
  fail "no <file.zs> given"; usage; exit 1
fi
if [ ! -f "${ZS_FILE}" ]; then
  fail "source file not found: ${ZS_FILE}"; exit 1
fi
if [ -z "${BIND_TOKEN}" ]; then
  fail "--bind <expected_token> is required for an attested run"
  warn "Run '$0 --show-token' on the target machine to capture its token."
  usage; exit 1
fi
if [ -z "${ZEUS_BIN}" ] || [ ! -x "${ZEUS_BIN}" ]; then
  fail "zeus binary not found. Set ZEUS_BIN=/path/to/zeus_compiler or put 'zeus' on PATH."
  exit 1
fi

ABS_ZS="$(cd "$(dirname "${ZS_FILE}")" && pwd)/$(basename "${ZS_FILE}")"
STEM="$(basename "${ZS_FILE}")"; STEM="${STEM%.zs}"

say ""
say "${C_CYN}=== Zeus Attested Run (SOFTWARE SIMULATION) ===${C_RST}"
say "${C_DIM}Trust chain:  proof  ->  signed certificate  ->  machine binding${C_RST}"
say "${C_DIM}Module:       ${ABS_ZS}${C_RST}"
say ""

# ----- scratch dir (builds drop artifacts in CWD) --------------------------
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/zeus-attest.XXXXXX")"
cleanup() { rm -rf "${SCRATCH}" 2>/dev/null || true; }
trap cleanup EXIT
ZCERT="${SCRATCH}/${STEM}.zcert"

# ===========================================================================
# Step (a): build the module -> signed .zcert
# ===========================================================================
info "Step 1/4: building module to produce a signed certificate..."
if ! ( cd "${SCRATCH}" && "${ZEUS_BIN}" build "${ABS_ZS}" ) >"${SCRATCH}/build.log" 2>&1; then
  fail "build failed:"
  sed 's/^/        /' "${SCRATCH}/build.log" >&2
  exit 2
fi
if [ ! -f "${ZCERT}" ]; then
  fail "build did not produce expected certificate: ${ZCERT}"
  sed 's/^/        /' "${SCRATCH}/build.log" >&2
  exit 2
fi
pass "build succeeded; certificate emitted: ${STEM}.zcert"

# ===========================================================================
# Step (b): verify the certificate signature
# ===========================================================================
info "Step 2/4: verifying Ed25519 certificate signature..."
if ! "${ZEUS_BIN}" verify-cert "${ZCERT}" >"${SCRATCH}/verify.log" 2>&1; then
  fail "certificate verification FAILED (signature/hash invalid):"
  sed 's/^/        /' "${SCRATCH}/verify.log" >&2
  exit 3
fi
pass "certificate signature valid (proof chain intact)"

# ===========================================================================
# Step (c): SIMULATED machine binding check
# ===========================================================================
info "Step 3/4: checking SIMULATED machine binding..."
THIS_TOKEN="$(compute_machine_token)"
say "${C_DIM}        expected (--bind): ${BIND_TOKEN}${C_RST}"
say "${C_DIM}        this machine:      ${THIS_TOKEN}${C_RST}"
if [ "${THIS_TOKEN}" != "${BIND_TOKEN}" ]; then
  fail "ATTESTATION FAILED: binary is bound to a different machine"
  warn "Refusing to run. (Simulated 'inert on other silicon'.)"
  warn "This is a software check; a real PUF/TPM binding needs hardware."
  exit 4
fi
pass "machine token matches --bind (simulated binding satisfied)"

# ===========================================================================
# Step (d): policy gate + execute
# ===========================================================================
info "Step 4/4: gating on proven properties and executing..."
if [ -n "${REQUIRE_FLAG}" ]; then
  say "${C_DIM}        gate: zeus run ${REQUIRE_FLAG}${C_RST}"
  RUN_ARGS=( run "${ABS_ZS}" "${REQUIRE_FLAG}" )
else
  warn "no --require given; running without an additional property gate"
  RUN_ARGS=( run "${ABS_ZS}" )
fi

( cd "${SCRATCH}" && "${ZEUS_BIN}" "${RUN_ARGS[@]}" )
RUN_RC=$?

if [ "${RUN_RC}" -ne 0 ]; then
  fail "policy gate refused execution or program exited non-zero (rc=${RUN_RC})"
  exit 5
fi

say ""
pass "ATTESTED: proof verified, machine binding satisfied, policy gate passed, program executed."
exit 0
