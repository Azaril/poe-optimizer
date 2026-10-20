# Proposal: scenario-owned usage intent resolved to exact occurrences

**Status:** Accepted by the project owner on 2026-10-19. Core/Data contracts and
native resolution pass focused validation. The current-format release reimports
all five unchanged originals; CLI and retained-evidence checks pass.
Strict lint and owned-library boundary checks also pass.
Configuration projection/accounting passes 627 targeted Import tests and the
all-five unchanged-draft rebuild. Targeted strict lint and native library
boundaries and repaired CLI regression suites pass. Hosted CI verification
remains pending publication.
Actual control publication remains pending.
**Date:** 2026-10-15.
**Decider:** Project owner.

## Problem at proposal time

Saved configuration can describe skill usage independently of the selected skill
preset. Pinned PoB's `sigilOfPowerStages` and `conditionCorruptingCryStages` are
examples. Copying their values into every skill preset would couple two axes
that the owned project deliberately keeps independent.

Original05 has one ConfigSet and six saved SkillSets. Its saved stage values are
Placeholders 1 and 10. The bounded preflight finds no consumer in those saved Gem
rows, declared additional effects or item texts. This is not a complete native
consumer census, and it does not make the controls presentation-only: retained
source02 records both callbacks executing twice. Sigil appends a Player stage
multiplier; Corrupting Cry appends value minus one with a source calculation-view
condition. Widget visibility does not prevent those calls.

The actual skill consumers differ. Corrupting Cry consumes the count on generated
triggered effects, rather than the support Gem container. Sigil consumes stages
in its global buff; a modifier family also scales the buff's critical chance by
those stages. They need exact skill/effect correspondence and existing effect
application/stacking semantics, rather than a global Player subtotal.

Before this change, the native runtime supported exact `UsagePolicySelection`
targets. Preset-owned intent supported dormant exact providers, and Scenario usage
provided exact overrides, without a dormant expansion contract. Import usage read
only occurrence/group frames, and configuration accounting had no usage-output
receipt. Core's Skill external-assumption type does not provide
an alternative: external-input reads targeted at a Skill are unsupported.
Skill-context programs can read Player externals, but that transport does not
establish exact skill ownership or applicability.

## Recommended ownership

Keep these settings as **typed Scenario-owned usage intent**. During normal
definition-aware build preparation, resolve an explicitly declared applicability
policy against the composed build's existing occurrence graph and produce the
same exact usage selections consumed today. The runtime representation must not
contain ConfigSet, source names, Lua records or UI selectors.

The first supported resolution policy should explicitly mean **all eligible
exact occurrences**, rather than an implicit first match. Injected applicability
data must identify the eligible skill/effect definitions and permitted supplying
relations. Definition equality or a matching display name alone cannot grant
recipient authority. Future selectors need their own semantics; this does not
introduce an unrestricted query language.

- Preserve independent Scenario and skill/equipment/tree selections. Resolve
  again when any relevant selected source, definition or policy changes.
- Reuse existing exact Skill and provider-path identities, including support-
  generated effects and item/tree grants. Create no second occurrence registry.
- A complete eligible-source census with no matching source leaves intent
  dormant. It neither creates a skill nor discards the saved value. Incomplete
  discovery, an unresolved provider or ambiguous receiving authority remains
  unavailable; zero matches cannot be inferred from failed lookup.
- Every eligible occurrence receives its own exact selection. Repeated sources
  remain distinct; effect non-stacking still belongs to effect applications.
  The intent never activates a disabled source or constructs a queried Action.
- Keep the existing whole-policy-record replacement model. Proposed precedence
  is selected preset preference, then applicable Scenario intent, then an explicit
  exact Scenario override. Reject overlapping defaults for the same policy and
  exact target; do not choose by declaration order or local ID.
- Validate stored policy/parameters even when dormant or overridden. Materialize
  only checked exact selections, with provenance explaining applied, dormant or
  overridden intent. Preserve strict behavior for existing explicit exact
  targets; do not reinterpret a stale exact target as a default selector.
- Bound expansion work and include the policy, selected build/Scenario and
  definition identities in resolved-plan/cache identity. An exact override cannot
  hide an incomplete eligible-source census for an applicable intent.

Core remains source independent and does not acquire a Data dependency. Source
interpretation stays in Import; definition-aware resolution belongs at the
existing preparation/binding boundary. The concrete validation stages must
preserve preparation readiness and participation, and prevent circular resolution
through a mechanic that itself needs the unresolved usage.

## Alternatives and trade-off

| Option | Benefit | Cost |
| --- | --- | --- |
| Scenario-owned intent resolved against the composed build | Preserves independent axes and dormant settings; supports candidate changes through the same native graph. | Adds an explicit applicability/completeness contract and a checked configuration-to-usage adapter. |
| Import only into explicitly selected exact consumers | Reuses the current final usage shape with less initial model work. | Requires re-resolution or user repair when sources/presets change, and cannot itself preserve an unbound dormant setting. |

Publishing raw Player assumptions could preserve transport temporarily, but would
not solve skill ownership or composition. It would create another migration for
these controls, so it is not the recommended final model.

## Implementation and validation gates

1. Accepted: Scenario ownership, explicit all-eligible policy and replacement
   precedence. Exact Scenario overrides follow resolved intent; preset
   preferences supply the earlier default.
2. Implemented: bounded checked applicability over existing definitions,
   exact occurrence discovery and usage schemas. No new effect or aggregation
   system, native source-language behavior or old-format compatibility branch.
3. Implemented and tested: the existing import projection/accounting path includes
   scoped usage-intent receipts. Input/Placeholder selection and malformed losing
   lanes are preserved; one successful destination cannot retire an incompletely
   accounted leaf.
4. Review numeric meanings separately. Sigil preserves Input zero; Corrupting Cry
   selects nonzero Input before Placeholder. Its callback subtraction, limit and
   source view condition are not automatically native gameplay semantics. A raw
   candidate, a skipped callback and an actual zero effect remain distinct.
5. Implemented and tested: cross two Scenarios with two independent skill presets;
   cover no consumers,
   repeated consumers, generated children, item grants, exact overrides, disabled
   sources, malformed dormant values, missing providers and incomplete census.
   Fresh/reused/parallel results and all five unchanged imports are verified.

No source leaf, input inventory or native coverage is completed by this proposal.
Original05's two stage leaves remain Pending until actual control data is published
and its exact source-consumer scope is proved.

## Implementation resume map, October 20

The concrete current contract retains `BuildInput.usage_preferences`,
`ScenarioInput.usage_intents` and exact `ScenarioInput.usage` independently.
Each intent contains a typed policy and parameter assignments. The required
`UsagePolicySchema.scenario_skill_applicability` field is either explicitly null
(no expansion authority), or a Complete/Partial declared set of direct-authored
Skill identities and exact supplied Skill/Grant/SkillGrant relations. This is one
bounded allowlist, not a second selector language.

The schema package remains the current version 7, changed in place. A stale
UsagePolicy record missing its required field is rejected; its containing
package's content identity changes on reauthoring. Input and draft envelopes are
version 5 and request identities use `owned-request-v2`. There is no old-format
runtime adapter. The importer initially adds a separate Pending intent inventory:
an unconverted configuration cannot certify a complete empty set of intents.
Current fixtures and release dependencies have been explicitly rebuilt with
inverse checks and unchanged source observations.

The implementation uses these existing seams:

1. Implemented: `owned_preset_intent::compose_selected` retains preset preferences
   and exact Scenario overrides independently. Definition-aware preparation inserts
   Scenario intent between them. Dormant and overridden records still validate;
   one resolved list retains source and replacement provenance.
2. Implemented: Scenario records and UsagePolicy schema validation use bounded
   all-eligible Skill applicability. Both definition and supplying relation are
   checked through existing SkillGrant/GrantTarget data; definition equality is
   insufficient. Existing exact Actor/Action usage remains supported.
3. Implemented: Engine discovery and `skill_supplies` share structural traversal
   with inactive support discovery. Disabled and off-loadout sources remain in
   the census without being enabled. Topology resolves before usage program
   binding, independently of participation, final level or computed usage values.
   `compile/usage.rs` and `compile/reads.rs` consume the same resolved list.
4. Implemented and tested: the current Import usage projection and
   configuration disposition receipts include Scenario intent destinations.
   Preserve source lanes and independent axes; complete one control only after
   all of its declared destinations are accounted for, including unresolved
   projections that have emitted no output.

Validation reused Core preset-intent, Engine usage/participation/support-generated/
readiness and Import numeric-usage/configuration tests. Added tests passed for
two-Scenario/two-preset crosses, repeated and generated recipients, overridden
malformed intent, overlap rejection, complete-empty versus incomplete discovery,
and fresh/reused/Rayon equivalence.
No additional architecture decision was identified for this implementation.
The reviewed source meanings below do not establish complete gameplay coverage.
Publish the actual controls only after exact grant/supply applicability,
recipient/stacking behavior and all-five unchanged-import validation pass.

Source anchors at pin `3887ae68`: `ConfigOptions.lua:338–342,599–601`,
`sup_str.lua:1813–1817,1974–1978,5453–5457`,
`act_int.lua:17660–17664`, `ModParser.lua:6213`.

### Configuration import slice

The existing usage-import policy now includes configuration-frame rows; the
importer stores typed policy/parameter intent on its Scenario and never resolves
native recipients or copies values across skill presets. Reuse destination-neutral
numeric selection from the numeric Reward adapter: ordinary recipe precedence
preserves an authored zero for Sigil, whereas the reviewed count reader selects
nonzero Input followed by nonzero Placeholder. Validate both lanes, including
losing malformed values and duplicates. The default-input adapter is unsuitable
because its constructor defaults and zero-placeholder semantics differ.

Use the existing configuration disposition/accounting path with checked
policy/parameter receipts. A receipt must match the complete unique materialized
intent, its exact Config scope, typed values and source links; all projections
of a shared leaf must be accounted for before retirement. Rebind the usage-policy
dependency with the existing release dependency order. Successful individual
projections do not close either Scenario usage inventory or overall configuration
coverage. The full imported-control census remains an independent obligation.

The implementation extends `UsageInputPolicy::PobOccurrenceUsageV3` in place,
shares occurrence-neutral policy/parameter validation and extracts numeric
selection from the Reward adapter into `owned_configuration_numeric`. Typed
intents materialize after historical issue allocation and before
`configuration_dispositions::account`. Existing private receipts and
all-projection accounting own retirement. The usage-policy identity is bound to
configuration dispositions and rebound in the existing release dependency order.

Passing tests cover two ConfigSets crossed with independent presets; preserved zero
versus explicit nonzero selection; malformed losing lanes and duplicate source rows;
wrong units/ranges and missing required parameters; absent, duplicate or altered
materialized intents/provenance; shared scalar/usage destination completeness;
stale dependencies and bounded work. Numeric Reward regressions and all-five
unchanged-import checks also passed after extracting shared selection.

No additional ownership decision was identified by this review. Corrupting Cry's
gameplay calculation and source `Effective` condition are separate from numeric
transport; the source review below records their finite supported meaning.

### Reviewed source meanings and remaining data work, October 20

The cited files were checked against the pinned source manifest. ConfigTab's
`countAllowZero` selects Input then Placeholder with zero preserved; `count` skips
zero in both lanes. The callback does not consult UI visibility. Sigil's saved
stage count is retained unchanged; its skill data caps the per-stage Spell MORE
effect at four, with a separate possible per-stage critical-chance contribution.
Corrupting Cry stores the positive count minus one, then its triggered child
applies the extra-stack multiplier with an upper bound of nine. On the reviewed
positive-integer domain this represents at most ten stacks. Neither default
values nor negative/fractional-stage behavior are established by this review.
Both-zero Cry lanes remain unresolved, not one stack.

The source's `Effective` switch comes from calculation view mode; it is not an
Actor gameplay property. Keep it in reference-view evidence, outside native inputs.
Existing checked numeric domains and arithmetic can express the reviewed formulas,
with caps and level-dependent values supplied as data. No Lua epsilon rounding or
display-name stage routing is needed for the reviewed integer domain.

The current release has no stage UsagePolicy. Sigil and the three triggered Cry
Skill definitions are still Unmapped, and their supplying Gem declarations need
real grant/supply work. Do not publish an empty Complete applicability list to
claim these settings dormant. Actual control publication must establish those
relations, preserve Partial coverage where required, and address Sigil recipients
and stacking and Cry damage-over-time/reporting consumers separately.

Additional source anchors: `ConfigTab.lua:1177–1188`,
`CalcSetup.lua:111–113,803–825`, `ModStore.lua:519–540`,
`CalcActiveSkill.lua:870–878,1033–1103`, `CalcOffence.lua:6169–6174`.
All five saved originals use Sigil Placeholder 1 and Cry Placeholder 10; those
observations are fixtures, not production defaults.
