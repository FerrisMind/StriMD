# Crate split spike (Task 8.4)

**Decision:** Stay on a **single crate** for the 0.3 release. Revisit split only if external consumers repeatedly depend on migration features or Cargo feature confusion becomes a support burden.

## Goal

Requirement R1.7 asks whether frostmark should split into `frostmark-core` + `frostmark-iced` so that only `no_iced`, `static`, and `stream` appear as visible features.

## Options evaluated

### A. Single crate (current)

```
frostmark
├── public: no_iced, static, stream
└── internal: _iced_backend, _legacy_comrak, _rcdom_compat, _html_preprocess
```

**Pros**

- One version line, one docs.rs page, simpler path dependency for Nova/frostmark consumers.
- Migration fallbacks are already gated and documented as unsupported.
- `compile_error!` guards prevent invalid feature combinations.
- Headless CI uses `default-features = false` cleanly today.

**Cons**

- Cargo `--features` listing shows internal `_`-prefixed flags.
- Default features still pull comrak until Task 8.1 completes.

### B. Split: `frostmark-core` + `frostmark-iced`

```
frostmark-core   → no_iced, static, stream only
frostmark-iced   → re-exports core + MarkWidget/MarkState
frostmark        → thin facade depending on both (optional)
```

**Pros**

- Literally three features on the core crate.
- Stronger boundary for headless-only dependents.

**Cons**

- Breaking publish/co-versioning overhead (two crates, two READMEs, cross-crate doc links).
- Nova and frostmark examples must update dependency paths.
- Migration internals (`_legacy_comrak`) still need a home — likely core until removal anyway.
- No functional gain once 8.1/8.2 cleanup removes comrak/RcDom from defaults.

### C. Split: `frostmark` + `frostmark-iced` only

Core stays as `frostmark` with headless features; iced moves to `frostmark-iced`.

**Pros**

- Headless crate name unchanged.

**Cons**

- GUI users need two dependencies or a facade crate.
- Same migration-feature problem on the core crate.

## Recommendation

**Do not split before 0.3 release.**

1. Complete Task 8.1 (remove `_legacy_comrak`) after the stabilization window.
2. Document the three-feature contract in README, `docs/API.md`, and crate-level docs (Task 8.3).
3. Re-evaluate split if post-release feedback shows consumers enabling `_`-prefixed features despite documentation.

If split becomes necessary later, prefer **Option B** with `frostmark-core` as the semver anchor and `frostmark-iced` as an optional GUI layer.

## Verification

This spike satisfies Task 8.4 verification: tradeoffs documented with a clear recommendation.
