# Controlled build mutations

Status: experimental finite-domain adapter, `poe_optimizer_import::controlled_mace` (also re-exported by `pob::mutation`), alongside the unchanged four-fixture `pob::candidate` calibration registry. This phase supplies parameterized weapon/support search inputs. The end state remains joint class, ascendancy, passive, item, active-skill, and support search with configurable requirements and independent locks.

## Supported structural profile

`ControlledMaceCatalog::with_data(snapshot, template_xml, weapons, supports)` retains an immutable selected dataset and checks structure and values, rather than requiring one of four XML hashes. `new(template_xml, weapons, supports)` remains a convenience constructor for the reviewed default. The template is an unallocated Warrior with no ascendancy, tree `0_5`, one active item/skill/configuration set, exactly one manually specified Mace Strike at level 1/quality 0, and zero or one Brutality I at level 1/quality 0. The only equipped item is slot `Weapon 1`, XML item ID `1`. Character level is explicit, 1–100, with automatic leveling disabled. No extra allocated passives, equipment, runes, modifiers, active skills, item-granted skills, minions, alternate sets, or unknown mechanics are accepted.

The weapon list contains 1–64 distinct exact item payloads with distinct user labels. Payloads have exactly these five lines:

```text
Rarity: NORMAL
Wooden Club
Item Level: 70
Quality: 20
Implicits: 0
```

The reviewed base names are Wooden Club and Smithing Hammer; custom records may rename these two supported slots; item level is 1–100 and quality is 0–20, as canonical integers. Whitespace around the complete payload and CRLF line endings normalize before hashing. Extra modifier lines, nonnormal rarity, range settings, corrupted items, and unknown bases fail closed. The selected records supply equip-level and attribute requirements; item level does not determine equip level. The reviewed bases have no level requirement; Smithing Hammer requires 11 strength. This is a bounded compatibility check, not a general equipment-requirement solver.

Support choices are `none` and/or `brutality_i`. Their Cartesian product with weapons produces at most 128 candidates. The compatibility claim is tied to the pinned source: `src/Data/Skills/sup_str.lua` defines Brutality I as supporting damaging attacks, and Mace Strike is the known one-hand Mace attack. Quality and level variants of support gems, support families, multiple supports, and arbitrary skill compatibility require further data translation and tests.

Every encounter input used by the original Mace template remains explicit. Permitted parameter changes include enemy level 1–85, boss setting `None`/`Boss`/`Pinnacle`, enemy armour, resistances, incoming damage components, penetration/overwhelm, and attack interval within conservative numeric bounds. Damage type stays Melee, enemy crit chance stays zero, the five optional condition toggles stay false, and nearby-enemy counts stay 1/0. Incoming damage must contain a positive component. Unknown configuration keys, custom modifiers, alternate scalar types, nonfinite values, omitted required inputs, and duplicates are rejected. These are fixed scenario inputs for the entire search, not optimizable choices.

## Requirement validation

`requirements(candidate)` returns available and required level/strength/dexterity/intelligence,
plus each failed boundary. `validate_requirements(candidate)` rejects a failure. Available
attributes come from the selected Warrior class record; this profile has no attribute-granting
items, paid passives or supporting effects. Each attribute requirement takes the maximum of
individual item/active/support requirements and the sum of matching-color support costs.
For example, 11 weapon strength and one red support costing 5 require 11 strength. The generic
additive resource budget is not used for this rule. Level requirements take a maximum too.

The CLI preflights every choice allowed by the locks before the diagnostic baseline, then
checks requirements again during search validation. Rejections consume no calculation
budget. An empty legal domain reports the reasons and zero evaluations. The catalog keeps
all alternatives so axes and exact locks stay stable; diagnostic source materialization and
native calculation may still inspect a build that fails requirements. General equipment
self-dependencies, acquired inventory and broader build legality are outside this profile.

## Identity and source preservation

The catalog identity includes the exact template SHA-256, sorted named weapon payloads, support choice set, projection version, pinned PoB source identity, and complete selected `DataIdentity`. Identical-content snapshots are compatible; different data rejects even if a result matches its own evaluator. Item and gem instances use hashes of exact normalized payloads; repeated PoB XML item ID `1` never merges different weapon settings. Catalog and candidate ordering are deterministic, and each materialization includes all canonical dimensions. `resolve_candidate(weapon_id, support)` exposes the stable two-axis mapping.

Materialization patches only the source ranges for the item element and optional support gem. It preserves every other template byte, including configuration, labels, Notes, comments, unknown prose, and formatting. Structural fields with unknown mechanics are rejected before mutation, so preservation does not silently imply support. The user's source file is not modified. A candidate from another catalog or any combination outside this finite registry cannot be materialized.

## Baseline and realization evidence

For the optional reviewed-default reference path, the caller first evaluates `template_build()` in a fresh supervised PoB process and passes its result to `bind_baseline`. This preparation attempt consumes the overall deadline and evaluation budget even if it fails. It is not a candidate cache hit or final verification.

Before creating `VerifiedMaceScenario`, the adapter checks the pinned backend identity, diagnostic marker, selected action ownership, exact active/support identities and settings, class/ascendancy/level/allocation, active sets, explicit source configuration, freshly exported weapon payload, and source metadata. The pinned host normalizes the legacy class ID `3` to canonical ID `6` through `classInternalId=6`, adds implicit Warrior start node `47175`, and inserts the derived `LevelReq: 0` item line. Those specific normalizations are checked explicitly. Other item-text rewrites fail.

`validate_realization(candidate, result, scenario)` repeats requested-versus-realized checks for each candidate, then compares all effective configuration inputs/placeholders and normalized noncandidate XML against the verified template. Player and enemy condition tables remain evidence and are not globally frozen: candidate mechanics can legitimately alter them. The source's explicit external condition inputs are still checked.

PoB uses Lua `pairs()` when exporting several tables, so equivalent fresh exports can reorder sections and entries. The drift signature sorts canonical child representations. Gem order is separately checked, including the selected active gem at index 1, so canonicalization does not permit support/active reordering. Numeric `PlayerStat`, `MinionStat`, and `FullDPSSkill` output nodes are excluded from the immutable signature. Noncandidate persisted state, including labels, notes, empty Buffs/TimelessData defaults, runes, configuration blocks, and other exported sections, remains guarded.

This baseline is **runtime-derived drift evidence**, not an independent numerical reference or a full legality certificate. The CLI must label all results diagnostic and require a fresh finalist verification before export. Canonical domain validation and realization checks complement each other; neither proves acquisition, full combat execution, mapping clear speed, progression assumptions, or completeness of PoB mechanics.

### Native realization

The native path uses `bind_native_baseline(result, expected_backend)` and
`validate_native_realization(candidate, result, scenario)`. It validates the selected native
backend/rules and matching catalog/data identity, exact materialized XML export, class/root, active/support identities and
settings, source configuration, stable external placeholders and parsed weapon base,
quality, item level and support evidence. It expects native source preservation, not PoB's
normalization. Candidate-derived condition tables remain free to change. Both paths retain
the same candidate locks, budgets, objective contracts and diagnostic status.

Native controlled search currently supports the normal-enemy subset; unsupported boss
scenarios fail preparation explicitly. Each native calculation runs directly on Rayon and
fresh finalist verification recomputes in fresh native state without PoB. There is no hidden
reference evaluation or fallback. See [native backend](native-backend.md) for complete scope.

## Validation and remaining work

`crates/poe-optimizer-pob/tests/controlled_mutations.rs` covers structural acceptance, exact payload identity, source-byte preservation, support addition/removal, coordinated choices, foreign-catalog rejection, unknown mechanics, ambiguous sets, and malformed encounter inputs. The generated quality-zero product matches all four unchanged independent C-host Mace DPS goldens. Fresh parameterized round trips cover quality 20, changed character/enemy levels, incoming damage, and item level, plus a Pinnacle scenario with explicit armour/resistance settings. Mutation probes reject source-frame and realized item/gem/configuration drift.

`crates/poe-optimizer-import/src/controlled_mace_tests.rs`, `tests/native_search.rs` and
`tests/dataset_search_cli.rs` cover selected snapshots, requirement boundaries, same-content
compatibility, cross-data rejection, custom names/quest selectors, serial/Rayon agreement,
empty domains and finalist reloads.

The four original goldens remain the independent numerical reference. Parameterized round trips establish requested state and interaction behavior; they are not newly independent absolute-number goldens. Future phases should expand independently calibrated item/support families, integrate translated tree and class data, preserve full candidate legality across combinations, and extend native calculation parity beyond the admitted profiles. No general source-document mutation or complete six-dimension release claim is made by this adapter.
