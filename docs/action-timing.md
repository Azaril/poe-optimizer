# Shared action speed and ordinary action timing

The native actor resolves action speed once for movement and supported direct actions.
Both current Spark/Mace pipelines consume that result and the injected server-tick limit.
This is a shared calculation stage; it does not establish support for every skill, actor,
trigger or support interaction. Delivery evidence and remaining work are in the
[living implementation record](implementation.md). The next phase is
[breadth validation](breadth-validation.md), before another individual-skill port.

## Injected data and source admission

Package schema **11**, semantics `poe2-native-profiles-v11`, adds `action_speed` and
`direct_action_timing`, bringing the package to **21** sections. Baseline multiplier,
minimum default, Temporal Chains cap, percentage divisor, query order, multiplier rounding,
server-tick rate and ordinary-action eligibility come from selected data. The movement
section no longer owns an independent neutral action-speed default.

Actor records represent ActionSpeed/TemporalChainsActionSpeed INC, MinimumActionSpeed/
MaximumActionSpeedReduction MAX and UnaffectedBySlows FLAG. Reviewed minimum phrases retain
GlobalEffect/unscalable metadata. The direct authored grammar includes ordinary integer
increased/reduced Action Speed, source-supported attribute conditions, plain minimum aliases,
minimum-percentage phrases and unaffected-by-slows wording. Item formatting precedes parsing.
Fractional ordinary authored percentages are rejected because the reviewed source grammar
accepts integer captures; numerical source records remain a distinct input seam.

The raw actor operations represent Temporal Chains and maximum reduction, but actual curse,
enemy, party and linked-skill producers are not generally admitted yet. A raw operation is
not whole-build mechanic support. Local item records, generated movement penalties and
surviving globals retain source order and ownership. Unsupported lines are never dropped.

## Shared numerical semantics

The MAX query returns an optional positive maximum; no matching positive row remains absent,
including when an authored MAX record explicitly contains zero. Original ModStore performs
its second evaluation in the requesting store context. The positive-sum query filters each
matched evaluated row before summing; it must not clamp a grouped sum after cancellation.

Action speed follows the original order: sum ordinary or positive-only INC rows according
to UnaffectedBySlows, apply the Temporal Chains reduction cap, add the baseline, apply the
minimum, then apply an optional maximum reduction. Contradictory bounds retain that source
ordering. No extra action-speed rounding is introduced. Prepared actors retain the MAX
presence, queried contributions and resolved flag as evidence.

Ordinary direct-action timing rounds the skill speed multiplier, calculates the reciprocal
of base time plus additional action times, then applies the shared action speed. **CastRate
is the rate after action speed and before the server cap.** Speed is capped at the injected
server tick rate times repeats. Time is zero when Speed is zero, otherwise its reciprocal;
DPS uses final Speed. The pinned tick rate is `1 / 0.033`, not an assumed 30. Complete current
profiles require the supported ordinary self-cast branch and one repeat. Channel, cooldown,
trigger, totem/trap/mine, reload and repeat producers require their complete dependencies
before admission.

Prepared actor/action/movement outputs retain the existing finite-result admission boundary.
A finite prepared actor can still produce nonfinite derived timing (for example, a very
small positive Speed can overflow Time). Timing evidence represents those values explicitly
with a mandatory `non_finite_values` kind map alongside null numerical fields; null alone
is not an availability decision. Typed metrics use the shared nonfinite classification.

## CLI and evidence

```powershell
cargo run --no-default-features --locked -- evaluate tests/fixtures/builds/mace-action-timing.xml --metric player.action_speed_pct --raw
cargo run --release --no-default-features --locked -- search-build --problem examples/action-timing-search.json --jobs 4 --max-evaluations 1000
cargo run --release --no-default-features --locked --example benchmark_assembly -- --problem examples/action-timing-search.json --sample-ms 700 --repeats 3 --jobs 1,2,4,32
```

These are caller-loaded example files; no example character is embedded in normal evaluation
or search. Required skill/item subsets, locks, inventory, objectives, budgets and encounter
assumptions remain problem data. The example combines damage, receiving, movement and action
speed constraints; those choices are not universal requirements.

Player `action_speed_pct`, definition schema 1 and unit `percent`, is `100 * ActionSpeedMod`:
100 is baseline and 120 is 20 percent faster. It is distinct from increased action speed,
movement speed, final attacks/casts per second and the server cap. Both backends expose the
same player-only metric. It is appended after the existing native thirteen metrics and the
existing PoB nineteen definitions.

Graph problem **11** yields report **12**, scope `action_timing_native_search_v1`. Earlier
graph schemas and legacy mutation schemas reject authored action-speed scope, including
unselected supplied inventory. As with prior authored-mechanic gates, the selected package
separately determines passive semantics: an AllowCustom passive change is not inferred from
the problem version. Exact data identity and capability admission remain mandatory.

Native profile IDs are `poe2-spark-action-timing-v7` and
`poe2-mace-strike-action-timing-v11`, media versions **7 / 9**. Both include `action_speed`
and `action_timing` evidence schema 1; movement and receiving evidence remain schema 1.
Fresh realization checks evidence against privately admitted source/data and preserves exact
XML exports and their matching data companions.

PoB raw diagnostics retain exact `MainHand.CastRate`, `MainHand.Speed`, `MainHand.Time` and
corresponding OffHand paths when source outputs exist. Attack CastRate is not generally a
flat aggregate field; these paths prevent substituting capped Speed for the uncapped rate.
They are diagnostic source paths, not new public objective definitions.

## Validation requirements

Original cold/warm ModStore, actionSpeedMod and the complete offence timing branch are
independent oracles. Cover mixed signs/layers, absent/zero MAX, current-store re-evaluation,
flags, conditions, floors/ceilings, zero/tiny/high rates, tick neighbors, custom numeric data
and all admitted support combinations. Whole builds must compare shared movement and offence,
hand timing, typed metrics and exact exports/reimports. Repeated changing actors/skills must
retain direct parallel execution without allocations in the prepared calculation loop.

Small numerical fixtures remain useful for diagnosis. Full replacement requires the broader
corpus and held-out whole-build evidence described in [breadth validation](breadth-validation.md).
