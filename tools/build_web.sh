#!/bin/sh
# Build the browser version into site/play/pkg (committed: GitHub Pages serves ./site as it is).
# Needs: rustup target add wasm32-unknown-unknown; cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
cd "$(dirname "$0")/.." || exit 2
export PATH="$HOME/.cargo/bin:$PATH"
set -e
cargo build -q --release --target wasm32-unknown-unknown -p zomboid-web
rm -rf site/play/pkg
wasm-bindgen --target web --no-typescript --out-dir site/play/pkg target/wasm32-unknown-unknown/release/zomboid_web.wasm
rm -f site/play/pkg/.gitignore
ls -la site/play/pkg
node tools/web_smoke.mjs .. "$(cargo run -q --release -p zomboid-scene --example scene_stats -- .. 32 45)"
