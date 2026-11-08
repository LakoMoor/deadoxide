#!/bin/sh
# Self-check before you report (analogue of Horizon's tools/pr_ready.py): fmt, clippy -D warnings, tests, parity checks.
#   sh tools/check.sh          full
#   sh tools/check.sh --quick  skip the Lua parity run
cd "$(dirname "$0")/.." || exit 2
export PATH="$HOME/.cargo/bin:$PATH"
fail=0
step() { printf '\n== %s\n' "$1"; shift; "$@" || { echo "FAILED: $*"; fail=1; }; }
step "cargo fmt --check"          cargo fmt --all -- --check
step "cargo clippy -D warnings"   cargo clippy -q --release --workspace --all-targets -- -D warnings
step "cargo test"                 cargo test -q --release --workspace
if [ "$1" != "--quick" ]; then
  step "Lua require/map-objects/Mannequin warnings vs the real client log" sh tests/lua_require_parity.sh
fi
[ $fail = 0 ] && echo "\nALL CHECKS PASSED" || echo "\nSOME CHECKS FAILED"
exit $fail
