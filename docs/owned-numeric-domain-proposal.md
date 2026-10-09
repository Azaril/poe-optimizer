# Proposal: checked numeric domains in typed rules

**Status:** Proposed; owner decision required before public-contract implementation.
**Date:** 2026-10-09.
**Decision owner:** Project owner.

## Context

Final Mana can use existing checked contribution groups, typed adjustment inputs
and coherent override selection. The remaining formula uses ordinary arithmetic,
rounding and a minimum of one. Native nearest-ties-positive rounding followed by
that minimum agrees with the original PoB expression for every finite binary64
operand at most 2^52. The [rounding audit](legacy-retirement.md#player-life-rounding-bound-2026-10-08)
records the analytical proof and original-helper counterexample at 2^52 + 1.
This is an evaluator parity bound, not a game rule or evidence that such values
are obtainable. The current computed Quantity and Integer types admit values
outside it; observing small values in the five examples does not narrow those
contracts. Raw parameter ranges alone do not constrain arbitrary computed values.

The final resource consumer must not silently clip a large value, encode
capability limits as build illegality, or reproduce floating-point accidents as
new native rounding semantics. Missing conversion sources remain a separate
coverage question; numerical-domain validation cannot close them.

## Recommended decision

Add one reusable **checked numeric-domain expression** to the existing typed
rule DAG. It accepts an expression and an explicit inclusive numeric range,
reusing the existing IntegerRange and QuantityRange representations. Its result
has exactly the input's type and, for quantities, exact unit.

- A known value inside the declared range passes through unchanged, including
  zero and its sign. The operator does not clamp, convert units or round.
- A known value outside the range produces an explicit unsupported-domain
  diagnostic. Consumers cannot use it as zero, absence or an inactive mechanic.
- Missing inputs, upstream unavailability and numerical failures remain distinct
  from a known out-of-range value; no fallback or retry is implied.
- The ordinary lazy branch rules apply. A resource override can bypass the
  ordinary rounding operand, while complete source coverage remains mandatory.
- Range order, finite endpoints, numeric type and exact units are validated once
  at compilation. Evaluation uses a bounded comparison in existing worker
  scratch. There is no Lua code, separate evaluator or game-specific branch.

The implementation must use the current diagnostic and availability pipeline,
with an explicit representation for the new domain failure if necessary. A
failed game Requirement currently represents a different concept and is not a
substitute. A false effect guard also means inactivity, so it cannot stand in
for a failed numeric-domain check.

For Mana, author the full arithmetic using its checked inputs, validate the
actual pre-rounding operand, and reuse native rounding/minimum within the
proved range. Preserve the source discrepancy outside that range as evidence;
do not grant a general PoB-bug exemption. The upper bound is calculation data,
not a hard-coded resource constant in the engine.

## Options and trade-offs

| Option | Benefit | Cost |
| --- | --- | --- |
| **Checked numeric-domain expression — recommended** | Explicit availability, reusable across resource/timing/scaling formulas, typed and constant-work | Extends the public rule and diagnostic contract; requires compiler, stage and worker tests |
| Prove every upstream source bound before adding each consumer | No new expression; strongest whole-domain static proof when obtainable | Delays consumers behind every supplier and composition limit; future source additions need new proofs, and unbounded computed types remain |
| Express PoB's add-half/floor sequence directly | Immediate exact reproduction of this source arithmetic | Carries an incidental large-number discrepancy into owned behavior; does not solve other supported-domain limits |

This proposal changes no global rounding mode or game-legality rule. It does
not require a generic theorem prover or range optimizer. Static proofs can later
remove redundant checks while preserving the declared semantics.

## Implementation and acceptance after approval

1. Extend the current contract in place, invalidating affected plan identities;
   do not add a historical reader or parallel rule path.
2. Check valid/reversed ranges, mismatched units/types, endpoint inclusion,
   adjacent values, signed zero, finite extremes and malformed storage.
3. Preserve lazy evaluation, missing/unknown propagation, preparation readiness,
   stage dependencies and complete-request coverage. Demonstrate that a domain
   failure cannot be mistaken for a satisfied game constraint or a zero metric.
4. Compare fresh/reused/parallel results and ensure constant bounded worker work.
5. Author the final Mana consumer with actual input bindings, retained original
   source vectors, complete effect membership and an explicit rounding-domain
   check. Missing conversion sources and Partial owners remain unavailable.
6. Rebuild the current package, reimport all five examples and refresh native
   replay; report full-build parity only when its independent gate passes.
