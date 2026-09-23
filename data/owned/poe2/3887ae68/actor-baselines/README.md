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

The native `compile-owned-actor-baselines` command converts selected finite facts to
owned rule programs using an explicit actor-slot policy and creating-skill level
projection. `native-extension.json` first allocates the seven reviewed output stats;
`native-policy.json` selects the existing Skeletal Sniper actor slot, six scalar facts
and an explicitly allied level-damage curve. `native-bindings.json` names these
output identities for downstream recipes. The parent skill already produces this
actor's level, so the policy does not install a duplicate level writer. Source
module/profile names are offline mapping keys, never runtime dispatch. A creating
skill's semantics choose the allied/hostile curve. A zero raw attack time is not a
spell cast time. The two SandDjinn occurrences in original01 must retain independent
providers and levels. This acquisition does not close any actor, action or build
coverage and does not promote an existing Unmapped declaration.

Rust tests compare all scalar/child facts and every table cell with an independent
literal source inventory, exercise constructor/resource failures, and require exact
catalog content against privately authenticated acquisition. Native packages do
not acquire a PoB or Lua dependency from these DTOs.

The reusable compiler supports multiple exact actor slots, explicit scalar absence
policies, ally/hostile curves and optional parameter-to-level table projection. It
checks units, scopes, writer conflicts and immutable prior programs. No conversion
closes an existing Partial owner. Data-only catalog changes alter new compilation;
the output package contains ordinary owned rules/tables, not catalog lookup code.

Rust native-plan tests bind two occurrences of the same creating gem with equal gem
levels but different explicit creating parameters. Their generated actors receive
independent levels, survive scratch reuse, and evaluate on separate threads without
Lua. That synthetic fixture certifies only its toy world; it cannot certify the
original builds or the remaining actor modifiers and action routes.

Native publication after the local-modifier successor:

```powershell
poe-optimizer extend-owned-recipe PRIOR --extension data/owned/poe2/3887ae68/actor-baselines/native-extension.json --output STAGED
poe-optimizer compile-owned-actor-baselines STAGED --catalog data/owned/poe2/3887ae68/actor-baselines/catalog.json --policy data/owned/poe2/3887ae68/actor-baselines/native-policy.json --output NEW
```

`PRIOR`, `STAGED` and `NEW` are explicit caller paths; both output directories must be
new. This authored policy binds the append ledger after the local modifier extension.
The reusable compiler accepts other independently validated catalogs/policies; the
runtime has no fallback to these example paths.
