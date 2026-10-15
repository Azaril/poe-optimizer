# Multiple projections from one configuration control

**Status:** Pending owner review, 2026-10-14. No implementation is authorized by
this proposal. This extends the owned Import policy; it adds no Core or Engine
authority and does not complete a configuration inventory.

**Recommendation:** Extend the existing configuration input adapter with explicit
selection rules for each semantic output. Parse each saved control once, produce
typed Scenario assumptions, and use the existing
[configuration disposition owner](owned-configuration-dispositions-proposal.md)
to account for every output. Update the current format and rebuild its data;
do not add a V4 compatibility branch, parallel importer or second source model.

## The concrete problem

Pinned PoB revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` uses
`enemyDistance` in two different ways:

| Consumer | Source behavior |
| --- | --- |
| Skill range | `CalcActiveSkill.lua:674,688` selects Input, otherwise Placeholder, and supplies `skillCfg.skillDist` in Effective mode. Explicit Input **0 stays 0**. `ModStore.lua:722–739` uses this value for distance ramps. |
| Distance modifier | `ConfigOptions.lua:1639–1641` declares a `count` control and emits `Multiplier:enemyDistance` BASE with an `Effective` condition. `ConfigTab.lua:1184–1187` skips Input **0**, falls back to Placeholder, and calls no callback if that candidate is also 0. |

Fresh configuration construction installs Placeholder **20** from the injected
catalogue default (`ConfigTab.lua:1317–1333`); XML can replace it
(`ConfigTab.lua:907–913`). Ten source distance units represent one metre.
The modifier also affects presence (`CalcPerform.lua:524–527`), Player defence
(`CalcDefence.lua:1476–1477`) and attack accuracy
(`CalcOffence.lua:2715–2727`). Neither consumer is presentation-only.

Thus Input **0** with Placeholder **80** gives skill range **0** and a distance
modifier candidate **80**. Original05 has no authored Input and saves
`<Placeholder number="20" name="enemyDistance"/>` in
`tests/fixtures/builds/breadth-20260908/build-05.xml:554`, source ordinal 444.
Its equality at 20 does not prove the two meanings interchangeable.
These are inspected source semantics; new controlled distance observations have
not yet established numerical parity.

The current `ConfigurationNumericInput` fallback preserves zero but retains only
the winning value. `ConfigurationDefaultInput` admits Input or a constructor
value, not a consumed Placeholder. Configuration compilation also requires each
`source_name` to be unique across projections. Two existing rows therefore cannot
represent both consumers faithfully, and ignoring the Placeholder would discard
a live input. Publishing one projection cannot account for this entire leaf.

## Proposed bounded extension

Reuse the existing typed adapter, source frame checks and value codecs. A control
declares its shared input/placeholder syntax and reviewed constructor defaults;
each output declares its own target, external-input ID and explicit selection:

- **Preserve zero:** authored Input, otherwise effective Placeholder.
- **Skip zero Input:** nonzero Input, otherwise effective Placeholder.

An effective Placeholder is its checked saved value, or its reviewed constructor
value on proven absence. Malformed, duplicate or unsupported source never selects
a default. The precise DTO spelling remains an implementation detail of this
decision; there should be one maintained selection path.

For distance, the second output is a **callback candidate**, not an emitted
modifier. A known zero candidate lets an injected native predicate suppress the
contribution; it does not manufacture a zero contribution or prove its supplier
inventory complete. The Effective applicability condition remains a separate
native dependency. Missing output evidence is unavailable, not false or zero.
Source widget dispatch stays in the import boundary. Native rules consume owned
quantities and conditions, with no PoB control types, UI state or Lua truthiness.

The accepted disposition contract already supports several
`Adapter(ScalarInput { input })` destinations for one leaf. Require a unique
checked receipt for **every** declared destination before removing that leaf's
fallback obligation. Preserve exact source scope, origin, selector, target and
typed output. For Input 0 / Placeholder 80, each leaf contributes to one projection
and is checked as unselected for the other; both responsibilities must survive.
Do not infer nonselection merely because another projection succeeded.

Output identity is checked by target and external-input ID; source names identify
the shared census rather than prohibiting multiple semantic outputs. Assign
distinct external-input IDs to projections from the same leaf, so the existing
`ScalarInput` destination identifies one checked output unambiguously. Reusing
an ID on another target must not rely on receipt order. Materialize one
source-to-Scenario link, not duplicate links per projection. Keep malformed
or missing receipts pending, preserve unrelated links, and retire no containing
inventory from these leaf receipts.

## Alternative and decision

Normalize a single owned distance and treat the inconsistent source consumer as
a PoB defect. This simplifies the native meaning but changes parity for explicit
zero controls. The code proves a discrepancy; it does not establish which game
mechanic is intended. We currently lack sufficient mechanics evidence to choose
this alternative confidently. Do not silently declare either consumer correct.

The recommended extension preserves the observed meanings within the existing
model. Its multiple-output and zero-selection authority changes the public
Import policy, so the owner should approve it before implementation.

## Validation required before publication

| Control or mutation | Required result |
| --- | --- |
| Input absent, Placeholder 20 | Both outputs 20; exact Placeholder provenance. |
| Input 0, Placeholder 80 | Range 0; callback candidate 80; both leaf roles accounted. |
| Input nonzero, conflicting Placeholder | Both select Input; checked nonselection of Placeholder for both. |
| Input absent or 0, Placeholder 0 | Range 0 and candidate 0; no fabricated distance contribution. |
| Both saved lanes absent | Reviewed fresh constructor 20; malformed or unproven frame remains pending. |
| Negative, fractional and boundary values | Preserve admitted numeric values; consumer clamps and units remain separate. |
| Duplicate/wrong-lane/malformed value, wrong source scope | No default repair or partial receipt used to retire the leaf. |
| Remove one projection, alter output target/value, duplicate its receipt/link | Refuse leaf accounting; retain the live obligation and unrelated links. |
| Change mode, active ConfigSet or selected preset | Recompute exact applicability and ownership; no stale receipt reuse. |
| Reimport all five originals | Preserve unrelated data, exact source correspondence and all 110 queries; record only the reviewed leaf/input delta. |

Authenticate the existing catalogue/constructor bodies and add bounded original
source controls for both consumer paths. Test the native predicates through
ordinary and prepared execution, reused workers and Rayon. Source agreement,
input accounting, numerical supplier coverage and complete-build evaluation
remain separate gates.
