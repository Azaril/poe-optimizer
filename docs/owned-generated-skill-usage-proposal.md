# ADR: usage preferences for provider-generated skills

**Status:** Proposed; owner decision required before changing Core or Import contracts.
**Date:** 2026-10-03.
**Decider:** Project owner.

## Decision to make

Keep generated-skill preferences with the selected skill preset, with explicit
applicability to exact providers in the selected build, or move usage into a
separate layer bound to a combined build selection. **The first option is
recommended.** It extends the accepted preset/scenario composition model while
keeping the provider graph authoritative.

This proposal concerns ownership, persistence and composition of intent. It does
not choose the pending canonical PoB reference lifecycle, complete generated
skill mechanics, or authorize a new activation writer.

## Current implementation and concrete gap

The accepted [skill-preset usage design](owned-skill-usage-proposal.md) stores
typed preferences in `SkillPreset.usage_preferences` and the corresponding draft
list. `owned_project::compose_request` composes the selected build, merges the
selected skill preset's preferences with scenario usage by exact `(policy,
target)`, and emits the existing `OwnedEvaluationRequest`. A scenario override
replaces the whole parameter record. Both layers are structurally checked before
replacement; the native binder subsequently validates definitions and topology.

Today, `PresetUsageScope::root` in
[`owned_build/structure.rs`](../crates/poe-optimizer-core/src/owned_build/structure.rs)
accepts only SkillUse and SupportAssignment roots belonging to that preset.
`scoped_root` in
[`owned_draft/structure.rs`](../crates/poe-optimizer-core/src/owned_draft/structure.rs)
enforces the same restriction. The generic `UsageTarget` can name generated
Skill, Actor and Action occurrences rooted in allocations or equipment, but
putting such a target in a skill preset is currently rejected. A permissive
scenario target is not a substitute for per-skill-preset ownership.

This restriction now blocks source conversion. Selected Original05 has nine
authored roots, plus saved representations of Tree Sand Djinn, Tree Water Djinn
and item-granted Firebolt. The membership importer correctly excludes those
three representations from authored roots. That classification does not prove
their saved usage settings are irrelevant.

The optional complete-source membership witness records all five originals,
archived selections, fresh state and two requested rebuilds. Matching generated
groups can retain saved group/Gem state. Pinned `CalcSetup.lua` reconstructs the
provider association and updates certain fields, including forcing
`enableGlobal1`; it does not replace every saved setting. The report distinguishes
saved objects, runtime groups and actual providers. In particular, removing the
granting item leaves saved source evidence but removes the generated Firebolt
runtime group; an independent manual Firebolt survives.

In the unchanged selected Original05, those three groups save enabled `true`,
count `"nil"`, global1 `true`, global2 `"nil"` and Full DPS `"nil"`. Retaining those
lexemes is not authority to import count one or a false reporting preference.
The source loader/reconstruction and each consumer need separate dispositions.

Evidence is the ignored Rust test
[`owned_authored_skill_membership_source.rs`](../crates/poe-optimizer-pob/tests/owned_authored_skill_membership_source.rs)
and `runs/owned-authored-skill-membership-source-01/source-jit-{off,on}.json`.
Both reports are 39,097,214 bytes, SHA256
`95738411d0c508b54bfab4ea89ffe8bdc74a2db1b0529a08f058c51a88fb2611`.
They prove source ownership and recorded settings, not a complete semantic
disposition for enabled/count/Full DPS or numerical parity. Additional controls
must establish each setting's actual consumer before importing its meaning.

The exact owned provider paths are also unfinished. Tree definitions abbreviated
`0b34` and `10d4` have Partial grant inventories. A reviewed `Tree:...` or
`Item:...` source spelling is ownership evidence for Import, not an executable
native provider address. The declared granting owner determines whether an item
skill uses EquipmentUse or ItemModifier ancestry; the importer must not guess.

## Options considered

| Option | Benefits | Cost and limitations |
| --- | --- | --- |
| **A. Skill-preset intent with explicit provider applicability — recommended** | Preserves independent skill alternatives and the existing scenario override boundary. Uses the same targets, typed parameters, native rules and occurrence graph. Selecting another item/tree combination can preserve dormant intent without retargeting it. | Requires a versioned complete/draft preference extension and explicit composition diagnostics. Standalone skill-preset validation cannot certify external supply; selected-build and definition binding must finish that proof. |
| **B. Usage bound to a combined variant** | All supplying axes are explicit at the preference's storage boundary. A combined selection can be validated without allowing a skill preset to refer outside its own roots. Both authored and generated usage can still feed one native usage list. | Introduces a usage-preset/combined-variant selection contract or couples scenario alternatives to build combinations. Independent equipment, tree and skill alternatives need additional bindings or duplicated preference records. Migrating all usage avoids parallel semantics but revisits the already accepted model. |

Option B is appropriate if usage is intended to be edited primarily as part of a
complete named configuration. That is a reasonable product model, but it is less
suited to the current independent presets and optimization across their axes.
Keeping only generated usage in scenarios while retaining authored usage in
skill presets would leave inconsistent ownership and is not the recommended
implementation of either option.

## Recommended contract: intent and supply are separate

A skill preset owns a preference because it is part of that skill configuration.
The preference does not own, supply or activate its target. Its target retains
the existing exact native occurrence address and provider ancestry.

Introduce an explicit, finite applicability contract for preferences that refer
to providers selected by other build axes. The names below describe semantics,
not approved Rust types or wire fields:

- **Required target:** the target must be supplied by the selected configuration.
  Existing preferences retain their strict supplying-preset semantics.
- **When its exact source is selected:** the preference is dormant only when a
  validated project occurrence is provably excluded by the selected build. When
  included, its target must pass the ordinary complete provider/target checks.

The initial extension should admit the demonstrated Allocation, EquipmentUse
and ItemModifier roots through this contract. It must not globally permit every
ProviderRoot or use an arbitrary predicate language. The applicability check
uses the roots already present in the typed target; a separate user-supplied
guard cannot authorize a different target. For an Action with an owned Actor,
check both provider ancestries and their declared correspondence.

Standalone project/draft validation must still check lineage, occurrence kind,
namespace, exact item-modifier ownership, bounded paths, duplicate keys and
applicability/target consistency. An external occurrence must be a real project
record. A historical, deleted or unresolved root is not proof of deliberate
nonselection. Drafts preserve the obligation; complete inputs reject invalid
ownership. No lookup by definition name, item name, source group position or
"whichever provider grants this skill" is allowed.

The new versioned contract also requires data-aware validation of every stored
resolved preference's policy and parameter schema, including dormant, unselected
and overridden records: policy existence, target kind, exact slot owner,
required values, types, units and ranges. A not-applicable declaration must not
launder an invalid value. This is stronger than the current pre-merge structural
check and must be an explicit opt-in contract, not a reinterpretation of old
bytes. Unresolved draft records retain their diagnostics and the accepted
isolation of unselected preset obligations; they are not complete, schema-valid
records and cannot acquire a not-applicable proof to hide missing information.

### Composition and binding outcomes

| Situation | Required result |
| --- | --- |
| Exact external source exists in the project and is included by selected presets | Include the preference; bind its exact generated path/slot and execute through ordinary usage planning. |
| Exact external source exists, is excluded by a complete selected-build inventory, and the preference explicitly permits nonselection | Retain the saved preference and report it as not applicable for this composition. Do not emit its executable usage record. |
| Required source is excluded | Diagnose the unsatisfied required preference; do not silently erase it. |
| Source identity, selected membership or required ownership is unresolved | Keep Pending/Unresolved. Absence from a partial list cannot prove nonselection. |
| Source is included but a grant, skill, actor or action correspondence is Partial/unresolved | Keep the ordinary binding/coverage obligation. An applicability flag does not certify a target. |
| Source is included but the requested declaration is provably missing or belongs to another owner | Reject the invalid/stale target; do not reinterpret it as dormant intent. |
| Declared target exists but ordinary activation is false or unknown | Preserve native inactive/unresolved behavior and completeness checks. Applicability does not change activation. |
| A different item/allocation grants the same skill definition | Do not transfer the preference. Retargeting is an explicit edit/search mutation. |

Changing the equipment preset can therefore deactivate an optional Firebolt
preference and restore it when the same exact source is selected again. Deleting
that source entirely leaves a repairable stale reference instead of silently
adopting another staff. Weapon-loadout scope remains native activation; merely
having another weapon loadout does not authorize composition to invent or erase
a provider.

The composition result must expose applied, overridden, not-applicable and
unresolved preferences with stable origins for CLI/GUI explanations. A not-
applicable result is an explicit proof with a reason, not a list entry dropped
without trace. It is distinct from a query's Unavailable result.

## One request and execution path

Extend the existing checked composition operation and draft finalization path;
do not add a source-specific request builder. Both options ultimately produce
the same resolved `UsagePolicySelection` list in `OwnedEvaluationRequest`.
Keep UI, XML source fields and compatibility rules in Import/editing metadata.

Composition decides structural selection using the selected build. Core does
not acquire a game-data index merely to evaluate a preference. Existing
schema validation must be reused in a data-aware project-preference validation
step before certifying the new contract's applicability. Cache this proof by
project content and data identity, including unselected records. Native
definition binding then proves selected slots and paths, and existing Engine
usage planning binds ordinary rule invocations to those exact occurrences.
Cold validation/composition/binding and their diagnostic receipt together
establish applicability; no evaluation can bypass a step by accepting a
structural composition success as numerical readiness.

Validate all authored layers and their resource use before replacement or
exclusion. Malformed preferences cannot be hidden by a scenario override or a
not-applicable predicate. Structural checks alone are insufficient for the new
contract's schema guarantee. Legacy requests retain their current validation
boundary; changing their treatment of overridden values is outside this decision.

Scenario records still override the exact `(policy, target)` parameter record.
An explicit scenario record retains its existing target contract; it does not
inherit permission to disappear from a dormant preset preference. Duplicate
keys within a layer remain invalid regardless of applicability. No implicit
field merge, source fallback or fanout is introduced.

Keep the accepted [single preparation/execution graph](owned-preparation-readiness-proposal.md).
Generated count can reuse a declared count policy once its target exists.
Requested enabled state needs an explicit consumer at the correct phase; a
usage program must not synthesize its own provider or write a gate that it must
already pass to run. Full DPS inclusion needs a defined reporting/aggregation
consumer, not a physical Gem field or a silently modified query. Those are
separate data/consumer acceptance gates, not defaults conferred by this proposal.

Compiled reuse keys must cover the chosen build content, preference contents,
applicability and effective scenario, with existing data identities for binding.
Candidate selection changes must invalidate any prior applicability proof.
Perform bounded indexing/validation during construction; parallel workers use
immutable compiled inputs and private scratch without XML or PoB execution.

## Implementation and acceptance gates

1. [ ] Owner chooses A or B. Do not relax `PresetUsageScope` as an interim fix.
2. [ ] Specify the versioned complete/draft representation and shared validation.
   Preserve omitted legacy bytes, identities, strict local ownership, direct
   request behavior and scenario semantics; reject explicit null/unknown fields.
   Add data-aware validation for all stored new-contract preferences, including
   their global project root identities and inactive parameter schemas. Wire
   names and migration details need review before implementation.
3. [ ] Add exact generated source-to-provider correspondences and declared native
   paths. Preserve Partial owner/program/grant coverage until individually
   established. Authored-membership closure does not supply this authority.
4. [ ] Extend one composition/finalization path with explicit applicability
   outcomes. Test two skill presets with distinct intent for one provider,
   independent equipment/tree selections, same-definition repeated providers,
   missing/deleted/foreign roots, wrong modifier ownership, partial inventories,
   inactive/unknown activation, exact scenario overrides, bounded work, round
   trips and cache/scratch reuse across candidate changes.
5. [ ] Give each saved generated setting a reviewed semantic disposition. Reuse
   source lifecycle observations without choosing a canonical numerical phase.
   Keep unknown settings and incomplete preference inventories Pending; prove
   enabled/count/group override/Full DPS behavior separately where relevant.
6. [ ] Rebuild all five original imports, all archived variants and all 110 query
   rows. Preserve raw settings, ownership and unrelated diagnostics. Report
   exact additions and blockers; no complete-build or numerical parity claim
   follows from storing generated preferences.

The immediate decision is where this intent belongs and how proven source
nonselection behaves. It does not change complete-build execution coverage or
authorize dropping generated settings to close the current usage inventory.
