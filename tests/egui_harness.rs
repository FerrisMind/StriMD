//! Automated egui harness checks (Tasks 4.5 and 7.4) — no window, CI-friendly.

#![cfg(all(feature = "no_iced", feature = "static", feature = "stream"))]

#[path = "../examples/shared/harness_checks.rs"]
mod harness_checks;

#[test]
fn task_4_5_static_table_path() {
    harness_checks::check_static_table_path().expect("static table");
}

#[test]
fn task_4_5_stream_table_path() {
    harness_checks::check_stream_table_path().expect("stream table");
}

#[test]
fn task_7_4_unified_pipeline() {
    harness_checks::check_unified_pipeline().expect("unified pipeline");
}

#[test]
fn task_4_5_and_7_4_combined() {
    harness_checks::run_all_checks().expect("all harness checks");
}
