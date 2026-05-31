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

echo "== unit + integration tests (default) =="
cargo test

echo "== streaming parity tests =="
cargo test --features stream --test stream_parity

echo "== downstream static integration (Task 7.1) =="
cargo test --no-default-features --features no_iced,static --test downstream_static

echo "== downstream stream integration (Task 7.2) =="
cargo test --no-default-features --features no_iced,stream --test downstream_stream

echo "== html preprocess (Task 2.5) =="
cargo test --features _html_preprocess html::preprocess

echo "== headless tests (lib + integration; iced doctests excluded) =="
cargo test --no-default-features --features no_iced,static,stream --lib --tests

echo "All feature-matrix checks passed."
