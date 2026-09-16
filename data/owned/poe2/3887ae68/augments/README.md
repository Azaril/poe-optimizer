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

The next native consumer is explicit rune reconstruction, not direct evaluation
of these descriptions. Preserve each socketed item's identity and its container
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
