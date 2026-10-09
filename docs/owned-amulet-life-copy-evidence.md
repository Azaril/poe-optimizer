# Flat-Life Amulet copying: evidence and native handoff

Original05's flat-Life owner still has an Amulet-copy obligation. The existing
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

## Validation and next step

Run08 passes three tests in **40.41s**. Both comparison reports are 230,607 bytes,
SHA-256 `12956b5e5aba18aa246593597c44f20687257217566ed0f63144da0b1df00871`.
Strict all-feature Clippy passes in 0.27s. Evidence:
`runs/owned-amulet-life-source-08.log`, its report directory and
`runs/owned-amulet-life-clippy-01.log`. The existing LNK4098 source-runtime linker
warning remains; Clippy has no errors.

Reproduce with a fresh `POE_OPTIMIZER_TEST_AMULET_LIFE_SOURCE_OUT` directory and
`cargo test --locked -p poe-optimizer-pob --test owned_amulet_life_copy -- --include-ignored`.

Next, author the copy using existing typed arithmetic, explicit Count-to-Life
units, exact modifier/equipment occurrences, eligibility `32e3`, pre-copy snapshot
`32e4` and frozen stages. Add its potential writer to checked Life membership.
Preserve Partial equipment reduction and unrelated owner gaps. Validate actual
native rules against these controls, duplicate/removal and unknown inputs,
staging, overflow, scratch reuse and parallel execution. Then retire only the
proved copy-consumer obligation and continue the remaining Life dependencies.

This source checkpoint changes no native artifact, API or import permission;
complete native builds remain 0/5. The
[implementation plan](implementation.md#latest-source-checkpoint-flat-life-amulet-copying)
records the current resume point.
