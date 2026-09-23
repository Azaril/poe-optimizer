# Explicit source preamble metadata

This adapter-only successor adds a configured `Unique ID: ` recipe with an opaque
value and a metadata-only emission. Source policy v5 names the reviewed recipe in
`pob_exported_single_text_preamble_v1.metadata_rules`; it does not accept every
Header recipe as harmless metadata. The compiler requires unique, existing Header
rules with nonempty, exclusively Metadata emissions. No owned definition, rule
program, receiver, query, item identity or game value is added.

Publish after the 10,537-entry constructed-layout successor:

```powershell
poe-optimizer extend-owned-recipe PRIOR --extension data/owned/poe2/3887ae68/item-metadata-inputs/extension.json --items data/owned/poe2/3887ae68/item-metadata-inputs/items.json --item-source data/owned/poe2/3887ae68/item-metadata-inputs/item-source.json --output NEW
```

The native adapter validates the exact schema/line-policy bindings. The declared
preamble is valid after the base and before modifier insertion, occupies zero
modifier-index positions, and retains raw source evidence. Duplicate source ID
lines remain distinct evidence; they do not deduplicate inventory instances.
Unknown, malformed, ambiguous and misplaced headers keep their existing gates.
The v3/v4 wire shapes, digest domains and numerical meanings remain unchanged;
v5 keeps the v4 flag semantics even when its flag-binding list is empty.

Pinned `Classes/Item.lua:781-785` immediately continues after assigning Unique ID.
Modifier tags handled after line 966 therefore do not reinterpret ordinary
`{range:...}`, `{tags:...}` or `{rune}` text inside its value. Selection controls
are different: the earlier all-line scan at 600-641 recognizes closed variant,
version and group tags anywhere. Metadata with those controls or the `Foil Unique`
rarity marker is unsupported. Square/angle markup is also unsupported because
`escapeGGGString` can expose concealed controls before that scan. No source text
rewriter or editor lifecycle is added to the native model.

Zero modifier membership does not mean the source field has no later use.
Unique ID affects external-item reconciliation and later quality normalization.
The supported fresh single-text XML path does not request high-quality handling;
its absent-quality branch sets zero independently of Unique ID. Existing reviewed
quality/catalyst default rules remain valid under that boundary. A future importer
of later source state must convert its effective quality explicitly, without
adding PoB identity or UI fields to Core.

The supplied five-build corpus contains 97 such headers across 116 item records.
Gameplay fields such as Spirit, Charm Slots, defence displays, corruption and rune
configuration are separate work. Neither this recipe nor zero-member attribution
establishes complete item rolls, modifier order, allocations, actor/action routing
or whole-build parity. All 110 query rows remain part of validation.

The compiler, authoring path and tests are Rust. Existing Python utilities/tests
remain unchanged under the T1 migration plan.
