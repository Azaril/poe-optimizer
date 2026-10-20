# ADR: support-generated Skill supply and execution

**Status:** Proposed — not approved or implemented.
**Date:** 2026-10-20.
**Decider:** Project owner.

**Recommendation:** put a support's generated Skill on the existing grant graph.
Data must declare which existing Actor owns it and what must be true before it
executes. Keep one child per physical support assignment and declared supply,
with no automatic expansion per receiver. Add only the supplier authority needed
by a proven real case; reject unsupported Actor bindings and invocation rules.

## Context

The subsequent [executed Cry controls](../runs/cry-lifecycle-review-20261020/runtime-results.md)
confirm that construction, retained support/level donor and valid invocation need
separate gates: PoB constructs and calculates a level-1 child with its Warcry
disabled or incompatible. Fixed fresh/JIT replays agree. This is source
observation, not gameplay authorization or approval of this proposal.

Some support Gems also supply a Skill. The current native schema can describe
that positive membership with an exact physical `SupportAssignment`, Gem-owned
Grant, SkillGrant and generated Skill. It cannot yet execute that entire path.
This blocks publishing a real Corrupting Cry consumer for the accepted
[Scenario usage intent](owned-scenario-usage-intent-proposal.md) model.

The missing behavior is not another import format or a named-spell exception.
It concerns who may prepare a support's child, which Actor owns that child, and
what makes it eligible to execute. These are public execution semantics and
require design approval.

Keep the existing constraints:

- One owned occurrence graph, injected game data and generic native execution.
- Exact physical sources, independent repeated occurrences and complete source
  accounting. Definition equality does not merge sources.
- Separate structural membership, preparation readiness and requested
  participation, as accepted in the
  [readiness](owned-preparation-readiness-proposal.md) and
  [participation](owned-skill-participation-proposal.md) designs.
- Exact final-input ownership and provider projection from the
  [generated-source design](owned-generated-source-properties-proposal.md).
- No compatibility branch, inferred true activation, fabricated authored SkillUse,
  Player fallback, first-receiver selection or source-language behavior in Engine.

## Evidence and current limits

The pinned PoB revision is
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.
Its Gem data declares these positive routes:

| Support definition | Supplied Skill definition |
| --- | --- |
| Corrupting Cry I, `06eb` | `05a5`, `TriggeredCorruptingCryPlayer` |
| Paquate's Pact / tier III, `06ec` | `05a6`, `TriggeredCorruptingCryThreePlayer` |
| Corrupting Cry II, `06ed` | `05a7`, `TriggeredCorruptingCryTwoPlayer` |

The suffixes abbreviate the release's registered identifiers; native behavior
must not dispatch on these names or IDs.

`Data.lua:995-1015` assembles a Gem's granted-effect list.
`CalcSetup.lua:1994-2028` constructs an active effect for an enabled physical Gem's
non-support effect and retains its exact `srcInstance`, level and quality.
At line 2099, PoB passes `env.player` to `createActiveSkill`; line 2104 inserts
that result into the Player's active-skill list. Thus this source path constructs
one potential Player child per physical Gem/effect, not a child per retained
support receiver.

That is **oracle implementation evidence**, not generic gameplay authority.
Construction does not prove successful trigger invocation, correct ownership
under every supported skill, or behavior after duplicate suppression. It also
does not establish that a support's generated Skill always belongs to the Actor
of its assigned Skill. PoB's selected-group preview and later support application
are separate concerns.

The [bounded source handoff](../runs/scenario-stage-authoring-20261020/README.md)
and [source manifest](../runs/scenario-stage-authoring-20261020/evidence.json)
record full source-file hashes and the actual saved stage settings. They do not
identify an independent game-client snapshot. The five original builds retain
their saved stage settings, but a bounded saved-tag search is not proof that the
complete composed build has no applicable consumer.

The [six-test Rust draft](../runs/scenario-stage-authoring-20261020/support_owned_child_reproducer.rs)
and [execution-seam notes](../runs/scenario-stage-authoring-20261020/support-owned-child-reproducer.md)
were initially captured unexecuted; their frozen notes and manifest retain that
snapshot status. A subsequent temporary Engine target ran all six probes:
**six passed, zero failed, zero ignored** in the
[validation log](../runs/scenario-stage-authoring-20261020/support-child-probe-01.log),
with a separate [validation receipt](../runs/scenario-stage-authoring-20261020/support-child-probe-validation.json).
The temporary target was removed. One unused-import warning came from an
unchanged shared fixture; this investigation has no strict-Clippy claim.
The fixtures use real typed support assignments and supplied child declarations,
but synthetic values. They prove the current compiler/scheduler boundaries and
failure behavior, not successful Cry execution or numerical parity.

The subsequent [static Cry lifecycle audit](../runs/cry-lifecycle-review-20261020/README.md)
narrows the evidence still needed. The child can inherit a compatible active
Skill's recorded level through its exact physical support source, while
support-tagged Gems bypass ordinary active-Gem property collection. Construction,
retention, compatibility and trigger reporting remain separate paths. Reusing
source-property infrastructure does not authorize copying the ordinary active-Gem
collection recipe or inferring execution from positive DPS. These are source
findings awaiting controlled runtime validation; they do not change this proposed
permission contract or approve a parity exception.

Current code establishes these boundaries:

| Existing seam | What it permits | What remains missing |
| --- | --- | --- |
| Core provider/Grant/SkillGrant resolution | Exact potential child identity and structural census | An explicit gameplay Actor binding for that support child |
| `SupportOrigin` rule context | Local preparation stats/capabilities, Requirements and explicitly admitted source-property contributions | Grant activation and child-parameter projection are rejected |
| Ordinary `FinalInputAssembly` role | Declared Grant activation and child projection in an otherwise authorized invocation | The role does not grant a new SupportAssignment invocation context |
| `SourceFinalInputAssembly` with `ExactSupplyingProvider` | Final-input projection to one exact supplied Skill after source-property collection | Grant activation is forbidden; descendant execution is not automatically scheduled |
| Support preparation/finalization | Exact selected origins, receiving applications and one bounded attempt | Non-SupportOrigin descendant Skill programs remain unsupported outside narrowly classified paths |

The current Core SupportAssignment root carries Player even when its assigned
Skill is on a supplied Actor. Reusing that root value as proof of a generated
child's gameplay owner would be incorrect. Existing finalization appropriately
remains unavailable when these producers or contexts are missing.

## Proposed decision

Add a **bounded, explicitly classified supplier/preparation path** for physical
support sources, retaining the existing provider and generated-Skill identities.
Keep SupportOrigin's local-property restrictions.

The first admitted cardinality is one potential child per exact physical support
assignment and declared supply. Its identity remains:

```text
GeneratedSkillKey {
    provider: the exact declaring SupportAssignment provider,
    slot: the Gem's declared SkillGrant
}
```

The entered child provider additionally contains its exact entering Grant.
Repeated physical Gems and assignments remain distinct. Alias checks must reject
ambiguous reuse of a physical Gem. The source does not become a new Actor or a
second authored Skill record.

This decision authorizes neither a specific wire shape nor a generic trigger
dispatcher. Before implementation, the chosen relation must express the following
semantics explicitly and participate in package validation and identity.

### 1. Exact ownership and Actor binding

The declaration names its own Grant, supply, generated Skill and classified
producer programs. It cannot write another Gem's child, a sibling supply or an
arbitrary parent.

The generated child's Actor must come from an explicit, data-defined binding to
an existing exact Actor. Two relevant existing relations are described below to
make the ownership choice precise; this does not require implementing both.
The first implementation admits only the binding demonstrated by the actual
mechanic and its ownership tests. These are alternatives in authored data, not
a fallback order:

- **Existing Player:** a positive mechanical declaration binds to the actual
  Player identity. It is never inferred merely because a physical Gem was
  equipped by the player or PoB stored the source there.
- **Assigned Skill's Actor:** resolve the assignment's exact `SkillTarget` and
  its provider Actor. A missing, ambiguous or unsupported target is unresolved;
  no search by Skill definition or first matching receiver is allowed.

For the initial Cry packet, the bounded positive control should use an actual
Player Warcry. That fixture cannot establish the second binding's correctness
for minions, companions or other supplied Actors. Do not enable a binding without
the corresponding source/gameplay evidence and exact ownership tests. It is
acceptable to implement only the proven binding first and reject the others.

Child rules, Actions, incoming/outgoing effects and source properties must agree
on the resolved Actor context. Resource obligations retain their separately
declared exact payer under the accepted resource model; resolving the child's
Actor grants no payment or other-Actor authority. A private finalizer override
that disagrees with Core query resolution is not an acceptable fix.

### 2. Potential membership is independent of invocation

Structural discovery enumerates known potential children from declarations,
including sources whose execution is disabled or unresolved. It creates no
successful grant value, final parameter, selected support or trigger event.
Partial or unmapped inventories remain incomplete.

Preparation uses the same graph and exact mechanical source. It may compute raw
inputs and independent facts before final inputs exist. A mechanical activation
producer must explicitly account for the source's existence and availability.
Neither a Scenario stage intent nor an Action query can activate a missing source.

Gameplay execution additionally requires the applicable exact gates described
below. A visible PoB active-effect record is not evidence that those gates pass.

### 3. Selection, target and participation gates are declared dependencies

A declaration must state whether child execution depends on the support being
retained for its exact assigned Skill, on an exact admitted receiving application,
or on mechanical source availability independently of support selection.
No omitted classification means "independent."

For a path declared to depend on selection, require the exact origin and retained
position from the same native attempt. If receiver applicability is required,
bind the declared receiving relation explicitly and check its cardinality.
Do not select the first receiver, apply an implicit Any reduction, or merge
several applications into one child invocation.

One child identity also does not imply one retained position: existing source
collection can observe several independent positions for the same source. If
multiple admitted positions or applications claim the source-owned child,
require a proven unambiguous execution rule or report that cardinality as
unsupported. Never clone the child or silently combine eligibility or values.

The child's relevant target, provider/ancestor availability and participation
requirements remain separate inputs to execution. A disabled support, disabled
assigned target or inactive loadout cannot be enabled by the child's own usage
preference. False participation suppresses the appropriate execution descendants;
unknown participation remains unresolved. Preparation retains its accepted
mechanical semantics unless a specific preparation rule explicitly depends on
participation.

In particular, Core's current support-root enabled check follows the assigned
target's provider root; it does not by itself establish that a generated target's
entering grants, readiness and participation are satisfied. Existing support
templates separately bind exact receiving and assigned-target gates. The proposed
supplier path must reuse those applicable phase-aware checks, including the
target's complete ancestor chain, rather than treating root enabled state as a
complete eligibility certificate.

The source evidence still needs to establish which of these gates applies to
each actual Cry child. Until that is established, publishing its positive
membership and raw usage transport must not be reported as complete execution.

### 4. Reuse preparation and source-property assembly with narrow authority

A classified supplier invocation may use the existing ordinary
`FinalInputAssembly` effects for its own Grant and child parameters, subject to
exact write authority and explicit readiness outputs. It must not gain arbitrary
Actor writes or receiving-effect authority.

When final inputs depend on collected properties, reuse
`SourceFinalInputAssembly` and `ExactSupplyingProvider` for the projection step.
The generated Skill owns collected level/quality properties; the exact supplier
projects final inputs. Preserve that role's projection-only restriction.

Current source-property authority is explicitly `PlayerScenarioV1`, and Engine's
source assembly rejects non-Player contexts. Resolving a non-Player assigned
Skill Actor would not by itself authorize source-property execution there.
Non-Player source-property preparation remains unsupported in the first scope;
widening it needs a coordinated ownership/readiness review, not removal of the
current Player check.

Preparation must be acyclic. In particular, admitting the support must not
require the child's final input when that same support selection produces it.
The cold compiler must validate preparation versus execution requirements,
producer coverage and phase ordering. Missing producers, cycles and competing
writers remain explicit failures.

Child numerical programs require their own classified execution path after
those gates are satisfied. Scheduling an assembly program alone does not prove
that the child can execute.

### 5. Keep per-receiver children outside this first contract

An independent child per receiving Actor or Action has different cardinality.
The current source-owned `GeneratedSkillKey` has no retained-application
discriminator. It cannot represent two such children by copying the same key or
silently collapsing them.

Keep such mechanics unsupported until a concrete case establishes the needed
identity and lifecycle. This proposal does not add that identity, clone providers
per receiver, or authorize general command/trigger dispatch. The
[Djinn command investigation](djinn-command-investigation.md) remains a separate
design boundary.

## Alternatives and trade-offs

| Option | Benefits | Costs and limitations |
| --- | --- | --- |
| **Classified supplier path with explicit Actor and execution dependencies — recommended** | Reuses exact identities, readiness, source collection and projection; preserves SupportOrigin's narrow purpose; covers physical-source children without named mechanics. | Requires public invocation/Actor semantics, coordinated Core/Data/Engine validation and exact runtime scheduling. It cannot skip unresolved gameplay evidence. |
| Broaden SupportOrigin to permit own child grants and projection | Fewer immediate invocation distinctions; physical source ownership is readily available. | Mixes local preparation with structural writes. Actor binding, selection, participation and descendant scheduling still need all the same rules, so the apparent smaller change does not remove the difficult work. |
| Treat each retained receiver as a separate child | Fits mechanics where each recipient truly creates an independent effect/Actor. | Needs application-aware child identity and lifecycle, which current generated-Skill keys do not provide. Overstates the available Cry construction evidence and is broader than the current blocker. |
| Keep support children unsupported for now | Adds no execution authority before mechanics are understood. | Blocks real Cry consumer coverage and leaves these Scenario intents unresolved. Potential declarations and honest diagnostics can still be published independently. |

The recommended direction preserves a generic model while bounding the first
implementation. Approval would accept its ownership and permission principles,
not claim that Cry trigger semantics or any positive numerical path are already
proven.

## Consequences and migration

Affected definition/receiving/readiness contracts must state their new authority
explicitly. Change the current format in place and rebuild affected artifacts;
do not add an old/new compatibility branch. Update canonical identities and
execution-plan/cache domains wherever changed semantics can affect a result.

Cold compilation must bound relation discovery, actor resolution, descendant
expansion and templates. Hot evaluation remains inside the existing attempt and
work budget, using immutable plans and worker-owned scratch. Retained selection
or readiness evidence cannot be supplied by callers or reused across attempts.

This work does not justify retiring unrelated rule validation, treating empty
inventories as proven absence, or changing the original five inputs. Full
numerical coverage and optimizer admission remain separate completion gates.

## Acceptance work after a decision

1. Baseline completed: all six diagnostic probes passed. Preserve their exact
   rejection/gap provenance when adding positive paths; their success does not
   close any actual Cry lifecycle or numerical obligation.
2. Source-observation baseline completed: eight controls cover a Player Warcry,
   each support tier, disabled support/Warcry, a non-Warcry receiver and tied
   duplicate order. Fixed fresh/JIT replays agree. Remaining evidence includes
   mixed tiers, multiple donors, final supported levels, disabled-group selection,
   participation and gameplay invocation/cost/frequency. Potential construction
   and reported damage do not prove those remaining gates.
3. Review and implement the exact relation/wire shape only for the approved
   Actor/cardinality domains. Validate all declarations, including dormant ones;
   reject foreign writes, aliases, ambiguous targets and incomplete inventories.
4. Prove a positive physical SupportAssignment -> Grant -> SkillGrant -> child
   through current V28 support preparation/finalization and source-property
   readiness. Use real data for the final integration, not an authored-Skill
   substitution.
5. Check repeated independent sources, fresh/reused/Rayon scratch, reordered
   declarations, unrelated metric requests and bounded work. Verify that disabled,
   unselected, unavailable and partial cases cannot produce successful execution.
6. Publish real stage transport/applicability/consumer data only to the extent
   demonstrated, retaining unresolved trigger, receiver or numerical obligations.

## Decision requested

Adopt the bounded classified supplier direction, with explicit data-defined Actor
binding and execution dependencies, while retaining SupportOrigin's current
restrictions and leaving per-receiver children/general dispatch unsupported?

The project owner can decide that architectural direction. The actual Cry gating
and Actor rules still require source/gameplay evidence; they should not be
invented as a user preference or hidden behind an implementation default.
