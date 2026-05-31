#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== default iced build =="
cargo check

echo "== stream =="
cargo check --features stream

echo "== static =="
cargo check --features static

echo "== static + stream =="
cargo check --features static,stream

echo "== headless static =="
cargo check --no-default-features --features no_iced,static

echo "== headless stream =="
cargo check --no-default-features --features no_iced,stream

echo "== headless static + stream =="
cargo check --no-default-features --features no_iced,static,stream

echo "== legacy comrak migration =="
cargo check --features static,stream,_legacy_comrak

echo "All feature-matrix checks passed."
