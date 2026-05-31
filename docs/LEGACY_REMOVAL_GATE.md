# Legacy removal gate (Task 6.6 / 8.1)

## `_legacy_comrak` — removed (Task 8.1)

Comrak and `_legacy_comrak` were removed after automated parity gates passed. The stabilization window was replaced by CI/local test matrices (`scripts/verify-all-features.sh`, `cargo test`).

- [x] `cargo test` with default features passes
- [x] `tests/static_parity.rs`: all fixtures pass on pulldown-only path
- [x] `tests/static_parity.rs::gfm_wikilink_exports_via_pulldown` — pulldown is canonical for wikilinks
- [x] `Document::diagnostics()` / `parse_backend()` reports `ParseBackend::Pulldown`
- [x] `rg "comrak|_legacy_comrak" src Cargo.toml` returns no production references

## `_rcdom_compat` / `markup5ever_rcdom` removal checklist

- [x] `tests/html_fragment_parity.rs` passes for all supported HTML fixtures (requires `--features _rcdom_compat`)
- [x] `tests/static_parity.rs` raw HTML fixtures pass with TreeSink-only path
- [x] Iced renderer uses `HtmlFragment` traversal without `RcDom` in production
- [x] `_iced_backend` no longer enables `_rcdom_compat` / `markup5ever_rcdom`
- [x] `rg "markup5ever_rcdom|RcDom" src` returns only `_rcdom_compat` module and test-only cfg
- [x] Table/details/image fixtures pass for static and streaming paths

## Commands

```bash
./scripts/verify-features.sh       # CI: feature-matrix job
./scripts/verify-all-features.sh   # CI: feature-matrix-full job (clippy -D warnings)
./scripts/verify-egui-harness.sh   # CI: egui-harness job
cargo test
cargo test --no-default-features --features no_iced,static,stream
cargo test --features _rcdom_compat --test html_fragment_parity
```

## Accepted deltas (historical)

| Fixture | Delta | Reason | Policy |
|---------|-------|--------|--------|
| `gfm_wikilink.md` | pulldown wikilink HTML ≠ former comrak output | pulldown-cmark 0.13 wikilink extension | **Pulldown is canonical.** Comrak removed; no shadow compare. |
