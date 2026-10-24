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

Build the maintained KDL/DSL release using the
[checked authoring predecessor](../authoring-genesis.md). That release already
contains these raw-profile rules. The catalog and conversion policy here are
pinned acquisition and regression evidence, not a second authoring workflow.

Three Rust laws in the maintained Authoring release suite cover all 337 profiles,
all 1,756 template bindings, the 14 fields and their units/defaults/omissions,
exact effects and Partial owners. Each forward/reverse pass with fresh/reused
scratch checks 1,762 source values, 2,652 zero defaults, 304 reload omissions and
337 reload-presence flags. Authored zeros remain distinct from absent fields.
The optional acquisition CLI suite reproduces both exact catalogs and checks
that a repeated export leaves every output file unchanged.

The old weapon-profile CLI publication test is removed. Its compiler and shared
predecessor remain temporarily because the extension harness still consumes them;
the nine independent Import compiler laws also remain. Their final retirement is
tracked under R1/R4 in the [implementation plan](../../../../../docs/implementation.md).
These component checks do not establish final damage or complete-build parity.
