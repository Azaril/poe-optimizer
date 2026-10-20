# Ordinary physical hit-DPS component

This current-format packet appends one typed Action program and four definitions
to the reviewed `f72d73…965d8` release. It reads the published normal and critical
armour-adjusted physical averages, outgoing critical chance, **post-block** hit
chance, and ordinary attack rate. Its outputs are internal weighted physical
damage (`33b8`), post-block physical damage (`33b9`), and ordinary physical damage
per second (`33ba`, unit `33bb`).

The formula preserves the source's operation order:

```
weighted = normal * (1 - critical_percent / 100)
         + critical * critical_percent / 100
post_block = weighted * hit_percent / 100
ordinary_physical_dps = post_block * ordinary_rate * 1 * 1
```

The final factors are justified only within the separately reviewed ordinary
source domain. Explicit unit bases convert the typed Damage and Rate values to
DamagePerTime using the existing Ratio/Scale operations. Those conversions
divide or multiply by exactly one; they do not change the source's percentage
operation order or introduce a new arithmetic operation.

`source-domain.json` reviews the complete retained input roots and selected
recipes, then separately accounts for late factories and transport using full
pinned source bodies. It reuses the existing eight-input timing proof and
extends the vocabulary to DPS, quantity, repeat, hazard, grenade, seal, ruthless,
trigger and hit-time branches. A zero modifier, inactive supplier, false branch
field or explicit `dpsMultiplier = 0` is still a nonordinary input. In particular,
`repeatMode = FINAL` is unsupported even when repeat count is one.

The constructor capture is unfiltered by modifier name, but it is **not a final
minion modifier-store capture**. Its acquisition code and limitations are
retained. Complete late source branches and exact recipient/callback ownership
are necessary in addition to the input census. The source comparison rows are
diagnostics, never native operands or neutral-value authority.

Production owners, source inventories and input routes remain Partial. This
packet adds no contributor query, metric route, evaluation bundle, or complete
build claim. The saved unavailable `selected_average_hit` remains unavailable:
the internal physical weighted mean is a different quantity. Final
`selected_hit_dps` also needs all damage types and the remaining input/provider
coverage. Saved use counts and FullDPS aggregation are separate from mechanical
quantity and are unchanged.

Ordinary Rust tests authenticate the retained sources and exact predecessor
dependencies without Lua, a vendor checkout, or ignored run files. A finite
fixture composes the actual armour, critical, hit-chance and Offering rate
producers; its explicit completions are test-only. Controls cover arithmetic
order, zero chance/rate, unavailable/unsupported inputs, late writes, independent
Actions, fresh/reused scratch and Rayon workers. Shared source archives and
their pinned upstream license are reused without duplication.

Validation on 2026-10-20 passes 27 ordinary tests in each of the all-features
and native-only profiles, strict Clippy in both profiles, and the separately
invoked package publication test. The latter rebuilds byte-identically and
preserves all five unchanged imports, all 110 saved queries and every remaining
input obligation. Its local package is
`runs/owned-physical-hit-dps-publication-02/package`, input
`2d307fd317100b5282026a38d91c281a937eb850e275664d8c5ddfc9ee2c6560`.
No complete build or final all-type DPS binding is claimed.
