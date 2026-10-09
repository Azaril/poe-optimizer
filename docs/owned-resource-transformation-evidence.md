# Resource transformation source evidence

Status: reproduced source behavior and native-adoption guidance, 2026-10-09.
No native transformation implementation or new parity exception is approved by
this evidence. The final Mana consumer remains an unpublished draft.

## What the witness executes

`crates/poe-optimizer-pob/tests/owned_resource_transformation.rs` uses the existing
full PoB bootstrap with the pinned, unchanged source. Controls change only the
selected ConfigSet through a reversible XML insertion; the original parser,
modifier stores, item properties and calculation functions still run. Nothing
injects precomputed outputs into the reference calculation.

Each case compares the initial load with exactly three normal `BuildOutput`
calls. The middle rebuild uses a read-only line observer to copy the original
Player's resource tables before and after `CalcDefence.lua:1375–1428`; the two
other rebuilds have no observer. Exact current MAIN/CALCS environments, original
method identities, hook removal and unchanged numerical results are checked.
Fresh-repeat and a changed-build/warm-restoration control also agree. This is
not retry-until-pass and establishes resource results, not whole-build parity
or coverage of every possible JIT trace.

The full run covers **19 cases and 20 complete loads per configured JIT mode**:
all five originals, twelve conversion/gain controls, a fresh repeat and warm
restoration. The two comparison reports agree byte for byte.

## Findings

| Control on Original05 | Original observed result | Consequence for native work |
| --- | --- | --- |
| Convert 25% of ES to Mana | Local ES totals 101; ExtraMana 25.25; final Mana 665 | Use pre-scaling item/global donors. Actor ES 146 would be the wrong donor. |
| Add 100 global ES, then convert 25% | ExtraMana 50.25; final Mana 691 | Global and slot inputs both contribute before Mana scaling. |
| Convert 150% of ES to Mana | Per-edge rate caps at 100; ExtraMana 101; final Mana 744 | Keep raw rate, capped rate and resulting contribution distinct in evidence. |
| Convert Evasion 100% to Armour and 50% to ES | Donor total caps at 100, but destination rates remain 100/50. Global donor 107 supplies 107 Armour and 53.5 ES. | The string-keyed table's `ipairs` normalization does not normalize these rates. Do not adopt that iteration behavior as a native law. |
| Convert 25% Armour to ES, then add 25% Armour as Ward | Boots' 161 Armour becomes 90.5625; Ward receives 30.1875. Conversion alone leaves 120.75; gain alone leaves 161 and supplies 40.25 Ward. | Each positive destination depletes slot bases again, even when the additional destination is gain-as. |
| Reverse those two custom-modifier lines | Same resource intermediates and outputs | The observed behavior follows the fixed resource pass, not the authored line order. |
| Gain Mana as ES, alongside ES-to-Mana conversion | ExtraMana stays 25.25; later Mana-to-ES adds 158 global ES | This is one ordered pass, not a simultaneous or fixed-point solution. Intended game chaining still needs independent authority. |
| ES-to-Mana with a zero Mana override | ExtraMana remains 25.25; final Mana is 0 | An override does not erase donor production or source-coverage obligations. |

These custom modifiers are accepted by the actual parser and reach the actual
resource pass. That is **not proof that every combination is obtainable in the
game**. The normalization and repeated depletion are reproduced candidate source
defects, not accepted game laws or authorized numerical parity exceptions.
Positive Total-channel production is also not established by these controls.

Original04 already converts Evasion to Armour. The five unchanged builds have
zero additional Mana in this witness; they are not five globally conversion-free
builds. Numerical zeros alone do not certify absent or completely known sources.

## Determinism and diagnostic ordering

Run03 exposed swapped raw Evasion record positions for Wind Dancer and Dexterity
Mote in Original02 across fresh VMs. Every numeric output, rate and transformation
intermediate agreed. Within each VM the initial/rebuild snapshots also agreed.

The witness retains raw record sequences in separate diagnostic files. Its
comparison uses complete source-qualified record multisets, preserving duplicate
counts and every record field. Only list order changes; no numerical value,
transformation sequence or table intermediate is sorted, rounded or tolerated.
This observation-only ordering is not a native accumulation law. It follows the
same distinction already recorded for Amulet-copy evidence; it grants no wider
determinism exemption and does not address the separate Frost initialization bug.

## Shortest native adoption path

Original05 remains the closest target. Its neutral transformation domain can be
reviewed using current contracts; implementing all positive transfers first is
not required. Authenticate the actual composed source bodies and declarations,
including selected Class/Ascendancy, shared Player, item/template/modifier inputs,
passives, sockets, generated skills, supports, configuration and rewards. Reuse
the existing normalization inventories, item-source layouts and checked queries.

The current five Complete-empty adjustment groups only cover admitted writers.
Their census does not account for untranslated mechanics. Adding an inactive,
zero-valued or application-based unknown supplier must still refuse admission.
Keep owner/global Partial gates until their own work is complete; neither a
negative source proof nor final-Mana arithmetic closes unrelated owners. In
particular, an empty passive description is not an empty socket inventory.

Positive ES-to-Mana work should extend actual final item ES properties across
Robe/Cuffs/Crown/Leggings, then collect exact pre-scaling donors and rates in the
existing typed graph. The existing local-defence value `330d` precedes per-level
and property-override stages. It is not automatically a final donor. Eldritch
Battery `188a` is not selected in Original05, so its implementation alone would
not close that baseline. Game data, recipient selection and arithmetic belong
in injected rules; source record names and mutable Lua resource objects remain
offline evidence only.

## Validation and reproduction

Run05 passes in **77.03s**. The comparison reports contain **1,135,040 bytes** and
have SHA-256 `5b9d8fd01c96d3839d4311ed64f1d8886387b4e04ddfc130deddb44ed786e35f`.
The existing optional-runtime LNK4098 linker warning remains.

- Full local evidence: `runs/owned-resource-transformation-source-05/`.
- Retained comparison: `tests/fixtures/owned-resource-transformation-source.json`.
- Raw order diagnostics: `raw-record-orders-jit-{off,on}.json` in that run.
- Ordinary Rust check: `retained_resource_transformations_match_current_source_and_driver`.
  It verifies source/observer/driver pins, all five original XML identities and
  the exact controls without executing Lua.

Both ordinary Rust tests pass in 0.01s, including negative comparison controls
for changed values/sources/tags, missing duplicates and transformation order.
Strict all-feature Clippy passes; selective formatting and whitespace checks
pass. Logs: `runs/owned-resource-transformation-retained-02.log` and
`runs/owned-resource-transformation-clippy-02.log`.

Set `POE_OPTIMIZER_TEST_RESOURCE_TRANSFORMATION_SOURCE_OUT` to a fresh directory,
then run:

```text
cargo test --locked -p poe-optimizer-pob --test owned_resource_transformation original_resource_transformations_preserve_full_source_evidence -- --ignored --exact
```

The optional PoB crate owns the witness. No production Core/Data/Engine code,
runtime package or definition ID changes here. Complete native builds remain
0/5; the numeric-domain decision and selected-owner completion work remain open.
