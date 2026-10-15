# Multiple projections from one configuration control

**Status:** Accepted by the owner, 2026-10-14; portable Import contract implemented
and validated. This adds no Core or Engine authority and does not complete a
configuration inventory. Distance data has not yet been published under this
contract.

**Decision:** The existing configuration input adapter now has explicit selection
rules for each semantic output. It checks each shared numeric control once,
produces typed Scenario assumptions, and uses the existing
[configuration disposition owner](owned-configuration-dispositions-proposal.md)
to account for every output. The current format is extended directly; there is
no V4 compatibility branch, parallel importer or second source model.

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
Existing retained source controls cover these selection differences; their
bounded scope and remaining validation limits are recorded below.

Before this change, `ConfigurationNumericInput` fallback preserved zero but
retained only the winning value. `ConfigurationDefaultInput` admitted Input or
a constructor value, not a consumed Placeholder, and compilation required each
`source_name` to be unique across projections. Two rows could not represent both
consumers faithfully. Ignoring the Placeholder would discard a live input;
publishing one projection cannot account for this entire leaf.

## Implemented bounded extension

`ConfigurationDefaultInput` reuses the existing typed adapter, source frame
checks and value codecs. A control declares its shared input/placeholder syntax
and reviewed constructor defaults; each output declares its own target,
external-input ID and explicit `ConfigurationValueSelection`:

- **`input_then_placeholder`:** authored Input, including zero, otherwise
  effective Placeholder.
- **`nonzero_input_then_placeholder`:** nonzero Input, otherwise effective
  Placeholder.
- **`input_or_constructor`:** the existing default mode, whose serialized field
  is omitted and whose Input-or-constructor behavior remains unchanged.

An effective Placeholder is its checked saved value, or its reviewed constructor
value on proven absence. Malformed, duplicate or unsupported source never selects
a default. Multiple projections must agree on their numeric codec, destination
schema and constructor value; the compiled grammar fixes the remaining source
recipe semantics. A shared census validates both saved lanes, including an
unselected lane, and shares failures as well as successful values. These new
modes accept numeric Integer or Quantity codecs only.

For distance, the second output is a **callback candidate**, not an emitted
modifier. A known zero candidate lets an injected native predicate suppress the
contribution; it does not manufacture a zero contribution or prove its supplier
inventory complete. Independently reconcile the source `Effective` gate to owned
scenario semantics before emitting a contribution; this proposal imports no UI
or calculation-mode flag. Missing output evidence is unavailable, not false or zero.
Source widget dispatch stays in the import boundary. Native rules consume owned
quantities and conditions, with no PoB control types, UI state or Lua truthiness.

The source gate is a calculation-view policy, not a game state:
`CalcSetup.lua:804–825` derives it from CALCS `misc_buffMode`, with other modes
using Effective; `CalcSetup.lua:111–113` installs the database condition.
`CalcsTab.lua:13–16,170–175` defines Unbuffed/Buffed/Combat/Effective views while
the sidebar uses Effective. Selecting owned enemy assumptions must remain
independent of that UI. Approving the projection selector does not establish
an automatic mapping for these source view modes.

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

The owner approved the extension: preserve the observed meanings within the
existing model, including explicit multiple-output and zero-selection authority.
The retained source controls and their exact data bindings gate publication of
the raw distance inputs. Native applicability reconciliation separately gates
emission of distance contributions; accounting for raw input ownership does not
establish downstream mechanic coverage.

## Retained source evidence and implementation validation

No new PoB execution was performed for this Import change. The existing
`owned_minion_accuracy_source` witness includes a distance subset of 12 controls
and 14 rows: Input 0 / Placeholder 80, saved Placeholder 0, constructor 20,
negative and fractional inputs, and CALCS Effective/Combat behavior. Its JIT-off
and JIT-on packets have identical bytes, SHA-256
`a61e14e329084002bdc1b1ef9034f33e62574e0982619a9a6a5dcb4554786890`, as
recorded in `data/owned/poe2/3887ae68/minion-accuracy/authoring.json`.
The full witness has 31 cases and 32 complete-load attempts per JIT mode; these
are broader witness totals, not additional distance controls. There is no plain
uninstrumented control or independent repetition of each distance variant in
that witness. It is neither whole-build parity nor supplier-completeness proof.

The portable contract passed 78 focused checks and the full Import library and
normalization regression: 191 library tests plus 346 normalization tests. Strict
Clippy passed for both targets. Logs are
`runs/owned-configuration-projections-tests-01.log`,
`runs/owned-configuration-projections-regression-01.log` and
`runs/owned-configuration-projections-clippy-01.log`.
The new controls cover both numeric codecs, zero constructors, shared-census
order independence, losing-lane validation, source scopes, bounded work,
projection identity and completeness, and tampered output value/target/count.
Existing default policies retain their prior behavior.

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

Before distance publication, bind its injected definitions and constructor to
the authenticated catalogue and retained controls for both consumers; identify
any additional evidence needed without treating the existing witness as new
acquisition. Native contribution predicates still require ordinary and prepared
execution, reused workers and Rayon tests after the source view gate has been
reconciled. Source agreement, input accounting, numerical supplier coverage and
complete-build evaluation remain separate gates.
