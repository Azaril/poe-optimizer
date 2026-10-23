# Class base attributes

This documents the historical acquisition step. Its output was subsequently
rewired by the [Count cutover](../attribute-stages/COUNT-CUTOVER.md) into the
current pass-specific inputs. Intrinsic attack components were added later;
common Player rules have their own shared Actor owner. Remaining Class coverage is tracked in the
[active plan](../../../../../docs/implementation.md#checkpoint-workflow-and-next-action).

`policy.json` binds three reviewed source integer fields to existing owned contribution
stats 7470 (Strength), 7471 (Dexterity) and 7472 (Intelligence). The Rust converter reads
all eight class rows from the caller's pinned tree JSON. No class values, names, example
builds or Lua execution are built into the converter or native evaluator.

Current publication uses the [maintained KDL source workflow](../migration-source/README.md).
The old `compile-owned-class-bases` converter remains only for live intrinsic
acquisition callers and its Import tests. It is not a required step in current
source publication. Those tests retain source pins, complete class membership,
reviewed field shape, target types, predecessor bindings, bounds and idempotency.
Generic Authoring checkpoint and Host tests retain immutable replay, failed-build
cleanup and occupied-destination preservation.

The dedicated historical CLI test target is retired. Its useful Class behavior
checks now run against the maintained release, covering all eight Classes,
both Count contribution passes, the unarmed components and every original's
resolved Class presets. Run
`cargo test -p poe-optimizer-authoring --test release class_support`.
The [intrinsic Class source-domain proof](../class-support-domains/README.md)
uses complete archived source tables and preserves separate owners for passive
roots, ascendancies, equipment and shared Actor rules. An intrinsic Class domain
does not establish coverage of those other sources.

At the historical checkpoint, only reviewed input declaration closures became
complete; unarmed defaults were still absent. Current Class numerical rule coverage
remains Partial even though the separate intrinsic attack components are present.
Class-base contributions are not complete
attributes. The full [attribute design](../../../../../docs/owned-attributes.md) records
finite staged dependencies, MORE grouping, rounding and downstream inherent bonuses.
Complete native original builds remain 0/5.
