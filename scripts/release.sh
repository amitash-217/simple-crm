#!/usr/bin/env bash
# Builds the frontend, then the release binary with the frontend embedded.
# Usage: scripts/release.sh [extra cargo build args, e.g. --target x86_64-unknown-linux-musl]
set -euo pipefail
cd "$(dirname "$0")/.."

(cd web && npm ci && npm run build)
cargo build --release "$@"

# With --target, cargo writes to target/<triple>/release.
target=""
while [ $# -gt 0 ]; do
  case "$1" in
    --target) target="$2"; shift 2 ;;
    --target=*) target="${1#--target=}"; shift ;;
    *) shift ;;
  esac
done
bin="target/${target:+$target/}release/crm"

echo
echo "Binary:  $bin"
echo "Size:    $(du -h "$bin" | cut -f1)"
echo "SHA-256: $(shasum -a 256 "$bin" | cut -d' ' -f1)"
