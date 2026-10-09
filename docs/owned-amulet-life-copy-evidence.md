# Flat-Life Amulet copying: evidence and native adoption

Original05's flat-Life owner now has an injected Amulet-copy consumer. The existing
Minion-level copy consumer cannot be transferred unchanged: its nested level
record and an ordinary numeric Life record take different branches of original
`ModStore:ScaleAddMod`. This is a dependency of equipment Life coverage, not a new
public model or an alternate evaluator.

The optional Rust witness
[`owned_amulet_life_copy.rs`](../crates/poe-optimizer-pob/tests/owned_amulet_life_copy.rs)
reuses the complete-source resource loader. Its observer replaces no calculation
method and installs no debug hook. Five unchanged originals, four parsed Amulet
controls and a fresh replay run in each JIT mode. A changed-to-original warm
replay adds two loads, for 12 complete loads per mode. The control builder edits
the existing raw-item text before `ModRange` children and preserves those children;
a later text node would instead replace the raw item in PoB's loader.

## Copy behavior

The full-build controls add parsed Life lines to the selected Amulet and use the
parsed ring/Amulet bonus-effect modifier. These are mechanical controls, not proof
that the custom combinations are obtainable rolls.

| Added Life records | Bonus percentage | Separate copied records |
| --- | ---: | --- |
| 17 | 0 | 0 |
| 17 | 25 | 4 |
| 17 | 100 | 17 |
| 17 and 19 | 25 | 4 and 4 |

The last case proves per-record scaling: combining 36 first would give 9. Direct
records remain distinct from copies, including the zero copy. Nine detached
scalar probes call the authenticated original method in fresh original ModDB
instances, preserve inputs and repeat each call. Their broader values and record
bypass tag are source-contract probes, not importer or gameplay admission.

| Effective value | Factor | Copy | Behavior |
| ---: | ---: | ---: | --- |
| 17 | -0.25 | -4 | Truncation toward zero differs from floor |
| 17 | 0.9999 | 17 | Decimal rounding before truncation differs from floor |
| 17.5 | 0.25 | 4.3 | Fractional input selects default precision 1 |
| 17.5 | -0.25 | -4.4 | Fractional branch floors at that precision |
| 17.5 | 1 | 17.5 | Identity preserves the value |
| 17 with the record bypass tag | 0.25 | 17 | Bypass preserves the value |

The displayed near-integer factor is produced as `99.99 / 100` in source run09.
Native tests supply the equivalent percentage and require the resulting factor's
bits to match. A literal decimal `0.9999` can differ after percent conversion;
merely obtaining the same rounded answer would not establish operand parity.

Pinned `Modules/Data.lua` has no Life/BASE precision override and sets default
precision 1. `ModStore.lua:82–121` handles identity/bypass first, then precision
metadata and the effective value's fractional part. Integral Life rounds the
scaled value to two decimals and truncates; fractional Life floors at the chosen
precision. This is deliberate source arithmetic. It is not authority to add Lua
truthiness, dynamic tables or record types to native execution. Independent game
evidence for the diagnostic fractional/negative inputs remains unconfirmed.

## Determinism scope

Run06 exposed different storage order for the two **ring** bonus records in the
25% control across JIT modes. `CalcPerform.lua:1489` uses `pairs` over item slots
for those later copies. Values, multiplicity, source identities, Amulet-copy
order and final Life matched exactly. No numerical fluctuation was observed.

Raw observations remain in `case-{off,on}-*.json`, including warm restoration.
The comparison report explicitly treats complete read-set buckets as
source-qualified multisets, retaining duplicates and every record field. Direct
and copied Amulet sequences, all numbers, MAIN/CALCS agreement within a load and
fresh/warm results remain exact. There is no tolerance or retry-until-pass.
Ordinary tests reject changed values, duplicate removal and reordered Amulet
copies. **Raw source sequence parity is not claimed.** General equipment-Life
accumulation order remains unproved; this diagnostic sorting is not a native
numeric fold order. The separate Frost initialization issue is unaffected.

## Validation and native adoption

Run09 passes three tests in **39.83s**. Both comparison reports are 230,739 bytes,
SHA-256 `66c27ef862dc688eda80721710d61a204e4b24b09ca52637694c9e319ab97153`.
Evidence is in `runs/owned-amulet-life-source-09.log` and its report directory.
The existing LNK4098 source-runtime linker warning remains. Strict all-feature
Clippy for the native and source targets passes in **0.49s** in
`runs/owned-amulet-life-copy-clippy-01.log`.

Reproduce with a fresh `POE_OPTIMIZER_TEST_AMULET_LIFE_SOURCE_OUT` directory and
`cargo test --locked -p poe-optimizer-pob --test owned_amulet_life_copy -- --include-ignored`.

The [published copy data](../data/owned/poe2/3887ae68/amulet-life-copy/README.md)
now uses existing typed arithmetic, explicit Count-to-Life units, exact
modifier/equipment occurrences, eligibility `32e3`, pre-copy snapshot `32e4` and
frozen stages. Its potential writer joins checked Life membership. Numeric Life
delivery has execution readiness; gem-level preparation does not dictate its
phase. No runtime API or import admission is changed.

All nine native/publication tests pass in **23.67s** in
`runs/owned-amulet-life-native-publication-09.log`. They cover these source vectors,
duplicate/removal and unknown inputs, exact recipients/units, staging, overflow,
lazy branches, storage permutations, scratch reuse and four-worker execution.
Publication preserves all five imports and rebuilds 18 artifacts identically.
Only the copy-consumer obligation is retired. Partial equipment reduction and
the five remaining flat-Life owner gaps still block complete resource coverage;
complete native builds remain **0/5**. The
[implementation plan](implementation.md#latest-data-checkpoint-native-amulet-life-copying)
records the current package and next dependency.
