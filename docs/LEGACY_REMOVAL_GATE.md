# Legacy removal gate (Task 6.6)

Do **not** remove `_legacy_comrak` or `_rcdom_compat` until every item below is green.

## `_legacy_comrak` removal checklist

- [x] `cargo test` with default features passes
- [x] `tests/static_parity.rs`: all fixtures pass without comrak fallback
- [x] `tests/static_parity.rs::wikilink_fixture_reports_shadow_mismatch_without_fallback` — **accepted delta**: pulldown wikilink HTML ≠ comrak; pulldown is canonical (see [Accepted deltas](#accepted-deltas))
- [x] `Document::diagnostics()` / `parse_backend()` reports `ParseBackend::Pulldown` for application preview samples (`tests/downstream_static.rs`, `examples/static_export.rs`)
- [ ] `rg "comrak|_legacy_comrak" src Cargo.toml` returns no production references after removal
- [ ] Stabilization window: 2 weeks with shadow compare enabled and zero **unexpected** mismatches in CI

## `_rcdom_compat` / `markup5ever_rcdom` removal checklist

- [x] `tests/html_fragment_parity.rs` passes for all supported HTML fixtures (requires `--features _rcdom_compat`)
- [x] `tests/static_parity.rs` raw HTML fixtures pass with TreeSink-only path
- [x] Iced renderer uses `HtmlFragment` traversal without `RcDom` in production (`BlockRenderCache` stores `HtmlFragment`; `DomRef` walks fragment arena)
- [x] `_iced_backend` no longer enables `_rcdom_compat` / `markup5ever_rcdom` (parity tests still use `_rcdom_compat` explicitly)
- [x] `rg "markup5ever_rcdom|RcDom" src` returns only `_rcdom_compat` module and `#[cfg(feature = "_rcdom_compat")]` tests (`src/html/rcdom_compat.rs`, `src/html/treesink.rs` test)
- [x] Table/details/image fixtures pass for static and streaming paths (`static_parity`, `stream_parity`, `iced_regression`)

## Commands

```bash
./scripts/verify-features.sh
./scripts/verify-all-features.sh   # optional full matrix
cargo test
cargo test --no-default-features --features no_iced,static,stream
cargo test --features _rcdom_compat --test html_fragment_parity
```

## Accepted deltas

Document any intentional pulldown vs comrak or TreeSink vs RcDom differences here before flipping defaults:

| Fixture | Delta | Reason | Policy |
|---------|-------|--------|--------|
| `gfm_wikilink.md` | pulldown wikilink HTML ≠ comrak | pulldown-cmark 0.13 wikilink extension | **Pulldown is canonical.** Shadow compare may report `shadow_mismatch`; fallback is not used under default `ShadowCompare` policy. |

## Wikilink policy (Task 8.1 prerequisite)

1. Default parse path uses pulldown output even when comrak differs.
2. `LegacyFallbackPolicy::ShadowCompare` (default) records mismatch without replacing blocks.
3. `PreferLegacyUntilParity` may still select comrak for migration debugging only.
4. After comrak removal, wikilinks render exclusively via pulldown — no shadow compare.

## Remaining blockers for Task 8.1

- Stabilization window (2 weeks CI green with shadow compare, no new unexpected mismatches)
- Delete comrak dependency and `_legacy_comrak` feature after window closes
