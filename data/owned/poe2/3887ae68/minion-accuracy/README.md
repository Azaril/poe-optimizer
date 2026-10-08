# Ordinary minion attack hit chance

This directory retains the original acquisition and numerical evidence for the
native Sniper population and its `MinionMeleeBow` Basic Attack. The current
[Boolean replacement packet](../minion-accuracy-flags/README.md) supersedes the
two integer flag schemas and consumer bodies in `extension.json`. Those old
bodies are offline exact-inverse/source-proof inputs, not selectable current
runtime behavior.

The calculation uses the pinned PoE2 behavior: `CalcPerform.lua:1072–1076`
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
| 3219 | MinionAccuracyEqualsAccuracy Boolean contributions | Player actor |
| 321a | Intrinsic CannotBeEvaded boolean | Owned actor |
| 321b | Intrinsic accuracy hit chance | Owned actor |
| 321c | Routed accuracy hit chance | Exact Basic Attack action |
| 321d | reduceEnemyBlock BASE contribution channel | Action |
| 321e | CannotBlockAttacks Boolean contributions | Enemy |
| 321f | Effective enemy block | Action |
| 3220 | Hit chance after block | Action |

The current flag channels use typed `Flag` contributions and checked, unordered
`Any` queries at those same IDs. Their identity is Boolean false, valid only for
complete incoming membership. Integer ADD presence counts and their negative-total
guards are retired from current evaluation. Numeric values cannot enter a Flag
channel; true does not conceal an active unknown contributor. Inactive sources
are skipped, while duplicate active sources retain their separate identities.

The release retains Partial flag-source inventories, owners and routing, so original builds
cannot infer absent modifiers or false flags from this finite slice. No raw
PoB output value is used as a production input, and no hard-coded build or actor
level exists in the production path. Actual game constants and the complete
arithmetic are injected rule data.

## Validation and limits

The maintained native integration is `tests/owned_sniper_item_attack.rs`, with
accuracy controls in `tests/support/owned_sniper_accuracy_native.rs`. It joins
canonical imported Crown/Solar rolls, actual source preparation and final-input
assembly, population, intrinsic weapon data and hit chance on one checked graph.
It consumes the published Boolean programs and current configuration import.
It does not supply precomputed final parent levels or require an artificial
parent Action query. The old Engine `owned_minion_accuracy` test and its exclusive
`minion_accuracy_fixture` helper have been deleted; no parallel staged accuracy
world replaces them.

This graph still declares a finite component inventory. Its selected Encounter
contains the block-configuration program; other Encounter behavior and real
flag-source membership remain unproved. Missing inputs cannot become known
damage, and restored Partial coverage prevents a completed result.

Tests cover fractional and out-of-range block values, additional block sources,
clamp-before-reduction order, action-specific reductions on two independent
actors, typed false/true/duplicate/unknown flags, true inheritance refusing the
ordinary result, wrong numeric values and recipients, missing inputs/producers,
restored Partial coverage, scratch reuse after unresolved results, and Rayon
execution. The sixteen joined tests pass, together with the Sniper and Offering
regressions and the Boolean publication/source-authentication checks.
Two ordinary Engine tests in `owned_minion_accuracy_rules` retain rule execution
and all thirteen source controls in CI using explicit resolved facts. They do not
replace joined membership/routing checks: the joined target is currently ignored
in ordinary CI until its published package is reproducibly provisioned there.

The committed calibration preserves all thirteen Sniper consumers from the
31-case, 32-load source witness. Eleven ordinary observations now match the
joined graph, including MAIN/CALCS and imported block controls. One authenticated
custom CannotBlockAttacks observation is replayed with an explicit finite typed
contributor; this proves the consumer law, not custom-modifier import. The raw
level 40 observation remains a source-clamping diagnostic: real item bonuses
produce 42, which native preparation refuses rather than pretending its input
was 40. The separate intrinsic component retains the final-level-40 numerical evidence.
Source validation receipts remain in `authoring.json`; retained JIT-on/off reports
are authenticated without running a source VM for the native integration.

The historical root publication test `tests/owned_minion_accuracy.rs`, its helper,
and the optional PoB source witness remain useful acquisition/parity consumers.
Retire the historical publication orchestration and old integer bodies when a
maintained current-format acquisition path and retained independent proof replace
their exact-transition and source-law checks. Keep the optional source witness
while it validates PoB updates. No actual flag-producer coverage, inherited-player
accuracy, complete actor, final damage or whole-build parity is claimed.
