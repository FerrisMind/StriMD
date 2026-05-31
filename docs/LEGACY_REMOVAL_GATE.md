# Legacy removal gate (Task 6.6)

Do **not** remove `_legacy_comrak` or `_rcdom_compat` until every item below is green.

## `_legacy_comrak` removal checklist

- [ ] `cargo test` with default features passes
- [ ] `tests/static_parity.rs`: all fixtures pass without comrak fallback
- [ ] `tests/static_parity.rs::wikilink_fixture_reports_shadow_mismatch_without_fallback` shows **no** `shadow_mismatch` OR wikilink parity is accepted and documented
- [ ] `Document::diagnostics()` reports `ParseBackend::Pulldown` for application preview samples
- [ ] `rg "comrak|_legacy_comrak" src Cargo.toml` returns no production references after removal
- [ ] Stabilization window: 2 weeks with shadow compare enabled and zero unexpected mismatches in CI

## `_rcdom_compat` / `markup5ever_rcdom` removal checklist

- [ ] `tests/html_fragment_parity.rs` passes for all supported HTML fixtures
- [ ] `tests/static_parity.rs` raw HTML fixtures pass with TreeSink-only path
- [ ] Iced renderer uses `HtmlFragment` traversal without `RcDom` in production (`tests/iced_regression.rs` green)
- [ ] `rg "markup5ever_rcdom|RcDom" src Cargo.toml` returns only test remnants or none
- [ ] Table/details/image fixtures pass for static and streaming paths

## Commands

```bash
./scripts/verify-features.sh
cargo test
cargo test --no-default-features --features no_iced,static,stream
```

## Accepted deltas

Document any intentional pulldown vs comrak or TreeSink vs RcDom differences here before flipping defaults:

| Fixture | Delta | Reason |
|---------|-------|--------|
| `gfm_wikilink.md` | pulldown wikilink HTML ≠ comrak | migration tracking only |
| (add rows as discovered) | | |
