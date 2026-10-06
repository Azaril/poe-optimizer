# Implementation note: local item facts before Skill preparation

**Status:** Implemented under the accepted readiness model. Data, private/public Engine, real-data Offering, historical native and authoring tests pass, along with strict workspace Clippy and WASM compilation. Exact pushed-head hosted CI remains separate.
**Date:** 2026-10-05.
**Boundary:** Explicitly classified local item `Derive` outputs before Skill preparation under stages V4.

## Concrete blocker

The accepted [readiness design](owned-preparation-readiness-proposal.md) uses one
occurrence graph with Structural, Preparation and Execution requirements. The
accepted [source-property design](owned-source-property-preparation-proposal.md)
then binds exact physical sources and retained support positions before final
generated-input projection. Neither design requires item calculations to live in
a separate evaluator.

The V1–V3 contract has a narrower first boundary.
`owned_stages/readiness.rs::validate_effect` permits `PreparationFacts` derivation,
contribution and capability writes only at Actor, Skill or SupportOrigin scope.
The real item programs needed by Pain Offering use additional local scopes:

| Existing producer | Actual local outputs |
| --- | --- |
| Crown `1f1c` / Solar Amulet `2343`, `catalyst-inputs` | EquipmentUse Option `0a19` and percentage Quantity `0a1a` |
| The same templates, `amulet-copy-eligibility` | EquipmentUse Boolean `32e3` |
| Modifier `30ca`, catalyst/ordered-magnitude/corruption/effective-amount programs | Modifier Quantities `0a1b`, `253d`, `2541`, `295b` |
| Modifier `30ca`, direct and Amulet-copy delivery | Existing allowed Player contribution channel `30ab` |

Classifying the first three rows as Execution leaves genuine late dependencies
when `effective-input-preparation` reads the resulting level contributions.
Classifying them as PreparationFacts under V1–V3 fails Data validation. Merely
changing stage order cannot fix either failure.

This is more than a numeric-only permission: the authentic catalyst and placement
inputs include Option and Boolean Stats. Conversely, the blocker does not require
new early capabilities, local contribution streams or modifier-transform writers.

The actual item/template/Gem owners still have Partial inventories. Resolving the
local output restriction does not complete them, establish the required
pre-Amulet snapshot `32e4`, or make the full Original05 graph executable.

## Implemented bounded change

Explicit stage version V4 permits the
existing `PreparationFacts` role to use **only `Derive`** for these additional
local destinations:

- `Current` in an EquipmentUse program, with an EquipmentUse Stat;
- the exact `Modifier` of an EquipmentUse program, with a Modifier Stat.
- `Current` in a Modifier program, with a Modifier Stat.

Keep the existing compiler's program-owner, parameter, occurrence and destination
authority. This is no permission to address another item, another modifier,
`PropertyOwner`, or an arbitrary actor. Permit the known computed Stat value types
needed by these facts—Quantity, Integer, Boolean and Option—with their ordinary
schema/type validation. Do not widen the current combined
`Derive | Contribute | Capability` branch wholesale.

The new permission must still require:

1. An explicit early phase, a Complete program owner and complete program classification.
2. Exact potential output channels, including every conditional write.
3. No overlap with an Execution channel and no competing potential early scalar writer.
4. Valid existing raw-input and equipment/modifier ownership.
5. The unchanged concrete dependency and activation proof, including later contributors and transforms.

No new rule expression, effect kind, raw-input storage, contribution reducer or
runtime occurrence is proposed. The same ordinary item programs and source
assembly execute in the existing prefix/suffix graph with one decreasing budget
and worker-local scratch.

This is a bounded implementation of the accepted architecture, with an
explicit version for its added preparation authority. It introduces no new
ownership choice or second calculation model. V1–V3 must not silently gain the
permission. No gameplay coefficient or complete item/source census is authorized
by this extension.

## Existing proof that must remain intact

Data already checks complete owner programs, exact outputs, early/Execution
channel overlap and potential scalar writer conflicts in
`crates/poe-optimizer-data/src/owned_stages/readiness.rs`.

Engine's `owned_plan/compile/readiness.rs` already indexes scalar values,
contributions and modifier transforms separately. It checks every declared read,
implicit gate, potential support template and source-property program, including
unselected branches. A later contributor or transform cannot become an early
zero/identity merely because it is inactive in one candidate. Prefix/suffix stage
validation remains another independent check.

Missing producers and Partial owner/contributor inventories must continue to
yield unavailable results. The extension does not bless an empty incoming
inventory or promote a final metric to early authority.

## Alternatives and tradeoffs

| Option | Benefit | Cost and limit |
| --- | --- | --- |
| **A. Versioned local derivations, recommended** | Runs the actual item and Gem programs together through the accepted graph; reuses existing scope and phase proofs. | Adds public early-write authority and requires compatibility, negative and integration tests. It does not solve actual Partial inventories. |
| **B. Keep the boundary and test components separately** | No public change; source assembly can be checked against an explicitly supplied pre-support boundary. | Cannot claim item-to-generated-input integration. Passing copied intermediate values from a separate plan is a test adapter, not the production solution. |
| Broad item capabilities/contributions or a second preparation evaluator | Could bypass this immediate restriction. | Unnecessary scope, duplicated ownership/state or weaker authority. Not recommended. |

An independently source-backed early modifier-transform family may need a later
extension. Its absence is not grounds to authorize every transform now. If the
current selected component actually needs one, report that dependency rather than
erase or substitute its stream.

## Version and implementation boundary

- Core: add `OWNED_EVALUATION_STAGES_V4`; the DTO fields and rule operations stay
  unchanged. Preserve the stages V1 and operations V14 defaults.
- Data: select `owned-evaluation-stages-v4` as the new stage identity domain and
  gate the narrow local `Derive` arm on exact V4. V4 requires a known source-property
  operations version, currently V18–V20, and explicit readiness metadata. Source
  roles and receiving V3 accept checked stages V3 or V4. V1–V3 retain their
  historical authority and identity domains; unknown versions fail closed.
- Engine: reuse the existing binding and phase proof. Any required adjustment
  must preserve exact modifier/equipment identities and all implicit gates;
  no alternate calculation loop or fixture-only bypass.
- Plan identity: Engine already includes the checked stage identity in
  `owned-readiness-effect-plan-v1`. Therefore V4 changes the plan commitment
  without a new rule operation, effect-plan algorithm domain or migration family.
- Publication: retain schema V6/operations V20. Offering uses explicit full-endpoint
  owner replacement; its unpublished V5 append-only stage supplies checked
  dependency rebinding without changing migration permissions. A later evaluation bundle must carry
  the exact V4 stages identity; no older packet or rule semantics is reinterpreted.
- Game data: opt in through the checked successor; preserve actual owner gaps,
  query contracts and all unrelated rule bodies. A finite test may close only its
  explicitly admitted component and must test the actual Partial release refusal.

The affected production files are Core `owned_stages.rs`, Data `owned_stages.rs`,
Data `owned_stages/readiness.rs`, and the stage gate in Data
`owned_support_receiving.rs`. No Engine execution change is required.

Existing test homes are Data's
`tests/support/owned_readiness.rs`, Engine's public
`tests/owned_preparation_readiness.rs`, and its private
`owned_plan/compile/readiness_tests.rs`. The current Offering integration can then
serve as the real-data witness rather than a second synthetic implementation.

## Validation status and remaining gates

`runs/owned-offering-item-readiness-data-03.log` records 25 stage/readiness tests
and 22 support-receiving tests passing, including V4 local typed derivations,
historical version restrictions, exact outputs, owner closure, potential writers
and the explicit receiving-V3/stages-V4 pairing. The private Engine readiness
suite passes three tests in `runs/owned-offering-item-readiness-engine-01.log`,
retaining occurrence identity and rejecting late scalar, contribution, transform
and activation dependencies. No Engine execution behavior changed.

All seven actual Offering component tests pass in 8.98 seconds; see
`runs/owned-offering-native-03.log`. They run published item/copy programs and
canonical rolls through source assembly into generated inputs in one graph,
including exact refusals, independent occurrences, A/B/A scratch and four-worker
Rayon replay. Native01's fixture omitted receiving V3; native02 scheduled generated
predicate facts before their exact entering grant. Correcting those explicit
fixture declarations resolved the failures without changing production checks.
Source04 and publication03 pass independently. Public Engine readiness (14 tests)
and source properties (15 tests), including budget recovery and A/B/A/Rayon,
pass in `runs/owned-offering-engine-regressions-01.log`. Twenty-seven historical
native tests across Action, Encroaching, Magnified, Prolonged and Rapid pass in
`runs/owned-offering-native-regressions-01.log`. Bidding's five native tests and
nine ordinary authoring tests also pass in `runs/owned-offering-bidding-regressions-01.log`
and `runs/owned-offering-authored-regressions-01.log`. Strict workspace
all-target/all-feature Clippy passes in `runs/owned-offering-clippy-03.log`.
Minimal Engine/Data and full Core/Data/Engine/Import WASM library compilation
pass in `runs/owned-offering-wasm-minimal-01.log` and `runs/owned-offering-wasm-01.log`;
these are not new WASM numerical replays. Hosted CI remains a separate gate.
After test-helper cleanup, Amulet's nine native tests pass in
`runs/owned-offering-amulet-regressions-01.log` and the Offering seven-test replay
passes in 9.22 seconds in `runs/owned-offering-native-04.log`.
This paragraph records the initial Offering checkpoint. The subsequent
[Amulet snapshot packet](../data/owned/poe2/3887ae68/amulet-bonus-snapshot/README.md)
now supplies the real `32e4` producer, and the
[ordinary routing packet](../data/owned/poe2/3887ae68/ordinary-item-routing/README.md)
checks it alongside independent recipient applicability. Complete incoming
inventories remain unresolved independently of those later component results.

Keep the following checks as the validation contract:

- V4 positives for local EquipmentUse Option/Boolean/Quantity and exact
  Modifier numerical derivations; V1–V3 and unknown versions reject the same inputs.
- Negative local contribution/capability/transform writes, foreign destination,
  wrong Stat target/type, missing output declaration, Partial owner, competing
  writer, and early/Execution overlap.
- A single public-plan item-to-Gem-to-generated-parameter example with two
  independent items and physical sources; remove each genuine input and preserve
  exact missing/invalid diagnostic distinctions.
- Later scalar, contribution, transform and activation dependencies reject even
  through a false conditional branch or an unselected potential support.
- Preserve the existing wider A/B/A, private Rayon, budget failure/recovery and
  native/WASM proofs; do not count them as new V4 assertions unless rerun or
  extended for this slice. The new Offering component should add its exact
  item-derived input replay/isolation checks and report which checks actually ran.
- Historical stage/version fixtures and receipts remain unchanged. The historical
  missing-`32e4` control must still refuse preparation; the subsequent published
  producer does not close actual Partial coverage in the real release.

The decision is limited to early local derivation authority. It does not decide
participation ownership, source lifecycle policy, legal fractional game inputs,
table recovery behavior, or final damage/duration/cost formulas.
