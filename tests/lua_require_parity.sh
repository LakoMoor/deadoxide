#!/bin/sh
# Diffs the harness's require/recursive-require warnings against the real client's log (tests/golden/lua_real).
cd "$(dirname "$0")/.." || exit 2
cargo run -q --release -p zomboid-lua -- .. run 2>&1 | grep -E "require|can.t find map objects|Mannequin zone" | sed 's/^ *//' | sort > /tmp/h_req.txt
grep -E "require|can.t find map objects|Mannequin zone" tests/golden/lua_real/client_lua_warnings.txt | sed 's/^ *//' | sort > /tmp/g_req.txt
diff /tmp/g_req.txt /tmp/h_req.txt && echo "require + map-objects + Mannequin warnings IDENTICAL ($(wc -l < /tmp/g_req.txt) kinds)"
