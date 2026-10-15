# Proposal: scenario-owned usage intent resolved to exact occurrences

**Status:** Proposed; no runtime or import contract implemented.
**Date:** 2026-10-15.
**Decider:** Project owner.

## Problem and current behavior

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

The current native runtime supports exact `UsagePolicySelection` targets.
Preset-owned intent supports dormant exact providers, and Scenario usage provides
exact overrides. However, Scenario usage has no dormant expansion contract;
Import usage reads only occurrence/group frames; and configuration accounting has
no usage-output receipt. Core's Skill external-assumption type does not provide
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

1. Review the ownership, explicit all-eligible policy and replacement precedence.
2. Specify the smallest checked applicability contract over existing definitions,
   exact occurrence discovery and usage schemas. No new effect or aggregation
   system, native source-language behavior or old-format compatibility branch.
3. Extend the existing import projection/accounting path with actual scoped usage
   intent receipts. Preserve Input/Placeholder selection and malformed losing
   lanes; one successful destination cannot retire an incompletely accounted leaf.
4. Review numeric meanings separately. Sigil preserves Input zero; Corrupting Cry
   selects nonzero Input before Placeholder. Its callback subtraction, limit and
   source view condition are not automatically native gameplay semantics. A raw
   candidate, a skipped callback and an actual zero effect remain distinct.
5. Cross two Scenarios with two independent skill presets; test no consumers,
   repeated consumers, generated children, item grants, exact overrides, disabled
   sources, malformed dormant values, missing providers and incomplete census.
   Verify fresh/reused/parallel results and all five unchanged imports.

No source leaf, input inventory or native coverage is completed by this proposal.
Original05's two stage leaves remain Pending until the chosen path is implemented
and its exact source-consumer scope is proved.

Source anchors at pin `3887ae68`: `ConfigOptions.lua:338–342,599–601`,
`sup_str.lua:1813–1817,1974–1978,5453–5457`,
`act_int.lua:17660–17664`, `ModParser.lua:6213`.
