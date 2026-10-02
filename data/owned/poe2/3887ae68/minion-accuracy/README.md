# Ordinary minion attack hit chance

This injected slice extends the native Sniper population and its `MinionMeleeBow`
Basic Attack. It uses the pinned PoE2 behavior: `CalcPerform.lua:1072–1076`
adds `CannotBeEvaded` unless player `MinionAccuracyEqualsAccuracy` is enabled.
Ordinary minions therefore do not read a monster accuracy curve in this version.
The source's inherited-player-accuracy branch remains unsupported here.

The actor program reads the complete player inheritance-flag contribution channel.
Only a complete channel with no enabled flag selects the ordinary branch, derives
the intrinsic flag, and emits 100 percentage points of accuracy hit chance. The exact existing
Basic Attack selector routes that value to the action. Its action program sums
**all** enemy BlockChance BASE contributions (including the independently authored
configuration producer) and its own `reduceEnemyBlock` BASE contributions, then
uses the source order:

`max(min(blockBase, 100) - reduction, 0)`

Enemy `CannotBlockAttacks` changes that effective block to zero for this declared
attack. Hit chance is `accuracyHitChance * (1 - effectiveBlock / 100)`. There is
no extra rounding or final upper clamp: negative reductions can produce block
above 100 and consequently a negative result, as the pinned source expression
permits. This is not a rule for arbitrary spell actions.

## Definitions and input authority

| ID suffix | Meaning | Scope |
| --- | --- | --- |
| 3219 | MinionAccuracyEqualsAccuracy flag-presence contributions | Player actor |
| 321a | Intrinsic CannotBeEvaded boolean | Owned actor |
| 321b | Intrinsic accuracy hit chance | Owned actor |
| 321c | Routed accuracy hit chance | Exact Basic Attack action |
| 321d | reduceEnemyBlock BASE contribution channel | Action |
| 321e | CannotBlockAttacks flag-presence contributions | Enemy |
| 321f | Effective enemy block | Action |
| 3220 | Hit chance after block | Action |

The flag channels use existing integer ADD reductions. Each admitted source is
required to contribute 0 or 1; this is an authenticated producer contract, not a
new Core type invariant. Core validates integer values, but does not currently
validate each contributor's nonnegative domain. The aggregate negative-total
guard prevents that particular malformed input from proving absence; it cannot
reject forged cancelling +1/-1 contributors. No such producers are admitted by
this slice. A future real flag-producing family may justify a dedicated Boolean
aggregation or contributor-domain contract.

Empty reductions are valid only after the evaluator proves complete incoming
membership. The release retains Partial owners and routing, so original builds
cannot infer absent modifiers or false flags from this finite slice. No raw
PoB output value is used as a production input, and no hard-coded build or actor
level exists in the production path. Actual game constants and the complete
arithmetic are injected rule data.

## Validation and limits

The native component fixture reuses the unchanged intrinsic attack fixture and
its dynamic final-parent-level → actor-level → intrinsic weapon-data chain. It
adds the exact configuration producer and these authored programs. Its Encounter
contains only the new block-configuration program; omitted legacy encounter
programs are a finite component selection, not a proof of completeness for the
real Encounter. Only this
finite test world certifies otherwise empty membership and supplies final parent
levels explicitly. Real final-level producers and the parent Action-context
readiness requirement are still incomplete; this stage does not rewrite real
requests or make level-dependent damage available from missing inputs.

Tests cover fractional and out-of-range block values, additional block sources,
clamp-before-reduction order, action-specific reductions on two independent
actors, true inheritance refusing the default result, missing inputs/producers,
negative flag totals, restored Partial coverage, scratch reuse, and parallel
execution. Numeric source parity is authenticated separately through fresh full
PoB loads in both JIT modes. The committed calibration projects 13 Sniper consumers
from the 31-case, 32-load source witness; a separate native test replays those
measured inputs through these programs. The ignored publication join authenticates
the complete source evidence and every copied prerequisite definition, slot,
program, table, and preserved intrinsic route before closing the finite world.
Source validation receipts live in `authoring.json`. No final damage, readiness,
complete actor, or whole-build parity is claimed.
