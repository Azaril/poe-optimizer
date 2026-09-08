# Controlled build mutations

Status: experimental finite-domain adapter, `poe_optimizer_import::controlled_mace` (also re-exported by `pob::mutation`), alongside the unchanged four-fixture `pob::candidate` calibration registry. It supplies parameterized weapon/support and bounded class/ascendancy/entrance search inputs. The end state remains joint class, ascendancy, passive, item, active-skill, and support search with configurable requirements and independent locks.

## Supported structural profile

`ControlledMaceCatalog::with_data(snapshot, template_xml, weapons, supports)` retains an immutable selected dataset and checks structure and values, rather than requiring one of four XML hashes. `new(template_xml, weapons, supports)` remains a convenience constructor for the reviewed default. The template is an unallocated Warrior with no ascendancy, tree `0_5`, one active item/skill/configuration set, exactly one manually specified Mace Strike at level 1/quality 0, and zero to two reviewed supports at level 1/quality 0; legacy constructors retain the single-Brutality choice interface. The only equipped item is slot `Weapon 1`, XML item ID `1`. Character level is explicit, 1–100, with automatic leveling disabled. Only the admitted passive selections, equipped Mace and five local weapon modifier families are supported. Extra equipment, runes, global/conditional modifiers, active skills, item-granted skills, minions, alternate sets and unknown mechanics remain unsupported.

`with_tree_choices(snapshot, template_xml, weapons, supports, selections)` adds explicit
`ClassTreeSelection` values: canonical numeric `class_id`, optional internal `ascendancy_id`,
optional physical `entrance_node_id`, and optional `ascendancy_node_id`. The template may use any admitted selection.
`poe_optimizer_data::class_tree` owns the shared resolver and partial candidate graph; the
partial bundle is never represented as a complete extracted tree. All 31 class/ascendancy
identities support none or either of two class-local ordinary entrances, giving 93 choices before paid ascendancy nodes. An optional `ascendancy_node_id` adds one
of four reviewed owned small nodes, yielding 105 selections. These grant signed
unconditional player BASE resistance effects: Warrior3/14960, Druid2/61722, Monk3/24475
and Huntress3/17058. Ascendancy roots remain implicit and statless. Other allocated
ascendancy effects stay unsupported.

The weapon list contains 1–64 distinct exact item payloads with distinct user labels. Legacy normal items use these five lines:

```text
Rarity: NORMAL
Wooden Club
Item Level: 70
Quality: 20
Implicits: 0
```

The shared [local item parser](local-weapons.md) also admits `RARE` items with a separate
name line, optional explicit `LevelReq` and five configured local modifier families.
The reviewed base names remain Wooden Club and Smithing Hammer; custom records may rename
these two supported slots. Item level is 1–100, quality 0–20 and explicit equip level 0–100.
Reviewed modifier rolls use bounded unsigned integers. Ordered duplicate local lines are
preserved, without claiming affix-tier or affix-count legality. Unknown/global/conditional
modifiers, unsupported rarity, ranges, corruption, sockets, runes and enchants reject.

Exact raw item text, including edge whitespace and CRLF, now supplies the payload identity.
Interpretation ignores line-edge ASCII whitespace and empty lines without changing the
preserved source. Native and catalog XML admission use one shared element reader before
XML line-ending normalization. Problem schema **5** / report schema **6** enables these
items through the CLI; schemas 1–4 retain their previous normal five-line scope in both
templates and alternatives. `NormalMaceAlternative` remains a compatibility alias for
`MaceWeaponAlternative`. See the [local weapon example](../examples/mace-local-weapon-search.json).

The selected records supply base attribute requirements. An authored `LevelReq` replaces
the base equip level for this non-unique item path; otherwise the base level applies.
Item level does not determine equip level. The reviewed bases have no level requirement;
Smithing Hammer requires 11 strength. This is a bounded compatibility check, not a general
equipment-requirement solver.

[Support loadouts](support-loadouts.md) contain zero to two data keys from Brutality I,
Heavy Swing and Rapid Attacks I. `with_loadouts` and `with_tree_loadouts` accept these
canonical unordered choices; the old constructors map `none`/`brutality_i` to the same
representation. Seven loadouts and 64 weapons produce at most 448 alternatives per tree,
or 47,040 with all 105 selections. The estimated and cumulative 256 MiB source-hashing
work cap still applies. Duplicate keys/families and unsupported eligibility reject.
Gem levels/qualities other than 1/0 and other support mechanics remain unsupported.

Every encounter input used by the original Mace template remains explicit. Permitted parameter changes include enemy level 1–85, boss setting `None`/`Boss`/`Pinnacle`, enemy armour, resistances, incoming damage components, penetration/overwhelm, and attack interval within conservative numeric bounds. Damage type stays Melee, enemy crit chance stays zero, the five optional condition toggles stay false, and nearby-enemy counts stay 1/0. Incoming damage must contain a positive component. Unknown configuration keys, custom modifiers, alternate scalar types, nonfinite values, omitted required inputs, and duplicates are rejected. These are fixed scenario inputs for the entire search, not optimizable choices.

## Requirement validation

`requirements(candidate)` returns available and required level/strength/dexterity/intelligence,
plus each failed boundary. `validate_requirements(candidate)` rejects a failure. Available
attributes come from the resolved selected class record. The admitted entrance operations
do not change strength/dexterity/intelligence, and the profile has no attribute-granting
items or supporting effects. Each attribute requirement takes the maximum of
individual item/active/support requirements and the sum of matching-color support costs.
For example, 11 weapon strength and one red support costing 5 require 11 strength. Two red supports aggregate to 10 strength; red/green costs remain separate. The generic
additive resource budget is not used for this rule. Level requirements take the maximum of the item's effective equip level and each gem's level requirement; explicit item `LevelReq` is resolved before that maximum.

The CLI preflights every choice allowed by the locks before the diagnostic baseline, then
checks requirements again during search validation. Rejections consume no calculation
budget. An empty legal domain reports the reasons and zero evaluations. The catalog keeps
all alternatives so axes and exact locks stay stable; diagnostic source materialization and
native calculation may still inspect a build that fails requirements. General equipment
self-dependencies, acquired inventory and broader build legality are outside this profile.

## Identity and source preservation

The catalog identity includes the exact template SHA-256, sorted named weapon payloads, support choice set, projection version, pinned PoB source identity, and complete selected `DataIdentity`. Identical-content snapshots are compatible; different data rejects even if a result matches its own evaluator. Item instances hash exact supplied item text; gem instances hash their canonical supported-attribute JSON. Repeated PoB XML item ID `1` never merges different weapon settings. Catalog and candidate ordering are deterministic, and each materialization includes all canonical dimensions. `resolve_candidate(weapon_id, support)` retains the template-tree mapping. Expanded
`resolve_tree_candidate(selection, weapon_id, support)` addresses the complete selection.
Expanded catalog identity also includes ordered tree choices and every composed payload.

Materialization patches the source ranges for the item element, support gem ranges and,
in expanded catalogs, selected Build/Spec class, ascendancy and allocation attributes. It
preserves every other template byte, including configuration, labels, Notes, comments, unknown prose, and formatting. Item text is inserted without an added newline; supported named entities are escaped as needed while literal CRLF is retained. Structural fields with unknown mechanics are rejected before mutation, so preservation does not silently imply support. The user's source file is not modified. A candidate from another catalog or any combination outside this finite registry cannot be materialized.

## Baseline and realization evidence

For the optional reviewed-default reference path, the caller first evaluates `template_build()` in a fresh supervised PoB process and passes its result to `bind_baseline`. This preparation attempt consumes the overall deadline and evaluation budget even if it fails. It is not a candidate cache hit or final verification.

Before creating `VerifiedMaceScenario`, the adapter checks the pinned backend identity, diagnostic marker, selected action ownership, exact active/support identities and settings, class/ascendancy/level/allocation, active sets, explicit source configuration, freshly exported weapon payload, and source metadata. The pinned host normalizes the legacy class ID `3` to canonical ID `6` through `classInternalId=6`, adds implicit Warrior start node `47175`, and regenerates item metadata including the effective `LevelReq` (zero for the original normal fixtures). For fixed explicit local modifier lines, it emits one neutral `ModRange` per line. Realization requires exact ordered IDs, count and `range="0.5"`, preserving modifier order/spelling and rare name. Those specific normalizations are checked explicitly; other item-text or range rewrites fail.

`validate_realization(candidate, result, scenario)` repeats requested-versus-realized checks for each candidate, then compares all effective configuration inputs/placeholders and normalized noncandidate XML against the verified template. Player and enemy condition tables remain evidence and are not globally frozen: candidate mechanics can legitimately alter them. The source's explicit external condition inputs are still checked.

PoB uses Lua `pairs()` when exporting several tables, so equivalent fresh exports can reorder sections and entries. The drift signature sorts canonical child representations. Gem order is separately checked, including the selected active gem at index 1, so canonicalization does not permit support/active reordering. Numeric `PlayerStat`, `MinionStat`, and `FullDPSSkill` output nodes are excluded from the immutable signature. Noncandidate persisted state, including labels, notes, empty Buffs/TimelessData defaults, runes, configuration blocks, and other exported sections, remains guarded.

This baseline is **runtime-derived drift evidence**, not an independent numerical reference or a full legality certificate. The CLI must label all results diagnostic and require a fresh finalist verification before export. Canonical domain validation and realization checks complement each other; neither proves acquisition, full combat execution, mapping clear speed, progression assumptions, or completeness of PoB mechanics.

### Native realization

The native path uses `bind_native_baseline(result, expected_backend)` and
`validate_native_realization(candidate, result, scenario)`. It validates the selected native
backend/rules and matching catalog/data identity, exact materialized XML export, class/root, active/support identities and
settings, selected physical ordinary/ascendancy allocations, implicit roots, effective node IDs and configured
entrance effects, source configuration, stable external placeholders and parsed weapon base,
quality, item level and support evidence. Mace profile attachment version **3** adds exact
`weapon_item` evidence: raw payload identity, rarity/name, explicit/effective equip level,
ordered local rolls and rule provenance. `weapon_stats` exposes the prepared local values. It expects native source preservation, not PoB's
normalization. Candidate-derived condition tables remain free to change. Both paths retain
the same candidate locks, budgets, objective contracts and diagnostic status.

Ordinary native search attempts use [private typed candidate handles](native-candidate-evaluation.md)
by default. `native_components` requires this fresh bound baseline, and
`validated_native_candidate` checks catalog membership and requirements before creating a
handle. Each weapon's local stats are prepared once against the shared selected dataset;
private bindings reject a mismatched catalog or compiled dataset. The prepared numerical
axes retain no XML or candidate-result cache. Candidate locks and caller point budgets
remain domain responsibilities. The reserved finalist uses the complete native realization
path above, with one fresh calculation counted in the shared ledger. The optional
`--native-evaluation document` mode applies that path to every ordinary attempt as well.

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

`tests/class_search_cli.rs`, `tests/class_search_parity.rs`,
`tests/resistance_search_cli.rs` and `tests/resistance_build_parity.rs` add coupled class/tree/item/
support checks, caller budgets, exact locks, source-preserving exports and fresh PoB
materialization/reimport comparisons. Search rejects illegal point/ownership/requirement
combinations before dispatch. The native diagnostic allocation count remains evidence of
used points only; it does not supply the caller's budget.

The four original goldens remain the independent numerical reference. Parameterized round trips establish requested state and interaction behavior; they are not newly independent absolute-number goldens. Future phases should expand independently calibrated item/support families, broaden translated tree and class effects, preserve full candidate legality across combinations, and extend native calculation parity beyond the admitted profiles. No general source-document mutation or complete six-dimension release claim is made by this adapter.
