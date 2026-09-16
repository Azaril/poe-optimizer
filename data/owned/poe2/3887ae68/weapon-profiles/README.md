# Raw weapon profiles

These finite inputs cover all 337 constructed weapon tables in the pinned base catalog,
including profiles with only elemental damage. All 14 numeric source fields are accounted
for. The optional Rust exporter authenticates the checkout and runs the bounded data
constructor once, producing this catalog and the byte-identical item-base catalog. The
native compiler receives only finite JSON and an explicit conversion policy.

The policy allocates EquipmentUse statistics 9239–9252 in source-field name order,
Distance unit 9253 (source range units, ten per metre), and reload-field presence
Capability 9254. AttackRateBase uses attacks per second, CritChanceBase uses percentage
points, ReloadTimeBase uses seconds, and damage channels use damage units. These are
raw input channels, not final item/skill outputs. No runtime behavior dispatches on a
weapon, skill, class, monster or example-build name.

Rate, crit and range are required. Missing damage fields become explicit zero baseline
inputs under the reviewed source `or 0` rule; authored zeros remain valid. Reload absence
omits its numeric output and publishes a false presence capability. An absent whole
profile publishes neither a numeric program nor reload presence. Source-profile presence
and skill/equipment eligibility remain distinct. Partial owner coverage is preserved.

Quality, local modifier filtering/order, per-hand effects, rounding, final damage-pair
presence, WeaponData overrides and action compatibility are subsequent owned recipes.
This phase deliberately does not calculate final DPS or mark an original build complete.

Reproduce source acquisition through the optional adapter:

```powershell
cargo run --features pob -- export-owned-weapon-profiles `
  --source-root vendor/path-of-building-poe2 `
  --output runs/weapon-profiles-export
```

Generate the [item-base predecessor](../item-bases/README.md), then run the native host:

```powershell
cargo run --no-default-features -- compile-owned-weapon-profiles runs/owned-item-bases-package `
  --base-catalog data/owned/poe2/3887ae68/item-bases/catalog.json `
  --catalog data/owned/poe2/3887ae68/weapon-profiles/catalog.json `
  --policy data/owned/poe2/3887ae68/weapon-profiles/policy.json `
  --definitions data/owned/poe2/3887ae68/weapon-profiles/definitions.json `
  --output runs/owned-weapon-profiles-package
```

Publication uses compact successor format v2: canonical registry/schema/rules/routing
files are the single recipe representation. Bundle membership is closed: keep README
files, acquisition evidence and run receipts outside the published directory. The checked loader reconstructs and validates
the recipe and manifest; no duplicate `recipe.json` is shipped. Existing v1 bundles remain
readable with unchanged identities. Both prior recipe and remaining v2 transition inputs
are separately bounded at 64 MiB each (at most 128 MiB streamed in total);
published files still have one 64 MiB aggregate limit. All 110 original
queries, registry history, source policies and tree evidence survive the transition.

All new tooling and tests are Rust. Existing Python tooling/tests are unchanged; their
coherent replacement is tracked by architecture migration milestone T1.
