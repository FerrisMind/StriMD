#!/usr/bin/env bash
# Full feature-matrix: check, clippy (-D warnings), and tests per valid combination.
set -euo pipefail
cd "$(dirname "$0")/.."

run_combo() {
  local label="$1"
  shift
  echo ""
  echo "========================================"
  echo "== $label"
  echo "========================================"
  cargo check "$@"
  cargo clippy "$@" --all-targets -- -D warnings
}

run_combo_test() {
  local label="$1"
  shift
  echo ""
  echo "========================================"
  echo "== $label (tests)"
  echo "========================================"
  cargo test "$@" --lib --tests
}

expect_fail() {
  local label="$1"
  shift
  echo ""
  echo "========================================"
  echo "== $label (expect compile_error)"
  echo "========================================"
  if cargo check "$@" 2>&1; then
    echo "ERROR: expected failure but build succeeded for: $*"
    exit 1
  fi
  echo "OK: failed as expected"
}

# --- Default iced (pulldown-only) ---
run_combo "default (iced + pulldown)"
run_combo "default + stream" --features stream
run_combo "default + _html_preprocess" --features _html_preprocess
run_combo "default + stream + _html_preprocess" --features stream,_html_preprocess

# --- Iced headless contract variants ---
run_combo "iced only" --no-default-features --features _iced_backend
run_combo "iced + stream" --no-default-features --features _iced_backend,stream

# --- Headless public contract ---
run_combo "no_iced + static" --no-default-features --features no_iced,static
run_combo "no_iced + stream" --no-default-features --features no_iced,stream
run_combo "no_iced + static + stream" --no-default-features --features no_iced,static,stream
run_combo "no_iced + static + _html_preprocess" \
  --no-default-features --features no_iced,static,_html_preprocess

# --- Headless tests ---
run_combo_test "no_iced + static + stream tests" \
  --no-default-features --features no_iced,static,stream
run_combo_test "no_iced + static tests" \
  --no-default-features --features no_iced,static
run_combo_test "no_iced + stream tests" \
  --no-default-features --features no_iced,stream

echo ""
echo "========================================"
echo "== headless downstream integration tests"
echo "========================================"
cargo test --no-default-features --features no_iced,static --test downstream_static
cargo test --no-default-features --features no_iced,stream --test downstream_stream

# --- Default tests + stream integration ---
echo ""
echo "========================================"
echo "== default tests (lib + tests + examples compile via clippy)"
echo "========================================"
cargo test
cargo test --features stream --test stream_parity

# --- Explicit alias features ---
run_combo "static + stream (no iced flag)" --features static,stream
run_combo "markdown alias (static)" --features markdown
run_combo "iced-windowing alias on default" --features iced-windowing

# --- Invalid combinations (compile_error guards) ---
expect_fail "no features at all" --no-default-features
expect_fail "no_iced with default iced backend" --features no_iced

echo ""
echo "All feature combinations passed."
