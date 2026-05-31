#!/usr/bin/env bash
# Task 4.5 / 7.4 — egui harness checks inside frostmark (no Nova).
set -euo pipefail
cd "$(dirname "$0")/.."

FEATURES="no_iced,static,stream"

echo "== egui harness integration tests =="
cargo test --no-default-features --features "$FEATURES" --test egui_harness

echo "== egui_table_harness --check =="
cargo run --example egui_table_harness --no-default-features --features "$FEATURES" -- --check

echo "== egui_pipeline_harness --check =="
cargo run --example egui_pipeline_harness --no-default-features --features "$FEATURES" -- --check

echo "== egui examples compile =="
cargo check --example egui_table_harness --no-default-features --features "$FEATURES"
cargo check --example egui_pipeline_harness --no-default-features --features "$FEATURES"

echo "All egui harness checks passed."
