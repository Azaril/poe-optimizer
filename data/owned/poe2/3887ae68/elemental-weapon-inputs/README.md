# Elemental and chaos weapon inputs

This finite owned extension adds nine EquipmentUse statistics, eight shared
receivers and eight effective-value contribution programs. It applies after the
10528-entry local-scaling successor:

```powershell
poe-optimizer extend-owned-recipe PRIOR --extension data/owned/poe2/3887ae68/elemental-weapon-inputs/extension.json --output NEW
```

The eight assembled endpoints retain all raw profile channels unchanged:

| Family | Raw minimum / maximum | Assembled minimum / maximum | Canonical flat owner |
| --- | --- | --- | --- |
| Cold | 241b / 241a | 2921 / 2922 | 2600 |
| Fire | 241e / 241d | 2923 / 2924 | 261d |
| Lightning | 2420 / 241f | 2925 / 2926 | 263a |
| Chaos | 2419 / 2418 | 2927 / 2928 | 2657 |

All addresses abbreviate `def.000000000000` in `poe2/owned-mechanics-v1`.
The new shared `2929` channel carries local-elemental percentage contributions.
Its Add reduction remains separate from each elemental endpoint's type-specific
Increase reduction. Damage uses exact unit `1d3a`, percentages use `0002`, and
scale factors use `0001`. Each receiver applies to the same337 explicit weapon
templates as the existing physical receivers.

For each Cold, Fire or Lightning endpoint, the owned expressions compute:

```
with_flat = raw + sum(endpoint Add)
local_increase = sum(endpoint Increase) + sum(shared2929 Add)
scaled = with_flat * (1 + local_increase / 100)
assembled = floor(scaled + 0.5)
```

The order matches the reviewed local arithmetic in pinned `Classes/Item.lua`
2474-2489 and `Modules/Common.lua`722-728. The two increase families must remain
separate reductions. Chaos computes only `raw + sum(endpoint Add)`, retaining
fractions. None of these eight receivers reads ordinary quality, physical
increase, global elemental damage, or damage with attacks.

The four canonical Modifier families contribute their existing effective
minimum `253f` and maximum `2540` to their corresponding assembled Add channels.
They never contribute to raw baseline channels or substitute raw/nominal rolls
for missing effective values. Existing Partial owner declarations and their gaps
are preserved. No local-type or shared-elemental percentage producer is claimed
by this extension; these input channels require separate reviewed producers or
proof that the incoming contributor set is empty. A complete empty reduction has
its explicit zero identity; missing facts or incomplete membership remain unknown.

These are numerical endpoints before positive-pair field emission and ordered
WeaponData overrides. Zero or negative endpoint values remain visible here;
this stage does not publish final weapon fields, DPS, source eligibility or action
routes. Source modifier-locality/hand rules, contribution completeness, scalar
lifecycle and source admission retain their existing limits. Deterministic native
sum order is not a claim of bitwise source-list-order parity for cancellation
near rounding boundaries.

The Rust CLI helper validates publication, immutable predecessor data, exact
channel/owner/receiver bindings, canonical effective-value contributions, additive
percentage grouping, signed and fractional boundary cases, missing inputs and
parallel independent scratch. These are real compiled owned component programs
with explicit caller facts; they do not prove complete occurrence evaluation.
All 110 original query rows and 0/5 complete-build status are preserved. Native
evaluation gains no Lua runtime, source parser or new Core operation.

The optional PoB package also runs a bounded Rust oracle test against authenticated
original local-damage source spans. It compares these eight production receiver
programs across 208 endpoint cases, with explicit raw fields and local rows. A
read-only observation exposes intermediates before the unchanged positive-pair
filter. Source expressions remain verbatim; no PoB UI or parser lifecycle is loaded.
This test does not establish contributor order, final overrides or whole-build parity.
It runs normally in the existing PoB-package CI job:

```powershell
cargo test -p poe-optimizer-pob --test owned_elemental_weapons --locked
```
