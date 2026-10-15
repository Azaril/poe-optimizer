# Proposal: resource obligations independent of metric selection

**Status:** Accepted by the owner on 2026-10-15. The Skill support-recipient
extension is implemented; the typed obligation effect, payer/dependency binding
and reservation-data migration remain to be implemented.
**Date:** Proposed 2026-10-06; accepted 2026-10-15.

The owner selected Skill-owned obligations with explicit Action dependencies.
Implement the recommendation below, preserving the single occurrence graph,
exact payer/resource identity and complete contributor checks. No further
approval is needed for this agreed ownership change. The implementation plan
tracks the contract, support-recipient extension and current reservation
migration; acceptance alone does not change existing numerical coverage.

## Problem

The native Sniper reservation component currently runs in an Action context.
Actions are discovered from metric queries, saved Action choices or usage, and
explicit support receivers. An active summon can therefore lack the parent
Action needed to calculate its reservation when only its minion's damage is
requested. Adding a hidden query would confuse a mechanical dependency with a
request to report a result.

The pinned reference calculates reservation across the actor's active skills,
independently of which minion attack is selected for display. This supplies
evidence for the dependency, not authority to copy PoB's UI or activeSkill object
model. See the existing [reservation component](owned-summon-reservation.md).

## Recommendation

Represent an ongoing resource obligation on the exact participating Skill
occurrence. Its declared dependencies determine what must be evaluated, even
when no metric directly requests that Skill. Summon counts, activation and
scenario overrides retain their existing ownership. Repeated physical or
generated sources remain separate occurrences on the same graph.
Declare the payer Actor, resource and obligation kind explicitly. Aggregate
each exact obligation occurrence once, independently of the number of queried
Actions; do not assume that every payer is the Player.

Allow an obligation to depend explicitly on an exact Action when the mechanic
depends on its selected part, mode or stat set. That selection must come from
reviewed definitions and actual build/scenario choices; it cannot be inferred
from the first query or an arbitrary available Action. Unsupported or ambiguous
selection remains unresolved. Ordinary per-use costs stay associated with their
Action rather than being charged as ongoing reservation.
Reservation-to-cost conversion must choose the applicable cost calculation and
rounding, not relabel an already-rounded reservation amount.

This needs more than moving a program to Skill context. Before this migration,
the support receiving contract admitted Actor and Action endpoints only.
Skill-level resource modifiers need an explicit Skill recipient and applicability contract, retaining
support occurrence provenance and complete contributor checks. The existing
preparation/execution phases and exact source membership remain authoritative;
this proposal does not introduce a second build graph or let a query activate a
disabled Skill.

| Approach | Benefit | Cost |
| --- | --- | --- |
| **Skill-owned obligations with explicit Action dependencies — recommended** | Models sustained reservation independently of reporting, while retaining Action-dependent mechanics. Supports repeated summons and other sustained skills without a fake cast. | Requires a reviewed obligation/dependency contract and Skill support recipients; current Action-owned reservation data must migrate. |
| Explicit required Actions on Skill definitions | Can reuse current Action reservation programs and support recipients, with less immediate migration. | Treats sustained reservation as an Action calculation and requires concrete selections even for skills with no meaningful cast. Needs additional rules to prevent duplicate obligations across Actions. |

The accepted option permits no implicit zero costs, missing-input defaults or
relaxation of selected-build coverage. Skill support receiving is the first
implemented slice; it adds no obligation producer or payer aggregation by itself.

## Implementation gates

### Existing contracts to reuse

Keep `SkillTarget`, `ActorKey` and `ActionSelection` as the occurrence identities.
An obligation is identified by its exact Skill occurrence and declaration; it
does not create another Skill or Actor. Skill programs already instantiate from
the discovered build independently of requested metrics. Extend that binding
with a checked census of potential obligations, explicit payer relations and
named Action dependencies rather than adding a separate evaluator.

For implementation, reuse the existing owner/program/effect inventory as the
potential-obligation census. Add a typed obligation effect with explicit payer,
resource, kind and amount, emitted only by a genuine once-per-Skill owner
invocation. Its exact occurrence plus owner/program/effect declaration supplies
identity. Support applications modify its inputs; they must not create another
copy of that obligation. Complete/Partial program ownership already accounts
for untranslated mechanics, so a second top-level obligation catalogue would
duplicate authority. This effect and its binding are planned, not implemented.

The [aggregation refinement](owned-resource-obligation-aggregation-proposal.md)
records the next concrete query-signature and integer-quantum representation for
owner review. It keeps obligation amounts separate from resource-pool additions
and addresses repeated sources without inventing numeric ordering.

Extend the existing checked program-effect query address with a separately
validated obligation-to-payer origin. Validate the actual effect kind and its
resource, payer and numeric type before admitting membership. Ordinary Skill
self-contributions keep their current authority; they cannot be reclassified as
Actor contributions merely because their values or units match.

Use the canonical Actor pool Stat as the resource identity, validated for Actor
applicability and the resource-points unit. Life, Mana and Spirit remain distinct
by definition identity even though they share a unit. Reserved and unreserved
results retain their own Stat definitions. This does not require a parallel
resource catalogue or authorize an arbitrary Skill to write to any Actor.

Bind declared Action dependencies before owner-program instantiation, using the
existing discovery, readiness, value-channel and cycle checks. An explicit read
must name the bound dependency; scheduling an Action alone does not grant a
Skill access to its values. Action part/mode/stat-set predicates remain in Action
context. Unknown or ambiguous selections remain unresolved.

Extend the existing support-receiving endpoint with an exact Skill recipient in
both preparation and execution. The authored physical Gem container and the
generated Skill it exposes are distinct targets: an empty path must not alias
one to the other. Use checked grant paths for generated effects, and preserve
assigned-source identity, receiving-provider readiness, participation and
complete role inventories. Test a Gem exposing two different Skills as well as
repeated and independently generated sources.

The payer aggregate must preserve every admitted obligation occurrence exactly
once. Reuse checked contribution membership with an explicit obligation-to-payer
relation; existing Skill self-contribution permission is insufficient. Numeric
aggregation requires semantic ordering or a proven exact-sum domain, never a
tie-break using local IDs or discovery order. A counted summon has one
obligation whose amount uses its count, not one duplicate Actor per summon.

Implement and validate Skill support recipients first, then the obligation
declaration/binding and payer aggregation, then migrate the current Sniper
reservation data. Each checkpoint must state which of these is live; a support
endpoint alone does not make reservation query-independent.

### Delivery checks

1. Define one bounded dependency contract using existing occurrence identities,
   with explicit activation, selection authority, phase and cycle checks. Review
   contrasting shapes: a counted summon, a sustained buff, an Action-dependent
   reservation and a per-use cost, including reservation-to-cost conversion,
   mine population and stance/toggle choice. Preserve the separate participation
   decision.
2. Extend the checked support-recipient contract only as required by those shapes.
   Keep ordinary resource-cost and reservation multipliers distinct.
3. Migrate the existing Sniper arithmetic and intrinsic table through a checked
   release transition, removing superseded live bodies rather than leaving a
   selectable old reservation path. Preserve useful historical parity evidence.
4. Prove that adding, removing or reordering unrelated metric queries does not
   change reservation; disabled occurrences, independent roots, count zero,
   missing dependencies and cycles retain their correct outcomes. Validate
   selected Action sensitivity only where the mechanic declares it.
5. Supply real inputs and compare with the existing source controls before
   claiming full reservation or whole-build parity. Run deterministic scratch
   reuse and parallel replay, and preserve all five unchanged imports.

Gigantic Following's independent status and owner-side efficiency contributions
can be published while the accepted migration is being implemented. Its
Life/Damage consumers and resource delivery remain separate work. This proposal
grants no source-bug exception and no new numerical closure.
