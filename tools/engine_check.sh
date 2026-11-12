#!/bin/sh
# Gate for the direct port: run before every commit that touches crates/zomboid-engine. Exits non-zero on any failure.
cd "$(dirname "$0")/.." || exit 2
export PATH="$HOME/.cargo/bin:$PATH"
set -e
cargo fmt --all -- --check
cargo clippy -q --release -p zomboid-engine --all-targets -- -D warnings
sh tools/oracle.sh
cargo test -q --release -p zomboid-engine
node tools/class_ledger.mjs
echo "ENGINE CHECK PASSED"
