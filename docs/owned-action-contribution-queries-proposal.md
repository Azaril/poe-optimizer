# Checked contribution queries on exact Actions

**Status:** Proposed; implementation requires the owner's design decision.
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

Current checked-query reads admit an Action reading its Actor's channel, but
not a query on the current Action. Producer membership likewise has no exact
Action origin. Reading only Actor values would omit existing Command behavior;
using an unchecked reduction would bypass the membership model adopted for
other damage components. This is a numerical consumer blocker, separate from
the four remaining Original05 imported-input obligations.

## Recommended decision

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
