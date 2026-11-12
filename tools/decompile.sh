#!/bin/sh
# Decompile the game's own classes from the pinned jar into ./decompiled (git-ignored, never committed).
# The Java text is the porting source: each Rust item names the class/method it follows, see tools/class_ledger.mjs.
#   sh tools/decompile.sh            uses ../projectzomboid.jar
#   sh tools/decompile.sh <jar>
cd "$(dirname "$0")/.." || exit 2
JAR="${1:-../projectzomboid.jar}"
VF_VER=1.11.1
VF_SHA=a615d07ddbbcd489369674f40e42df639c32be95410890b38f173d5c1e2ea39c
VF=target/tools/vineflower.jar

want=$(node -e 'console.log(require("./reference.lock.json").jar_sha256.toLowerCase())') || exit 2
have=$(shasum -a 256 "$JAR" | cut -d' ' -f1)
if [ "$want" != "$have" ]; then
  echo "jar sha256 $have differs from reference.lock.json ($want): findings do not carry across builds" >&2
  exit 1
fi

mkdir -p target/tools
if [ ! -f "$VF" ]; then
  curl -sSfL -o "$VF" "https://repo1.maven.org/maven2/org/vineflower/vineflower/$VF_VER/vineflower-$VF_VER.jar" || exit 1
fi
[ "$(shasum -a 256 "$VF" | cut -d' ' -f1)" = "$VF_SHA" ] || { echo "vineflower checksum mismatch" >&2; exit 1; }

rm -rf target/pzclasses decompiled
mkdir -p target/pzclasses decompiled
JAR_ABS=$(cd "$(dirname "$JAR")" && pwd)/$(basename "$JAR")
# Only packages written by the game's authors; third-party libraries stay as a classpath for type resolution.
(cd target/pzclasses && unzip -q "$JAR_ABS" 'zombie/*' 'se/krka/*' 'fmod/*' 'generation/*' 'astar/*' 'N3D/*' 'de/*' && jar cf ../pz-own.jar .) || exit 1
java -Xmx8g -jar "$VF" -thr=8 -log=WARN -e="$JAR_ABS" target/pz-own.jar decompiled > target/vineflower.log 2>&1 || { tail -20 target/vineflower.log; exit 1; }
echo "$(find decompiled -name '*.java' | wc -l) files, $(find decompiled -name '*.java' -print0 | xargs -0 cat | wc -l) lines in ./decompiled"
