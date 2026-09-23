# Finite socketed augment acquisition

This optional offline catalog preserves 287 names, 594 category selectors and
1,188 ordered descriptions (629 normal, 559 Bonded) across Rune, SoulCore, Idol,
AbyssalEye and CongealedMist. Every selector is explicitly unconverted. Fractional
stat order, absent metadata, empty lanes, socket restrictions, level/limit facts
and trade metadata are retained without inferring gameplay defaults.

Reproduce using the optional Rust adapter:

```powershell
cargo run --release --features pob -- export-owned-augments `
  --source-root vendor/path-of-building-poe2 `
  --output runs/augment-export
```

The adapter verifies the pinned checkout and separately authenticates ModRunes.lua
before executing that one finite constructor in a bounded empty Lua environment.
No source loader, UI, modifier parser or build evaluator runs. `catalog.json` and
`evidence.json` are written only to a new directory. Import can validate bounded
catalog bytes portably, but carried source hashes alone are not authentication.
The adapter's private acquisition result independently compares exact content.

Native prepared-line reconstruction is available through the Import API and
`reconstruct-owned-augments` command, with explicit catalog, policy and request files.
It prepares descriptions for later owned line recipes, without evaluating effects. Preserve each socketed item's identity and its container
relationship; select only the admitted active sockets and categories. Reconstruction
must account for normal/Bonded grouping, duplicate summation and numeric formatting,
saved-line replacement/disabled state, unknown or surplus headers, extra-effect
rounding and Bonded activation. Level requirements inspect every supplied known
augment's selectors, not only its active effect category. Missing-header inference
remains a separate future policy. Do not count saved lines and reconstructed effects
twice, or treat equal stat-order keys as proof of equal modifier semantics.

Rust tests compare all lines/order/metadata against the independent pinned finite
snapshot, including Bonded-only bucklers, fractional AbyssalEye order and numberless
rune text. Authentication, unknown shapes and bounded construction have contrasting
tests. No runtime rune lifecycle or whole-build coverage is claimed here.

Reconstruction retains exact socketed item/use/container identities, normal/Bonded
lanes, explicit category traversal order and provenance for every contributing line.
Grouping uses family/lane and the reviewed source number format for the order key.
The first text controls numeric positions and signs; each merge formats and reparses
before the next merge. Extra incoming numeric components remain diagnostics; missing
components leave the merge unavailable. Equal order is not proof of equal meaning.

All 594 selectors and adversarial numeric merges are covered by Rust tests. Optional
Rust reference tests run only the authenticated original UpdateRunes method with an
explicit no-op parser observer; they do not instantiate the PoB UI or build evaluator.

The request is caller-authored conversion input, not proof that those occurrences or
socket declarations exist in a prepared build. Actual build/schema binding, saved-line
reconciliation and modifier admission are still required. Activation and magnitude
facts are carried as Unapplied; all selector effects remain Unconverted. No item,
allocation, contributor or build coverage gate is relaxed by prepared text.

To prepare caller-supplied selections without PoB:

```powershell
poe-optimizer reconstruct-owned-augments --catalog CATALOG --policy POLICY --request REQUEST --output NEW
```

`NEW/preparation.json` contains the same report printed to stdout. All three inputs
are required and bounded before parsing. `--max-output-bytes` can lower the conservative
retained-representation limit; existing output is never overwritten. Catalog updates
require an explicit policy digest update, not an implicit runtime rebind.
