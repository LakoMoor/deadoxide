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
  (cd "$ROOT" && java -Djava.awt.headless=true --enable-final-field-mutation=ALL-UNNAMED -Djava.library.path="$PZ_NATIVES" -Xmx4g -cp "$HERE/target/original-probe:projectzomboid.jar" "$probe" "$HERE/target/oracle/pz-oracle-$name" "$@" >"$HERE/target/oracle/$name.java.txt" 2>"$HERE/target/oracle/$name.java.err") \
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
# not in the default run until it matches (first gap: one Rand draw for crafted_01_29, a water-collector entity)
sel chunkobjects && server_case chunkobjects ChunkObjectsProbe 1036 1460 1039 1463
# a whole cell (1,024 chunks): sh tools/oracle.sh chunkobjects-cell
[ "$want" = chunkobjects-cell ] && server_case chunkobjects ChunkObjectsProbe 1024 1440 1055 1471
sel collide && server_case collide CollideProbe 1036 1460 1039 1463
sel entitysprites && server_case entitysprites EntitySpritesProbe
sel specialcollide && server_case specialcollide SpecialCollideProbe 129 182
sel basements && server_case basements BasementsProbe 30 43 34 47
sel doortoggle && { cp tests/golden/engine/specialcollide.sha256 target/oracle/sc.keep 2>/dev/null; server_case specialcollide SpecialCollideProbe 129 182 toggle; mv tests/golden/engine/specialcollide.sha256 tests/golden/engine/doortoggle.sha256 2>/dev/null; cp target/oracle/sc.keep tests/golden/engine/specialcollide.sha256 2>/dev/null; }
sel los && server_case los LosProbe 129 182
sel losdoors && { server_case los LosProbe 129 182 toggle; cp tests/golden/engine/los.sha256 tests/golden/engine/losdoors.sha256 2>/dev/null; server_case los LosProbe 129 182; }
sel stairs && server_case stairs StairsProbe 126 181
# mover: a character walked over two staircases; the Rust side is the scene crate's example, not pz-dump.
mover_case() {
  args="8118.5 11615.5 0 -0.1 45 8077.5 11612.5 -0.1 0 45"
  javac -nowarn -encoding UTF-8 -cp "$ROOT/projectzomboid.jar" -d target/original-probe tests/original_probe/ServerHarness.java tests/original_probe/MoverProbe.java 2>target/oracle/mover.javac || { echo "FAIL mover (javac)"; fail=1; return; }
  (cd "$ROOT" && java -Djava.awt.headless=true --enable-final-field-mutation=ALL-UNNAMED -Djava.library.path="$PZ_NATIVES" -Xmx4g -cp "$HERE/target/original-probe:projectzomboid.jar" MoverProbe "$HERE/target/oracle/pz-oracle-mover" 126 181 $args >"$HERE/target/oracle/mover.java.txt" 2>"$HERE/target/oracle/mover.java.err") || { echo "FAIL mover (server probe)"; fail=1; return; }
  cargo run -q --release -p zomboid-scene --example mover_walk -- "$ROOT" 31 45 $args >target/oracle/mover.rust.txt 2>target/oracle/mover.rust.err || { echo "FAIL mover (rust)"; fail=1; return; }
  if cmp -s target/oracle/mover.java.txt target/oracle/mover.rust.txt; then
    cp target/oracle/mover.java.txt tests/golden/mover_stairs.txt
    echo "OK   mover: $(wc -l <target/oracle/mover.java.txt | tr -d ' ') lines identical"
  else
    echo "FAIL mover: first differences:"; diff target/oracle/mover.java.txt target/oracle/mover.rust.txt | head -6; fail=1
  fi
}
sel mover && mover_case
# movercollide: objects pushed against the walls, doors and furniture of the spawn house (and up a staircase).
movercollide_case() {
  cell=$1; rc=$2; shift 2
  javac -nowarn -encoding UTF-8 -cp "$ROOT/projectzomboid.jar" -d target/original-probe tests/original_probe/ServerHarness.java tests/original_probe/MoverCollideProbe.java 2>target/oracle/mc.javac || { echo "FAIL movercollide (javac)"; fail=1; return; }
  (cd "$ROOT" && java -Djava.awt.headless=true --enable-final-field-mutation=ALL-UNNAMED -Djava.library.path="$PZ_NATIVES" -Xmx4g -cp "$HERE/target/original-probe:projectzomboid.jar" MoverCollideProbe "$HERE/target/oracle/pz-oracle-mc" $cell "$@" >"$HERE/target/oracle/mc.java.txt" 2>"$HERE/target/oracle/mc.java.err") || { echo "FAIL movercollide (server probe)"; fail=1; return; }
  cargo run -q --release -p zomboid-scene --example mover_collide -- "$ROOT" $rc "$@" >target/oracle/mc.rust.txt 2>target/oracle/mc.rust.err || { echo "FAIL movercollide (rust)"; fail=1; return; }
  if cmp -s target/oracle/mc.java.txt target/oracle/mc.rust.txt; then
    cp target/oracle/mc.java.txt "tests/golden/mover_collide_$(echo $rc | tr ' ' '_').txt"
    echo "OK   movercollide $cell: $(wc -l <target/oracle/mc.java.txt | tr -d ' ') lines identical"
  else
    echo "FAIL movercollide $cell: first differences:"; diff target/oracle/mc.java.txt target/oracle/mc.rust.txt | head -6; fail=1
  fi
}
sel movercollide && movercollide_case "129 182" "32 45" 8303.5 11689.5 0.0697 0.0070 80 8303.5 11689.5 0.0617 0.0331 80 8303.5 11689.5 0.0443 0.0542 80 8303.5 11689.5 0.0202 0.0670 80 8303.5 11689.5 -0.0070 0.0697 80 8303.5 11689.5 -0.0331 0.0617 80 8303.5 11689.5 -0.0542 0.0443 80 8303.5 11689.5 -0.0670 0.0202 80 8303.5 11689.5 -0.0697 -0.0070 80 8303.5 11689.5 -0.0617 -0.0331 80 8303.5 11689.5 -0.0443 -0.0542 80 8303.5 11689.5 -0.0202 -0.0670 80 8303.5 11689.5 0.0070 -0.0697 80 8303.5 11689.5 0.0331 -0.0617 80 8303.5 11689.5 0.0542 -0.0443 80 8303.5 11689.5 0.0670 -0.0202 80 8288.5 11702.5 -0.05 0.01 40 8290.3 11698.2 0.06 0.06 80
sel movercollide && movercollide_case "126 181" "31 45" 8118.5 11615.5 0.001 -0.08 70
exit $fail
