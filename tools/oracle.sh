#!/bin/sh
# Differential check of the direct port: run a Java probe on the ORIGINAL jar and `pz-dump` on the Rust port over the
# same install, and compare the two outputs byte for byte. Nothing game-derived is committed; only sha256 of the
# oracle output is kept in tests/golden/engine/<case>.sha256 so `cargo test` can check the port without Java.
#   sh tools/oracle.sh            all cases
#   sh tools/oracle.sh tiledefs   one case
cd "$(dirname "$0")/.." || exit 2
export PATH="$HOME/.cargo/bin:$PATH"
ROOT=$(cd .. && pwd)
HERE=$(pwd)
mkdir -p target/oracle target/original-probe tests/golden/engine
cargo build -q --release -p zomboid-engine || exit 1
fail=0
# case <name> <ProbeClass> [probe args...]   (pz-dump gets: <name> <root> [args...])
run_case() {
  name=$1; probe=$2; shift 2
  javac -nowarn -encoding UTF-8 -cp "$ROOT/projectzomboid.jar" -d target/original-probe "tests/original_probe/$probe.java" 2>target/oracle/$name.javac \
    || { cat target/oracle/$name.javac; echo "FAIL $name (javac)"; fail=1; return; }
  (cd "$ROOT" && java -cp "$HERE/target/original-probe:projectzomboid.jar" "$probe" "$@" >"$HERE/target/oracle/$name.java.txt" 2>"$HERE/target/oracle/$name.java.err") \
    || { tail -5 target/oracle/$name.java.err; echo "FAIL $name (java probe)"; fail=1; return; }
  ./target/release/pz-dump "$name" "$ROOT" "$@" >target/oracle/$name.rust.txt 2>target/oracle/$name.rust.err \
    || { tail -5 target/oracle/$name.rust.err; echo "FAIL $name (pz-dump)"; fail=1; return; }
  if cmp -s target/oracle/$name.java.txt target/oracle/$name.rust.txt; then
    shasum -a 256 target/oracle/$name.java.txt | cut -d' ' -f1 >tests/golden/engine/$name.sha256
    echo "OK   $name: $(wc -l <target/oracle/$name.java.txt | tr -d ' ') lines identical"
  else
    echo "FAIL $name: first differences:"; diff target/oracle/$name.java.txt target/oracle/$name.rust.txt | head -6; fail=1
  fi
}
# server_case <name> <ProbeClass> [args...]: the probe runs INSIDE the fully booted original (ServerHarness.java): a
# headless dedicated server from the pinned jar, natives for this OS from $PZ_NATIVES, fresh scratch cache dir, fixed seed.
PZ_NATIVES=${PZ_NATIVES:-"$HOME/Library/Application Support/Steam/steamapps/common/ProjectZomboid/Project Zomboid.app/Contents/Java"}
server_case() {
  name=$1; probe=$2; shift 2
  [ -d "$PZ_NATIVES" ] || { echo "FAIL $name: no natives in $PZ_NATIVES (set PZ_NATIVES)"; fail=1; return; }
  javac -nowarn -encoding UTF-8 -cp "$ROOT/projectzomboid.jar" -d target/original-probe tests/original_probe/ServerHarness.java "tests/original_probe/$probe.java" 2>target/oracle/$name.javac \
    || { cat target/oracle/$name.javac; echo "FAIL $name (javac)"; fail=1; return; }
  (cd "$ROOT" && java -Djava.awt.headless=true -Djava.library.path="$PZ_NATIVES" -Xmx4g -cp "$HERE/target/original-probe:projectzomboid.jar" "$probe" "$HERE/target/oracle/pz-oracle-$name" "$@" >"$HERE/target/oracle/$name.java.txt" 2>"$HERE/target/oracle/$name.java.err") \
    || { grep -n 'ORACLE\|Exception' target/oracle/$name.java.err | tail -5; echo "FAIL $name (server probe)"; fail=1; return; }
  ./target/release/pz-dump "$name" "$ROOT" "$@" >target/oracle/$name.rust.txt 2>target/oracle/$name.rust.err \
    || { tail -5 target/oracle/$name.rust.err; echo "FAIL $name (pz-dump)"; fail=1; return; }
  if cmp -s target/oracle/$name.java.txt target/oracle/$name.rust.txt; then
    shasum -a 256 target/oracle/$name.java.txt | cut -d' ' -f1 >tests/golden/engine/$name.sha256
    echo "OK   $name: $(wc -l <target/oracle/$name.java.txt | tr -d ' ') lines identical"
  else
    echo "FAIL $name: first differences:"; diff target/oracle/$name.java.txt target/oracle/$name.rust.txt | head -6; fail=1
  fi
}
want=${1:-all}
sel() { [ "$want" = all ] || [ "$want" = "$1" ]; }
sel tiledefs && run_case tiledefs TileDefsProbe
sel lotheaders && run_case lotheaders LotHeadersProbe "Muldraugh, KY"
sel lotpack && run_case lotpack LotPackProbe "Muldraugh, KY" 30 43 34 47
# the whole map (4.2 GB of lotpacks, minutes): sh tools/oracle.sh lotpack-all
[ "$want" = lotpack-all ] && run_case lotpack LotPackProbe "Muldraugh, KY" 0 0 99 99
sel metagrid && server_case metagrid MetaGridProbe "Muldraugh, KY" 30 43 34 47
exit $fail
