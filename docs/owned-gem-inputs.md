# Intrinsic gem inputs during import

This document covers physical input conversion. The separate
[effective-input recipes](owned-effective-gem-inputs.md) compute active pre-support
and support-preparation values from those inputs and injected modifier channels.
Import never substitutes a cached effective level for a physical value.

A Gem schema declares owned slots; it does not prove how an external document supplies
those slots. Even a Complete empty declaration requires an explicit, schema-bound
`GemInputPolicy` before the import adapter can close `GemDraft.parameters`.

Each injected rule selects an exact Gem definition, optional finite source-attribute
guards and typed parameter recipes. Guards distinguish missing, empty and exact text;
missing is never a wildcard. Recipes reuse the existing Boolean, Integer, Quantity and
Option codecs and explicit missing-value policy. There is no built-in corruption name,
Lua conversion fallback, skill-name dispatch or source operation in the evaluator.

Validation requires a Known Gem, slots declared by that exact Gem, compatible parameter
sites, exact units/types, valid explicit defaults and bounded policy/source work. A rule
closes a collection through declaration coverage only if all source guards pass, the
declaration is Complete, every declared slot is covered and every required value converts.
The separate finite physical-inventory proof below can establish a concrete collection
without claiming complete static declarations. Known values survive under
Partial declarations. Unmapped owners, missing rules, failed guards, absent required
values and malformed/out-of-range values retain Pending coverage. Optional absence is
admitted only through an explicit recipe and OptionalOnce slot.

Policies that omit `gem_inputs` retain their serialized bytes and policy-v3 identity.
Fresh normalization now leaves their Gem parameter collections Pending; old persisted
drafts are not rewritten. Sidecar schema/domain **12** identifies the changed import
behavior. Consumers must not compare a v11 inference with a v12 conversion as an
unchanged-behavior replay.

The [reviewed neutral policy](../data/owned/poe2/3887ae68/gem-inputs/README.md) supplies
exact guards for the two existing Known Gem definitions. It restores the twelve original
empty parameter collections only for corruption text `false`/`nil` and delta text `0`.
Edited/nonzero, missing and unreviewed spellings remain Pending under that policy. This
is a deliberately finite admission policy, not the whole source loader's accepted language.

Level, quality, support target, use scope, choices, generated actors/actions, active
contributors and numerical coverage retain their separate obligations. The full source
LoadSkill audit also finds count, global activation switches, choices and per-effect
maps. Knowing the two corruption scalars does not prove an exhaustive input model.

## Physical Gem knowledge migration

Unmapped-to-Known Gem conversion uses a separate import-only V4 refinement; the existing
V3 Known-schema membership contract is unchanged. The exact prior schema, registry,
source pin, mapping and role evidence bind the migration. Only listed physical Gems
with a known role and primary skill may be promoted. Provider-only identities are rejected.
New registry entries may only be explicitly declared parameter slots owned by those Gems.

Every previous definition, slot, program body, closure and route remains unchanged except
for the listed Gem knowledge and necessary artifact identity rebinding. Potential skill
membership and unreviewed choices, grants, actors, outputs and sockets remain Partial.
The currently reviewed two-scalar conversion must also retain Partial parameter membership
until the remaining saved inputs have an owned classification. Registry/source evidence
is not numerical game-rule coverage.

`migrate-owned-gem-schemas` hosts the reusable checked migration. A separate
`publish-owned-normalization` operation installs a complete caller-supplied policy,
validates the true previous policy/tree, preserves recipes and all query rows, and binds
unchanged tree content to the new policy. Stale authored bindings reject; they are never
silently repaired. Both commands publish to a new directory only.

Normalization allocates fresh occurrence and issue identities. Resolving an issue may
shift later allocator-local IDs. Compare independent runs through source attribution and
semantic values while preserving query identities/order; do not allocate phantom issues.

Support receiving and activation follow the separately accepted
[support contract](owned-support-activation.md). These input APIs introduce no new Core
or evaluator contract and do not complete any original build.

## Catalog conversion

`compile-owned-gem-inputs` is a source-free offline compiler for an explicitly selected
physical Gem family. The caller supplies a finite identity catalog and reviewed schema,
quality membership, parameter types and lexical recipes. No Gem name, level domain,
corruption field or default is embedded in the native calculation engine.

The compiler binds the catalog to the original role-compilation digest and immutable
source-file pins. Every selected source row needs an exact unique external mapping,
matching primary effect, known physical role and an Unmapped prior Gem descriptor.
It allocates owner-scoped slots in canonical Gem order and retains Partial membership
for every generated Gem collection. A shared policy is expanded only within bounded
input/output resources. Existing known definitions and rules are preserved.

Compilation emits the explicit V4 migration, successor-bound normalization policy and
both checked transition receipts. Separate publications keep the true previous policy
and the intermediate schema package auditable; neither silently repairs stale bindings.

The [reviewed support family](../data/owned/poe2/3887ae68/support-gem-inputs/README.md)
contains 514 single-declared-effect identities and two independent typed inputs. Fractional
corruption deltas use a Quantity rather than an Integer. Source-loader acceptance outside
the injected lexical/domain policy remains Pending. These declarations supply known
inputs for later support resolution; they do not implement support delivery or calculation.

## Explicit numeric token aliases

`ValueRecipeInput.numeric_aliases` is an optional finite import table for reviewed
nonnumeric source tokens. Each row contains a token and a replacement string decoded
once by the recipe's existing Integer or Quantity codec, including its scale and unit.
Aliases do not chain. Boolean/Option codecs reject a nonempty numeric alias table.
The existing direct codecs, including item-line formatting, are unchanged.

Candidate membership, precedence, duplicate handling and availability are resolved before
alias lookup. A converted alias remains a Selected value with the original source origin;
it is not a missing-value default. The codec's exact/ASCII-trim policy applies to lookup.
Colliding normalized tokens reject, as do tokens matching the full scientific decimal
grammar, even if that spelling would overflow or be non-integral for the target codec.
Limits bound row count, original token/replacement bytes and their aggregate before
lookup allocation. The Gem compiler additionally validates all scaled replacement values
against the owning slot's schema and charges the schema-work budget.

Empty or omitted alias tables retain historical serialized bytes and policy identities.
A nonempty table changes the existing normalization policy identity; it does not change
Core inputs, direct numeric syntax or the sidecar schema. The reviewed support successor
supplies exactly `nil` -> `0` for its 514 numeric corruption-delta recipes. Missing,
unavailable and unlisted malformed values remain Pending. Publish this explicit complete
normalization replacement through the existing command without another Gem-schema
migration or any change to parameter membership closure.

## Multiple potential effects on one physical Gem

`GemEffectMembershipPolicy::ResolvedPotentialSkillsV1` is an explicit offline compiler
opt-in. It proves the potential Skill set from resolved declared, constructed and final
catalog references, joins each through exact owned mapping, and canonicalizes membership.
Generated additional references and reordered display lists are allowed only when those
sets agree. A missing declared effect cannot disappear merely because source construction
omits it from a resolved list. Additional stat-set declarations are outside this mode.
Per-Gem/aggregate membership and pre-expansion byte budgets bound the work before copies.

The primary effect establishes the physical Gem's authored role. Additional effects
remain potential Partial Skills on that same Gem; conversion creates no additional
physical Gem, active SkillUse, support application, actor or provider. Physical input
levels and derived active-effect levels are distinct. The optional reference can inherit
an accepted supported action's level for an additional effect, so a physical level-1
schema must not constrain every derived effect to level 1.

Omitted/default `SinglePrimary` retains its historical serialized policy representation
and singleton publication path. The [multi-effect support family](../data/owned/poe2/3887ae68/multieffect-support-gem-inputs/README.md)
uses the existing V4 schema refinement and explicit normalization publication. Its inputs
and potential memberships stay Partial until remaining activation/choice/provider and
numerical obligations are represented and validated separately.

## Finite single-support physical inventories

An optional `GemInventoryPolicy::PobFreshSingleSupportV1` binds the exact definition
schema, role package, source catalog and existing scalar/admission recipes. Its injected
rows identify a reviewed physical support, exact external identities and its two
owner-scoped corruption slots. The reviewed domain comes from the authenticated catalog,
including absent additional effects and stat sets; a singleton Partial Skill list or a
Support role alone is insufficient evidence. Hybrid supports remain outside this domain.

The adapter proves an actual saved occurrence before completing its parameter list.
Every source field must fit the finite grammar: exact identities, converted intrinsic
level/quality and corruption values, known Gem/group activation, count one, both global
flags true, and absent or literal-nil legacy stat-set attributes. Complete PoB LoadSkill
overwrites those legacy tables before loading child maps. Child maps, group part
overrides, unknown attributes, namespaces and unsupported contexts remain Pending.
Manual and admitted generated-group supports retain their separate target obligations.

Successful proof supplies a private token; callers cannot request completeness with a
Boolean. Existing scalar recipes supply the values, and the proof preserves their exact
assignments. Static Gem declarations, potential effects, rule owners, support applicability
and numerical coverage remain unchanged. Failed proof preserves the known values and
the existing membership issue. Normalization allocates no phantom retired issue.

Omission preserves historical policy bytes, normalization results and allocation order.
The sidecar shape is unchanged; its existing policy/source commitments record the new
interpretation. Publication checks all proof commitments. A checked schema transition may
rebind an already validated prior; explicitly supplied replacements must carry correct
bindings and are never repaired. No source catalog, Lua runtime or PoB field name enters
the native evaluator through this adapter.

## Remaining physical-input classification

Completing a physical Gem's input inventory requires a reviewed disposition for
every semantic source field. It does not require copying every source attribute
into a native parameter. The adapter owns external field names; the runtime owns
typed physical values, activation, occurrence relationships and action selections.

| Source input family | Owned responsibility and remaining proof |
| --- | --- |
| Physical level and numeric quality | Intrinsic values, separate from effective Skill inputs. A computed alternate-quality effect is not by itself another physical quality-kind choice. |
| Corruption Boolean and level adjustment | Separate typed inputs. The Boolean can affect predicates even when the adjustment is zero; the adjustment can affect active levels when the Boolean is false. |
| Gem/group enabled state and weapon scope | Explicit activation and use scope. Retain disabled occurrences and saved presets. |
| Global effect switches | Activation of particular granted-effect ordinals, with an explicit effect correspondence. They are not weapon-set selectors. |
| Gem count, group count and full-DPS inclusion | Population, reservation and reporting semantics. Group-count precedence must be preserved; a displayed count does not automatically create that many Actor occurrences. |
| Skill part, stages, mines and main/calculation selections | Action and usage selections, including differences between calculation contexts. |
| Minion selection and equipment-set reference | Parent population selection and an explicit conditional equipment relationship. Neither is an arbitrary field on every Actor provider. |
| Per-effect stat-set and minion-skill child maps | Exact child/action selections. Legacy scalar attributes overwritten by the source loader must not become phantom required native inputs. |
| Titles, notes and ordering preferences | Import/UI provenance when they have no calculation consumer; they do not create numerical input obligations. |

Unknown source fields retain explicit unknown evidence until classified. Proving a
quality-kind inventory complete does not complete parameter, choice, activation or
potential-effect membership. Likewise, a source constructor census supplies adapter
evidence; it does not certify complete native calculations. Convert these families
through shared policies across the reviewed physical catalogue, while keeping
unmapped identities and exceptional effects explicit.

### Pinned physical-quality evidence

The optional Rust PoB test `owned_physical_quality_inputs` runs authenticated,
unchanged `LoadSkill` and `ProcessSocketGroup` functions against all 966 physical
catalogue rows. All rows resolve in the pinned checkout; there are no exclusions
in this run. Nine cases per row cover missing, literal `nil`, malformed, zero,
fractional and ordinary amounts, unrecognized/numeric quality-kind attributes,
and legacy UI attributes. JIT-off/on observations agree across 8,694 cases per mode.

The loader retains a numeric quality amount and no quality-kind selector. Missing,
literal `nil` and malformed amounts remain source `nil`; the native importer still
reports those amounts Pending under its current explicit policy. A source-ignored
`qualityId` is evidence about that adapter, not automatic authority to accept every
token or turn unknown native input into zero.

A scoped source component supplies the real Advanced Thaumaturgy node (`14429`)
to the unchanged environment constructor. It enables computed alternate-quality
stats without changing physical quality. The stat constructor retains ordinary
quality effects and adds the alternate effects for Twister and Cleric; Sniper's
tested stat set has no such addition. Original allocations, physical values,
selected action and recorded output scalars remain unchanged by the probe.

The same test distinguishes real child stat-set maps from overwritten legacy
scalar attributes, and observes group count overriding physical Gem count,
including explicit zero. These are source input/component facts, not native
numerical parity. A future quality-kind refinement must join witnessed identities
to exact mapped Known Gem descriptors, preserve other Partial facets, and prove
the published schema change separately. The source test itself changes no quality closure.

The [finite quality-kind revision](../data/owned/poe2/3887ae68/physical-gem-quality-kinds/README.md)
applies that evidence to the exact 603 mapped Known physical Gem descriptors whose
ordinary-kind inventory was Partial. The already Complete singleton and 362
Unmapped descriptors remain unchanged. Quality presence, amount semantics and
every other declaration retain their prior meaning. This is a schema facet
correction; it does not supply alternate-quality calculations or close rule owners.

Additional-stat-set metadata also need not block physical-input conversion. The
expanded active-input source test verifies twelve further families as one physical
Gem with one primary effect, while the extra aliases remain separate action
metadata. Their prepared policy reuses `SinglePrimary`; no extra native Skill or
new importer mode is needed. Actual action stat-set identities/selections still
need their own complete constructed inventory. The
[published physical-input successor](../data/owned/poe2/3887ae68/active-gem-inputs/statset-publication/README.md)
adds twenty-four parameter slots across thirty-four original occurrences. The new
twelve quality-kind facets remain Partial; the preceding 604 Complete inventories
are preserved. Fresh selected-request checks still fail at draft finalization,
so this input addition is not a cleared original-build evaluation gate.

### Action stat-set evidence still required

Physical input conversion does not resolve an action's stat-set selection. When
a selected-build blocker requires this work, its source witness must enumerate the
actual constructed
`grantedEffect.statSets` tables and exercise the unchanged loader, environment
constructor and active-skill constructor. Observe the selected table's identity,
index and flags separately for main and calculation contexts; do not assign
indices from the additional-stat-set aliases or merge this inventory with Skill
Part choices.

Use every constructed index for the twelve reviewed families, conflicting main
and calculation child maps, absent maps and unrelated effect keys. Preserve
malformed, duplicate, fractional and out-of-range cases as explicit observations.
Source inspection shows that invalid numeric indices can reach a missing table;
there is no basis for converting them silently to index 1. A complete source
witness is still needed for the resulting downstream behavior. This evidence
will inform finite owned action choices without creating additional physical
Gems or retargeting any original query.

### Active occurrence lifecycle and the usage composition boundary

The active-occurrence witness follows all five unchanged original builds through
the complete pinned source lifecycle, observing their actual MAIN and CALCS
contexts. Its catalogue-derived candidate group contains 22 selected occurrences
and 19 physical Gem definitions. Every candidate's constructed primary selects
the actual first stat-set table; this does not assign an owned action identity.
Only Twister currently has an owned action inventory. The other 18 mapped primary
Skills remain Unmapped, so their missing output/part/mode/stat-set declarations
remain an independent native dependency.

Count belongs to use and reporting behavior. The source count helper preserves
parent group-count precedence, including explicit zero. Twister count three
alone leaves CombinedDPS unchanged and FullDPS excluded. When the group is included,
FullDPS changes while its strongest Ignite contribution retains count one. Thus
neither copying count onto the physical Gem nor multiplying the whole per-skill
result describes the observed behavior.

The current candidates have no MultipleReservation type or active UmbralWell
environment modifier flag in the observed contexts. The latter is read from the
actual environment ModDB, separately from minion-limit and buff-query context
availability. Pain Offering retains CreatesMinion and minion-type metadata,
but its action has no attached minion actor in either saved context. Turning off
its first global switch removes that effect and changes the selected Sniper
minion's outputs while direct player outputs stay unchanged. That is not an
authored physical-Gem enable toggle or a population inferred from count. The
witness reads flags on the actual selected stat-set record and explicitly records
the absence of instance-level flags; absence is not fabricated false/zero output.

These observations justify the next model work, not input-list completion. The
[skill-usage composition contract](owned-skill-usage-proposal.md) was accepted on
2026-10-02: complete and draft skill presets will retain occurrence-targeted
preferences, and explicit request composition will apply scenario overrides. Today scenario
usage has neither this preset composition nor executable native policy programs.
All original physical-list obligations and all 110 queries remain unchanged at
this source-investigation checkpoint. Full native evaluations remain 0/5.
The Rust regression `owned_active_gem_occurrence_inputs` passes with identical
observations in both JIT modes: 15 full loads and 48 executions of eight distinct
fresh loader controls per mode. Source subprocess isolation is test-only; it
introduces no evaluator runtime dependency.

The other remaining active physical-input group has five families and seventeen
materialized occurrences: Skeletal Arsonist (six), Brute (one), Frost Mage (four),
Reaver (five) and Storm Mage (one). Their catalog declares command effects that
are absent from the constructed Skill catalog. Skeletal Sniper has the same
source-reference discrepancy but already has an explicitly authored Known
physical schema. Missing references must remain visible even when a constructed
effect list contains only its primary summon. Do not erase them to satisfy a
singleton compiler guard or invent replacement command Skills. A subsequent
physical-input conversion must preserve the unresolved potential-effect gap
separately from any source-proven scalar facts.
