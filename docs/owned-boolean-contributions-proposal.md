# Boolean contributions and complete flag queries

Status: proposed for owner review, 2026-10-06. This changes a public data
contract; no runtime implementation is approved by this document. The proposal was drafted against
baseline `runs/owned-player-offhand-01/package` at commit `7b7fb1b`.

## Problem and evidence

The five inherent Strength-to-Life controls (`3315` through `3319`) are resolved
Boolean Stats with no producers. Their consumer already exists. They represent
disabling all inherent bonuses, disabling Strength bonuses, disabling Strength
Life, doubling inherent bonuses, and halving Strength Life.

PoB reads these through `Flag` at `CalcPerform.lua:496`–`503`, independently of
its condition lookup. Actual effects can supply multiple instances of the same
flag. Giant's Blood supplies halving, Enhanced Effectiveness supplies doubling,
and the pinned source defines Irongrasp's Strength-Life suppression through both
Iron Grip and Iron Will. This provides a duplicate-source example without a
claim about current item obtainability. Timeless passives
supply further suppression cases; their canonical native acquisition remains
unfinished. Five originals having false flags is not candidate-wide absence
evidence.

Current native Stat and Capability effects are singular final producers.
Engine correctly rejects competing writes. Numeric contributions accept Add,
Increase and Multiply with Sum/Product reductions. An expression `Any` combines
a fixed list of program values, not the candidate's dynamically discovered
effect occurrences. None of these contracts currently represents the required
multiple-source Boolean fact.

## Recommended contract

Extend the existing contribution/query model with typed Boolean contributions
and an **unordered `Any` reduction**, on the existing occurrence/effect graph.
Keep the five resolved Stats and their consumer. Contribution channels and final
Stat values are already distinct graph keys, so this needs no numeric counter
Stats or duplicate build model.

```text
Selected provider occurrences → typed Boolean contributions
                             → checked complete query membership
                             → unordered Any → one resolved Boolean Stat
                             → existing inherent-bonus receiver
```

- Every potential matching effect must have an explicit declared member,
  including inactive, false and repeated occurrences. Reuse the existing cold
  query binder and its unmatched-member and coverage checks.
- Separate membership from semantic ordering in that shared query contract.
  Numeric folds keep explicit ordered policies. Boolean `Any` needs no invented
  source, slot or modifier ranks, and equal-ranked repeated occurrences must
  not be rejected merely because the numeric ordering rules would tie.
- Repeated true contributions are idempotent. False or inactive contributions
  cannot remove an independent true value. Unknown is distinct from false.
- Require complete membership and provider/receiver coverage before checked
  evaluation. Empty=false is an identity of a proved empty domain, never a
  missing-producer fallback.
- Initially use conservative unresolved propagation: any unresolved active
  contribution leaves the reduction unresolved, even if another is true.
  Collect diagnostics deterministically. Short-circuiting must not hide missing
  input or coverage. This policy belongs to the native typed contract; it does
  not import source-language truthiness or source iteration behavior.
- Reuse existing readiness, finite stages, recipient binding, cycle checks,
  immutable plans and worker scratch. No Lua host or source objects enter this
  evaluation path. Canonical diagnostic ordering has no effect on Boolean truth.

Update the current development contract and rebuild affected data. The owner
does not require old/new format compatibility. Exact enum and DTO naming will
be chosen during implementation without changing these semantics.

## Alternatives and trade-offs

| Choice | Benefit | Cost |
| --- | --- | --- |
| Typed Boolean contributions with unordered membership (recommended) | Models flags directly, reuses completeness and occurrence identity, supports future buffs and disabling effects | Requires Core/Data/Engine contract and validation work now |
| Count contributions followed by Sum > 0 | Reuses existing numeric machinery immediately | Adds artificial units, ranges and ordering to idempotent truth; creates a convention future flag authors must reproduce |
| Fixed Actor expressions or one producer per source family | Small for a selected example | Cannot naturally bind arbitrary repeated equipment/passive sources; risks duplicate writers and incomplete absence claims |

The recommendation addresses the representation gap before publishing five
false defaults or adding item-specific evaluator branches. It does not complete
source eligibility, transformed passives, effective conditions, or any build.

## Implementation and validation gate

1. Extend the single current typed contribution/query contract; preserve numeric
   ordering and single-final-producer checks. Replace superseded shapes instead
   of adding a parallel Boolean evaluator or compatibility mode.
2. Validate Boolean identity/type, exact membership, permitted origins and
   recipient authority. Reject Boolean/numeric mixing and ordering policies
   inappropriate to the selected reduction.
3. Test empty domains, false/true, duplicate instances, inactive and unresolved
   contributors, missing members, Partial inventories, cycles and stage access.
   Permutations, occurrence rebasing, fresh/reused scratch and Rayon must agree.
4. Publish real flag producers from authenticated source ownership, starting
   with contrasting passive/item cases above. Keep unrelated owner gaps open.
5. Feed the existing Strength-Life receiver, regenerate all five originals and
   preserve source dispositions and 110 queries. Final Strength and final Life
   remain separate obligations.

Source anchors: `ModStore.lua:281`–`295`, `ModDB.lua:297`–`317`,
`ModParser.lua:2526`–`2527`, `2624`–`2630`, `5887`–`5895`,
`Data/Uniques/body.lua:493`–`502`, and tree nodes 32349/58591 in the pinned
`vendor/path-of-building-poe2` revision. These are bounded source findings,
not claims of complete native producer coverage.
