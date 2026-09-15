#!/usr/bin/env bash
# Experimental Android runtime smoke test for naga-js.
#
# Why this looks the way it does:
# - Node.js ships no official Android build, so there is no `setup-node`
#   equivalent on device. We borrow the community-maintained Termux `nodejs`
#   build by extracting its .debs directly (no Termux app needed).
# - Our Android artifacts are arm64/armv7, so this must run on an arm64
#   emulator image; an x86_64 emulator cannot load them.
# - Only `node <script>` needs to work. The Termux binary is built with a
#   /data/data/com.termux prefix, so npm and friends may misbehave — the
#   smoke test deliberately avoids them.
#
# Exit code is signal only: the CI job sets continue-on-error and never
# gates publishing.
set -u

WORK=/data/local/tmp/nagajs
# Guest CPU arch under test. Defaults to aarch64 for manual runs; CI sets
# TERMUX_ARCH=x86_64 because the emulator has no Linux-ARM64 host build, so
# only the x86_64 target can be executed (arm64/armv7 stay build-verified).
TERMUX_ARCH="${TERMUX_ARCH:-aarch64}"
# Directory holding the downloaded *.node artifact under test.
BINDING_DIR="${BINDING_DIR:-android-binding}"
TERMUX_MAIN="https://packages.termux.dev/apt/termux-main/dists/stable/main/binary-$TERMUX_ARCH"
# The emulator action runs this script from $GITHUB_WORKSPACE; pin absolute
# paths before moving to the stage directory.
REPO="${GITHUB_WORKSPACE:-$(pwd)}"
STAGE="${RUNNER_TEMP:-/tmp}/nagajs-android"
mkdir -p "$STAGE"
cd "$STAGE"

echo "==> waiting for emulator boot"
adb wait-for-device
for _ in $(seq 1 60); do
  if [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; then
    echo "emulator booted"
    break
  fi
  sleep 10
done
[ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ] \
  || { echo "ANDROID-TEST-FAIL: emulator never finished booting"; exit 1; }
adb shell "mkdir -p $WORK/bin $WORK/lib $WORK/app"

echo "==> resolving Termux nodejs closure"
curl -fsSL "$TERMUX_MAIN/Packages" -o Packages || { echo "ANDROID-TEST-FAIL: cannot fetch Termux package index"; exit 1; }
# Full direct-dependency closure (not just lib*): node links c-ares, openssl,
# zlib, icu and sqlite directly. Tokens are whole words — substring matching
# would invent a bogus `lib` package out of `zlib`.
deps="$(awk '/^Package: nodejs$/{found=1} found{print} found && /^$/{exit}' Packages \
  | grep '^Depends:' | cut -d: -f2- | tr ',' '\n' | sed -E 's/\(.*//' | awk '{print $1}' | sort -u)"
wanted="nodejs $deps"
echo "packages:$wanted"
for pkg in $wanted; do
  file="$(awk -v p="Package: $pkg" '$0 == p {found=1} found{print} found && /^$/{exit}' Packages \
    | grep '^Filename:' | head -1 | awk '{print $2}')"
  [ -n "$file" ] || { echo "ANDROID-TEST-FAIL: no Filename for Termux package $pkg"; exit 1; }
  curl -fsSL "https://packages.termux.dev/apt/termux-main/$file" -o "$(basename "$file")" \
    || { echo "ANDROID-TEST-FAIL: cannot download $file"; exit 1; }
done

echo "==> extracting node and shared libraries"
mkdir -p termux
for deb in *.deb; do
  data="$(ar t "$deb" | grep '^data.tar' | head -1)"
  [ -n "$data" ] || { echo "ANDROID-TEST-FAIL: no data archive in $deb"; exit 1; }
  ar x "$deb" "$data" || { echo "ANDROID-TEST-FAIL: cannot unpack $deb"; exit 1; }
  tar -xf "$data" -C termux || { echo "ANDROID-TEST-FAIL: cannot extract $data"; exit 1; }
done
# Locate by suffix: .debs nest payloads under ./data/data/com.termux/files/,
# and that prefix is an upstream detail we refuse to hardcode.
NODE_BIN="$(find termux -path '*/usr/bin/node' | head -1)"
[ -n "$NODE_BIN" ] || { echo "ANDROID-TEST-FAIL: no node binary in Termux nodejs package"; exit 1; }
mapfile -t SOS < <(find termux -path '*/usr/lib/*.so*' -not -name '*.a')
[ "${#SOS[@]}" -gt 0 ] || { echo "ANDROID-TEST-FAIL: no shared libraries extracted"; exit 1; }
echo "libs (${#SOS[@]}):"
printf '%s\n' "${SOS[@]}"

echo "==> pushing binding and runtime to device"
adb push "$NODE_BIN" "$WORK/bin/node" || exit 1
for so in "${SOS[@]}"; do adb push "$so" "$WORK/lib/$(basename "$so")" || exit 1; done
adb push "$REPO/index.js" "$WORK/app/index.js" || exit 1
adb push "$REPO/$BINDING_DIR"/*.node "$WORK/app/" || exit 1
adb push "$REPO/simple-test.js" "$WORK/app/simple-test.js" || exit 1
adb push "$REPO/package.json" "$WORK/app/package.json" || exit 1

echo "==> node version on device"
adb shell "LD_LIBRARY_PATH=$WORK/lib $WORK/bin/node --version" || { echo "ANDROID-TEST-FAIL: node does not run"; exit 1; }

echo "==> smoke test on device"
OUT="$(adb shell "cd $WORK/app && LD_LIBRARY_PATH=$WORK/lib $WORK/bin/node simple-test.js" 2>&1)"
echo "$OUT"
echo "$OUT" | grep -q "Simple test passed" || { echo "ANDROID-TEST-FAIL: smoke test output mismatch"; exit 1; }
echo "ANDROID-TEST-PASS"
