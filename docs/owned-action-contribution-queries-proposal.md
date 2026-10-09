# Checked contribution queries on exact Actions

**Status:** Accepted by the owner on 2026-10-09; Core/Data/Engine implemented in operations V26. Command data adoption is next.
**Date:** 2026-10-08.
**Decider:** Project owner.

## Context

Original05's inherited passive and post-stacking Offering increases are joined,
and its quality/Gigantic MORE consumer is now validated. The next damage
consumer must include action-specific contributions. The existing Command
packet already contributes to `3304`: Basic Attack produces zero when its
Command applicability is false; Gas Arrow produces the received increase for
each applicable selection. These producers are data-defined Action-output
programs, and are not ordinary Actor or Skill self-contributions.

Before this extension, checked-query reads admitted an Action reading its Actor's
channel, but not a query on the current Action. Producer membership likewise had
no exact Action origin. Reading only Actor values would omit existing Command behavior;
using an unchecked reduction would bypass the membership model adopted for
other damage components. This is a numerical consumer blocker, separate from
the four remaining Original05 imported-input obligations.

## Recommended decision

In practical terms, an Actor represents the character or a particular minion;
an Action represents one specific skill output and its selected part, mode and
stat set. A modifier that applies to one minion's Gas Arrow must not become a
shared Actor value that also affects its Basic Attack or another minion.

The existing flow is: data-defined producers emit typed contributions, effect
applications resolve their stacking, checked queries account for the permitted
sources and reduce their values, and downstream formulas read those results.
The accepted application-group extension joins results after stacking into that
flow. This proposed Action extension preserves the same flow at Action scope.
It changes source authority and binding, not the calculation language or worker
execution model. Missing coverage remains unknown rather than becoming zero.

Extend the existing checked contribution graph to exact Action self-contributions:

- Admit a current-Action query read when its Stat explicitly targets Actions.
- Add explicit producer authority for known Action outputs, with ownership
  authenticated against the corresponding Skill definition or Action-output
  declaration. Preserve authored versus generated Skill supply authority rather
  than accepting an arbitrary provider merely because it ends at an Action.
- Bind membership to the validated Action occurrence: provider path, output,
  part, mode and stat set. An Action cannot read a sibling's contributions.
  Repeated skills, supplied actors and their child abilities remain independent.
- Reuse checked declaration/concrete membership, explicit numeric positions,
  unknown/Partial handling, dependency stages and worker reductions. Validate
  potential producers before looking at values, including zero/inactive writers
  and declarations not selected by this request.

This adds neither a new calculation engine nor new game-specific operations.
It grants no cross-Action inheritance. Support-delivered effects and future
Action-targeted application groups need explicit reviewed authority; this
decision must not silently admit them as self-contributions. Existing late
support inventory validation remains mandatory.

The first data adoption should consume the actual Command channel, with positive
Gas Arrow controls as well as Basic's zero result, then join the inherited/applied
damage components. The physical pipeline still needs checked flat damage,
conversion/gain, minimum/maximum modifiers and final rounding. This proposal
does not close those domains or authorize supplied constants as substitutes.

## Alternatives and trade-offs

| Approach | Benefit | Cost |
| --- | --- | --- |
| Extend exact Action membership in the shared graph (recommended) | One coverage, ordering and execution model; preserves action-local modifiers and repeated occurrences | Public contract change and generic authority tests before data adoption |
| Add a direct special-purpose Action aggregate | Can connect one consumer with less initial plumbing | A separate completeness/binding path; later support and damage consumers must coordinate it |

There is no requirement to preserve the development wire format. Rebuild current
artifacts and invalidate affected identities if the contract changes; retain
independent source evidence.

## Acceptance and follow-up

1. Check the contract generically for authored/generated Skills, Skill-definition
   and output owners, exact output/part/mode/stat-set distinctions, nested actor
   suppliers and duplicate occurrences. Bound validation work.
2. Reject missing/extra membership, forged ownership or supply, mismatched
   channels/recipients, unread/zero/inactive omissions and unreviewed delivery.
   Preserve missing and Partial results, without treating them as empty.
3. Verify stages/cycles and fresh/reused A–unknown–B–A and four-worker execution.
4. Adopt the real Command programs and validate positive/negative selection and
   source removal controls against retained evidence, preserving all five imports.
5. Continue the closest complete-build path. No optimizer-ready build is claimed
   by this component alone.

## Implemented contract (2026-10-09)

`ContributionOrigin::Action` identifies permission for authored Direct uses and
explicit generated Skill supply slots. Program ownership is a Skill definition
or an Action output declared by that Skill. Each generated slot must supply that
Skill and admit the specific output when the member is output-owned. Gem-backed
skills use their explicit generated occurrence; a symbolically bindable Gem root
does not substitute for that child or bypass activation/readiness.

Data validates all potential current-Action writers on the queried channel,
including unread, inactive, zero-valued and unselected declarations. Native cold
planning then authenticates exact provider/supply, owner and recipient addresses.
Part, mode, stat set and output remain part of the ordinary Action identity.
Existing membership, numeric ordering, Partial refusal, stage/cycle checks and
worker reductions are reused; there is no new collector or evaluation backend.
Unselected Skill-owned Action programs have no invocation in V26 and no longer
create a false unsupported-context gap. Their declarations still undergo census.

The current release assembler admits V26 with a new effect-plan identity domain.
The generic regression suite exercises Direct and Gem-supplied skills, nested
Actor supplies, repeated sources, distinct outputs and selection axes, missing
and Partial results, forged membership, unreviewed support delivery, work bounds,
cycles, stage ordering, storage round trips/permutations and fresh/reused/four-worker
evaluation. See the current implementation checkpoint for actual run results.

The canonical game-data package remains V25 until the real Command consumer is
published and its five original inputs are revalidated. This capability alone
closes no game-mechanics coverage gap and produces no complete build. Optional
numeric selection is accepted but remains a separate next implementation step.
