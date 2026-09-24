# Complete unconditional passive defence and recovery inputs

This injected policy converts 251 complete passive lists through the existing generic
`compile-owned-passive-views` command. The full catalog contains 44 reviewed families,
with 353 display lines producing 413 contributions. Selection is independent of any input
build. The policy includes 239 ordinary and 12 ascendancy nodes, retaining separate pools.

The definition-only extension appends three units and ten Actor statistics to registry
10732, ending at 10745. It adds no rules, receivers, slots, runtime parser or Core operations.
Energy Shield uses resource points, armour/evasion use a distinct rating unit, and mana
regeneration uses a distinct rate unit. Recharge-delay base contributions use seconds;
increased/faster contributions use percentage points. Maximum Mana reuses mana points.
Attribute effects retain the existing Strength/Dexterity/Intelligence channels.

| Key suffix | Domain contribution channel | BASE unit |
| --- | --- | --- |
| `29f0` | Energy Shield | Energy Shield points (`29ed`) |
| `29f1` | Evasion | Defence rating (`29ee`) |
| `29f2` | Armour | Defence rating (`29ee`) |
| `29f3` | Mana regeneration | Mana recovery per second (`29ef`) |
| `29f4` | Energy Shield recharge delay | Seconds (`0005`) |
| `29f5` | Evasion gained as deflection | Percentage points (`0002`) |
| `29f6` | Armour applying to fire damage | Percentage points (`0002`) |
| `29f7` | Armour applying to cold damage | Percentage points (`0002`) |
| `29f8` | Armour applying to lightning damage | Percentage points (`0002`) |
| `29f9` | Maximum Mana | Mana points (`0003`) |

All keys have prefix `def.000000000000` in `poe2/owned-mechanics-v1`. BASE effects use Add;
INC effects use Increase with percentage-point values regardless of the BASE unit. A
slower recharge start or reduced maximum Mana contributes a negative increase. These are
inputs to later calculations, not already-computed final capacities, rates or delays.

Complete lists include Pure Energy, Insightfulness and Enhanced Reflexes. Each contributes
its attribute and defence/recovery effects together. Armour applying to elemental damage
preserves three separate source effects; it is not replaced by a generic elemental bucket.
Increased maximum Energy Shield has an exact source Global scope marker, corresponding to
these Player-global providers. Arbitrary tags or conditional effects are not admitted.

`bindings.json` records the catalog/source pin, typed channel meanings, complete reviewed
lists and source-to-owned identities. Optional Rust reference tests execute authenticated
complete PassiveTree processing and ModDB queries, independently checking the policy's
values, signs, units, scopes and expanded effects. Raw metadata is reviewed as well as
text; display-list matching alone cannot certify socket, grant or special node mechanics.
The checked publisher, rather than the evidence manifest, establishes package authority.

Publish with an explicit validated predecessor and a caller-owned UTF-8 file containing `[]`:

```text
poe-optimizer extend-owned-recipe PRIOR --extension data/owned/poe2/3887ae68/passive-defence-inputs/extension.json --output CHANNELS
poe-optimizer compile-owned-passive-views CHANNELS --catalog data/owned/poe2/3887ae68/tree/tree-catalog.json --policy data/owned/poe2/3887ae68/passive-defence-inputs/policy.json --statistics EMPTY_ARRAY_JSON --output NEW
```

Both destinations must be new. The predecessor for this checkpoint is the passive-attribute
successor; no example or machine-local path is built into the compiler. Source text stops
at offline conversion. Native evaluation uses ordinary typed literal contribution programs.
Compact publication retains existing resource limits, endpoint validation, immutable prior
programs and carried import/query policies.

The active presets of originals01–05 gain intrinsic provider coverage for respectively
21, 11, 21, 5 and 0 selected nodes. Saved presets and all source inputs remain intact.
The shared CLI regression checks every original draft and item attribution, accounting only
for validated package bindings and independent import lineages. All 110 queries, 140 Shared
skill scopes, 12 Complete/466 Pending gem parameter collections, 64 modifier occurrences and
53 display observations remain present.

Allocation access remains Pending. Equipment-derived defences, conditional providers,
conversions, final receivers, mitigation and complete contributor membership remain separate
work. No final defence value or complete original-build evaluation is established here.
