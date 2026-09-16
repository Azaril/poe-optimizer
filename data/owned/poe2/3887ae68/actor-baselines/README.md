# Finite actor baseline acquisition

This catalog records all 649 final profiles from the pinned Minions/Spectres data
constructors and the 40 summon-level plus 100 allied/100 hostile damage rows from
Misc. Ordered child skills, absent facts, zero attack times, false flags and empty
lists remain distinct. Scalar life/defence facts and modifier/flag descriptions are
retained with explicit unconverted coverage.

Reproduce using the optional offline Rust adapter:

```powershell
cargo run --release --features pob -- export-owned-actor-baselines `
  --source-root vendor/path-of-building-poe2 `
  --output runs/actor-baseline-export
```

The command authenticates the pinned checkout and independently rereads the four
relevant files. Only three finite data constructors run in a bounded empty Lua
environment; the Data module's merge/limit behavior is separately pinned and
projected. It neither constructs a build nor runs the UI/evaluator. The new output
directory contains `catalog.json` and `evidence.json`; it must not already exist.
Evidence fingerprints normalize source line endings for cross-platform reproduction.

The next native consumer is an owned generated-actor baseline compiler with an
explicit actor-slot policy and an exact creating-skill level projection. Source
module/profile names are offline mapping keys, never runtime dispatch. A creating
skill's semantics choose the allied/hostile curve. A zero raw attack time is not a
spell cast time. The two SandDjinn occurrences in original01 must retain independent
providers and levels. This acquisition does not close any actor, action or build
coverage and does not promote an existing Unmapped declaration.

Rust tests compare all scalar/child facts and every table cell with an independent
literal source inventory, exercise constructor/resource failures, and require exact
catalog content against privately authenticated acquisition. Native packages do
not acquire a PoB or Lua dependency from these DTOs.
