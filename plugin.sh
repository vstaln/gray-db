#!/bin/sh
here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sys="$(uname -s)"
case "$sys" in
MINGW* | MSYS* | CYGWIN*) rel="x86_64-pc-windows-msvc/db.exe" ;;
Darwin*)
    case "$(uname -m)" in
    arm64) rel="aarch64-apple-darwin/db" ;;
    *) rel="x86_64-apple-darwin/db" ;;
    esac ;;
*)
    case "$(uname -m)" in
    aarch64 | arm64) rel="aarch64-unknown-linux-musl/db" ;;
    *) rel="x86_64-unknown-linux-musl/db" ;;
    esac ;;
esac
if [ -x "$here/$rel" ]; then exec "$here/$rel" "$@"; fi
for cand in "$here/target/release/db.exe" "$here/target/release/db" "$here/target/debug/db.exe" "$here/target/debug/db"; do
    if [ -x "$cand" ]; then exec "$cand" "$@"; fi
done
echo "db binary not found in $here" >&2
exit 1
