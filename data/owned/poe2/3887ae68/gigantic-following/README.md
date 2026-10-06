# Gigantic Following: default grants and Sniper status

This packet extends the exact `pob-3887ae68-minion-life-passives-v1` predecessor
(`ee4cc6c44202f2174129e12e782a02cd65c5a993c6ccffc87e779460fb7be09f`). It
publishes the complete default body of passive `46365` / owned `1532`, preserving
both independently observed records:

| Channel | Native meaning | Default contribution |
| --- | --- | --- |
| `3307` | Integer count of sources granting Gigantic to minions | Player `Add 1` |
| `3308` | Boolean Gigantic status on a reviewed recipient | Sum player `3307` grants, then compare greater than zero |
| `3309` | Percentage reservation efficiency for minion skills | Player `Increase -25%` |

The count is an internal aggregation measure; multiple grants still produce one
Boolean status. The existing typed `Contributions`, `Sum`, `Compare` and `Derive`
operations express that behavior without a new runtime operation. The single
published receiver targets actor slot `001f`, declared by skill `0012`, whose
reviewed source profile is `RaisedSkeletonSniper`. Other minion profiles are not
included in this delivery inventory.

The seven empty default declaration inventories are refined to Complete after
checking the entire source node, catalog/mapping row and source construction
rules. This closure does not prove that a selected occurrence has no effective
transform, close incoming contribution inventories, or complete the whole build.
The packet adds three definitions and three programs, with no query, routing,
Action-owner or support-demand changes.

`source-vectors.json` binds two existing independent witnesses. The passive
default Source02 report preserves the full typed-key source graph, including the
mixed numeric/string reservation modifier table, all parser returns and warm/
fresh JIT checks. The original physical damage Source02 reports bind JIT on/off,
fresh/repeat/warm Original05, removal of Gigantic, and CALCS display scopes. The
publication helper authenticates report bytes, original build bytes, source
manifest/files and exact projections before applying the ordinary V5 migration
and passive refinement. These source graphs are offline test evidence, not the
native runtime format.

Life/damage numerical factors and delivery of reservation efficiency to the
parent minion-summoning occurrence remain subsequent work. The current reservation
consumer is Action-owned; the [ownership proposal](../../../../../docs/owned-resource-obligations-proposal.md)
is pending. The source MAIN and CALCS
combat frames include Gigantic's numerical benefits, while CALCS buffed and
unbuffed scopes retain its flag without those benefits. That is a reference
calculation/display-scope distinction, not evidence for adding a native game
"in combat" condition. No parent Action, query or usage entry is fabricated to
make a consumer reachable. No whole-build parity is claimed.

`tests/support/owned_gigantic_following.rs` provides the bounded publication and
evidence gate. `tests/owned_gigantic_following.rs` exercises the authored programs
with the native evaluator and exposes the opt-in publication check. Generated
packages and full source reports remain ignored under `runs/`; publication/test
results are recorded at the implementation checkpoint.

Publication01 passes in 29.60 seconds, preserving all five unchanged originals
and their 110 queries, with eighteen files rebuilt byte-identically. The five
native checks pass in 14.95 seconds, including duplicate grants, missing/Partial
coverage and four-worker replay. The authored-data check and strict Clippy pass.
The new baseline is `runs/owned-gigantic-following-01/package`, input
`84e348f2c5e02ccb722ac68a3f071f8bb06eea75a0fbea7c2269b16144ddd889`.
Selected Original05 default passive coverage is 42/55; complete native originals
remain 0/5. See the [living implementation plan](../../../../../docs/implementation.md)
for reproduction and remaining inputs/consumers.
