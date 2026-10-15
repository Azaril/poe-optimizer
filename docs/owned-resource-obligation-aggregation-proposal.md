# Proposal: checked resource-obligation aggregation

**Status:** Proposed refinement of the accepted Skill-owned obligation model.
The generic exact-integer reducer is implemented and validated as the accepted
proven-exact-sum prerequisite. Obligation effects, amount channels and their
query signature are not implemented by that reducer.
**Date:** 2026-10-15.

## Why a separate amount channel is needed

The [accepted ownership model](owned-resource-obligations-proposal.md) uses the
existing Skill owner/program/effect inventory and checked contribution graph.
The current contribution channel is identified by recipient, Stat and
contribution kind. Emitting reservation as an ordinary Add to the canonical
Life, Mana or Spirit Stat would mix the reserved amount into the resource pool.
Matching units also cannot prove that a Mana obligation belongs in Spirit totals.

The exact obligation must therefore preserve its Skill declaration, payer Actor,
canonical resource Stat and obligation kind through aggregation. The amount
channel is separate from the pool and final unreserved-resource Stats. It is not
another source catalogue or a second occurrence graph.

## Recommended contract

Extend the existing checked contribution query with an explicit obligation
signature: canonical resource Stat, obligation kind and positive resource-unit
quantum. For the first admitted reservation domain:

- The query's amount Stat is an Actor-targeted Integer channel, distinct from
  the canonical pool Stat. Every obligation on that channel uses the same exact
  resource, kind and quantum.
- A typed Skill-owned obligation effect declares the same channel/signature,
  an explicit payer relation and an Integer amount node. Its owner/program/effect
  declaration plus exact Skill occurrence is its identity.
  Reservation requires a nonnegative emitted amount after its reviewed formula
  and rounding. A negative count is unsupported and cannot cancel another
  obligation; discounts belong in the occurrence's formula. Zero remains a real
  admitted occurrence. This restriction does not apply to the generic signed sum.
- A separately checked obligation-to-payer origin binds actual Skill sources
  to that payer. Ordinary Skill self-contribution permission is unchanged.
  Support/application invocations cannot emit another copy of the obligation.
- Validation rejects mismatched signatures, ordinary contributions or application
  group writers on an obligation amount channel, and unchecked raw contribution
  reads that would bypass the obligation census.
- Each active occurrence contributes once through an unordered exact Integer
  sum. A complete empty census produces zero; incomplete membership or an
  unresolved active amount remains unavailable. Disabled and counted-zero Skills
  retain their distinct occurrence and participation evidence.
- Existing typed quantization expresses the mechanic's reviewed per-occurrence
  rounding before emission. After aggregation, explicit scaling converts the
  total Integer count back into the exact signature's resource quantum. That
  binding must be checked; an arbitrary scaling constant is not authority for
  the declared quantum. This must not invent a rounding step or replace
  fractional mechanics with integer rounding.

Canonical pool identity is an Actor Stat in the resource-points dimension; Life,
Mana and Spirit remain distinct identities. The quantum must be finite, strictly
positive and use the pool's exact unit identity, not just the same dimension.
It is injected data, not a hard-coded assumption that every mechanic uses one
whole point. Unsupported
fractional or alternate-rounding domains stay unsupported until they have a
reviewed representation. Ordinary per-use costs remain Action-owned.

The alternative is a dedicated typed obligation-query variant alongside ordinary
contribution queries. It makes the channel distinction structural, but adds
public query shapes and broadens channel/read/stage plumbing, even if the
implementation shares membership and aggregation helpers. It still needs exact
source/payer checks and an explicit numerical domain. Reusing the existing query
with a checked signature is recommended because it preserves one aggregation
system while retaining all the domain identity.

This adds public query/effect fields beyond the accepted ownership decision, so
the signature and integer-quantum representation await owner review. The generic
integer reduction below is independent of that choice.

## Exact Integer sum prerequisite

The explicit `ExactIntegerSum` reduction is implemented in the current V27 contract. It is
available only in checked Add queries over Integer Stats, with unordered members,
integer zero as identity and no semantic ranks. Existing Sum/Product behavior
does not change, and raw contribution reads cannot request this reduction.

Accumulate admitted Integer values in checked `i128`, inspect all active failures
using the existing deterministic diagnostic policy, then check the final result
against `BoundedInteger`'s range. The hard 65,536-effect ceiling and the bounded
integer operand range keep every possible intermediate magnitude below `2^69`.
Cancellation is therefore exact and cannot create an order-dependent overflow.
Local IDs can determine storage order but have no numerical meaning. Repeated
sources remain distinct; equal values are not deduplicated.

This is a generic checked Integer operation, not reservation arithmetic. It adds
no rounding, resource identity, payer authority, obligation producer or fallback.
Test repeated source definitions, reordered declarations, cancellation near the
bound, final overflow, missing inputs, inactive contributors, incomplete census,
scratch reuse and parallel execution.

## Migration sequence

1. Complete the exact Skill support-recipient checkpoint (implemented and tested).
2. Implement and validate the generic exact Integer reduction (complete).
3. Following review, implement obligation effects, signature checks and exact
   payer binding in the existing graph.
4. Migrate Sniper's intrinsic reservation and ordinary flat consumer together,
   replacing their Action-owned live bodies through a checked data transition.
   Their existing reads need no Action selection: prepared level, typed Skill
   channels and exact usage count are sufficient. Preserve explicit unsupported
   branches and all existing input/coverage gaps.
5. Add named exact Action dependency declarations and reads for mechanics that
   actually need part/mode/stat-set selection. Bind through existing Action
   discovery and value/cycle checks; never infer a dependency from metric queries.
   Existing Build choices can select an explicitly mapped finite variant.
   Scenario usage parameters are not a generic Action-choice override map;
   scenario-driven selection would need explicit composed-input authority.
6. Prove query invariance, exact payer/resource isolation and source parity before
   claiming reservation completion. Preserve all five original imports.

The direct Sniper path can advance without pretending that step 5 is already
implemented. Reservation-to-cost conversion and other Action-dependent mechanics
remain explicitly outside its admitted domain until that contract is delivered.
