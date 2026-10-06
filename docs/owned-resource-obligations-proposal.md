# Proposal: resource obligations independent of metric selection

**Status:** Proposed; owner decision required before implementation.
**Date:** 2026-10-06.

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

This needs more than moving a program to Skill context. The current support
receiving contract admits Actor and Action endpoints only. Skill-level resource
modifiers need an explicit Skill recipient and applicability contract, retaining
support occurrence provenance and complete contributor checks. The existing
preparation/execution phases and exact source membership remain authoritative;
this proposal does not introduce a second build graph or let a query activate a
disabled Skill.

| Approach | Benefit | Cost |
| --- | --- | --- |
| **Skill-owned obligations with explicit Action dependencies — recommended** | Models sustained reservation independently of reporting, while retaining Action-dependent mechanics. Supports repeated summons and other sustained skills without a fake cast. | Requires a reviewed obligation/dependency contract and Skill support recipients; current Action-owned reservation data must migrate. |
| Explicit required Actions on Skill definitions | Can reuse current Action reservation programs and support recipients, with less immediate migration. | Treats sustained reservation as an Action calculation and requires concrete selections even for skills with no meaningful cast. Needs additional rules to prevent duplicate obligations across Actions. |

Neither option permits implicit zero costs, missing-input defaults, or relaxation
of selected-build coverage. Neither is implemented by this proposal.

## Implementation gate after approval

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
can be published while this decision is pending. Its Life/Damage consumers and
resource delivery remain separate work. This proposal grants no source-bug
exception and no new numerical closure.
