# Player Mana adjustment inputs

This data packet adds five typed input collectors for the final resource formula.
It uses existing Stat definitions, checked contribution queries and Player
receivers. Production Core/Data/Engine code and operations V25 are unchanged.

| Definition | Owned meaning | Unit |
| --- | --- | --- |
| 3357 | Mana conversion to Energy Shield | percentage points |
| 3358 | Mana conversion to Armour | percentage points |
| 3359 | Mana conversion to Evasion | percentage points |
| 335a | Additional Mana before increased/MORE scaling | Mana |
| 335b | Additional Mana after increased/MORE scaling | Mana |

Each receiver reads its own checked Add/Sum group. The published groups currently
contain no admitted writers and resolve to zero only within that guarded domain.
Publication censuses all ordinary and application effects; adding a writer
requires reviewed query membership. Native planning rejects unlisted potential
bound writers even when inactive, while missing active values remain unknown.
Existing owner/global Partial declarations and complete-request gates survive.
This is not a universal assertion that the game has no resource conversions.

`bindings.json` records source observation keys for acquisition only. Those
strings are absent from native calculation rules. The pre/post distinction
preserves operation order and quantity units; these are not final resource
values or integer-only channels. The data contains no example-build selectors.

The optional full-source witness now also exercises conversion from Energy
Shield to Mana, with and without a zero override. PoB emits **25.25** additional
pre-scaling Mana in those controls, producing **665** ordinarily and **0** with
the override. Native conversion producers are not implemented by this packet;
the controls establish why an additional input and explicit override presence
are necessary. Existing 12 cases are retained unchanged. All 14 cases and 17
complete loads per JIT mode pass in 54.83s. The two 113,778-byte reports in
`runs/owned-mana-pool-source-03` have SHA-256
`9f13db531cbc18214f8468ac79714721d2536a7fca06e9f14567581d4fd3728e`.

The joined native tests check exact Player recipients/units, empty identities,
positive and inactive controlled producers, unknown activation, Partial groups
and unlisted inactive writers. Controlled producers are finite test inputs, not
game-data conversion formulas. Fresh/reused/four-worker tests now include zero,
unavailable, doubled and nonzero fractional cases followed by restoration.
Final Mana and complete build coverage remain open.

Run `cargo test --locked -p poe-optimizer-cli --test owned_intelligence_mana`.
Publication sets `POE_OPTIMIZER_TEST_MANA_ADJUSTMENTS_PRIOR` to the checked Mana
query package and `POE_OPTIMIZER_TEST_MANA_ADJUSTMENTS_OUTPUT` to a fresh output,
then runs `publish_mana_adjustments_preserving_all_five_originals` with
`--ignored --exact`. The schema-only migration precedes ordinary owned assembly;
the exact inverse removes only the five reducers, receivers and queries. All-five
import preservation and artifact reconstruction remain required.
Publication `runs/owned-mana-adjustments-publication-01` passes with 12 tests in
30.13s. All 18 artifacts rebuild identically; selected input counts remain
107/117/109/123/4. The final ordinary run passes 11 tests in 4.51s, and strict
all-feature Clippy passes for the native and source resource targets.

Numeric override selection awaits the separate
[design decision](../../../../../docs/owned-numeric-selection-proposal.md).
That proposal is not implemented here. Final arithmetic also needs validated
contributor bounds and rounding; neither neutral observations nor these input
collectors close that proof. The next free definition ID is **335c**.
