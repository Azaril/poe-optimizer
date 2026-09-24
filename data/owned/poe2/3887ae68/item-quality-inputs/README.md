# Authored ordinary quality for defensive equipment

This data-only extension admits explicit standard quality for the 1,240 reviewed
item templates with a present defensive profile. It reuses quality definition `0006`,
percentage-point unit `0002`, EquipmentUse statistic `2427`, and the existing
`declared-standard-item-quality` program. IDs use the prefix `def.000000000000`
in namespace `poe2/owned-mechanics-v1`. No new registry entry, rule operation,
receiver, source interpreter or runtime dependency is introduced.

The optional Rust reference test independently verifies the full constructed catalog:
all 1,240 selected bases have source quality 20, an armour table and no weapon table.
Six armour tables are empty. The scope includes 347 body armours, 191 boots, 51 focuses,
201 gloves, 257 helmets and 193 shields. The other 516 bases are outside this extension;
this does not assert that they cannot have quality. The native runtime consumes the
injected declarations, without inspecting source type names or profiles.

Each template gains only the existing standard-quality member in its Partial
allowed-kinds declaration and the exact existing authored-quality read program.
Presence remains Optional and every prior schema/rule closure remains unchanged.
The shared item-header grammar and source-admission policy are unchanged apart from
checked schema/digest rebinding during publication.

An explicit unsigned integer header, including zero, supplies the original amount.
Missing, malformed, duplicate, out-of-range and unsupported-template inputs remain
unresolved. Neither the source's base quality 20 nor its UI defaults authorize an
import fallback. The raw authored amount is not the effective value after crafted
quality, alternate-quality effects or repeated editing; those remain separate
calculation stages.

The original five builds gain 32 known quality values across all stored item sets
(5/9/4/4/10), including 21 selected physical defensive items (5/4/4/4/4).
Existing modifiers, gem inputs, display observations, source attribution and all
110 query rows remain covered by the publication integration test. Item membership,
modifier ordering and whole-build evaluation remain incomplete.

Publish after the [defensive modifier inputs](../item-defence-inputs/README.md):

```text
poe-optimizer extend-owned-recipe PRIOR --extension data/owned/poe2/3887ae68/item-quality-inputs/extension.json --output NEW
```

The native command requires a new destination and preserves the predecessor.
`bindings.json` records the reviewed template set and reused identities;
`extension.json` is the typed data consumed by publication. The registry remains
at 11,228 entries. Reproduction and malformed-input checks are Rust tests; existing
Python tooling and tests remain unchanged under [T1](../../../../../docs/architecture-migration.md#t1-rust-tooling-and-test-consolidation).
