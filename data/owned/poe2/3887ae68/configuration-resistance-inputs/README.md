# Configured Enemy resistance BASE inputs

This family contributes the four configured Enemy resistance BASE values. It
preserves raw override presence separately from the optional raw quantity. It
does not calculate final resistance, resistance caps, penetration, damage or any
original-build metric.

`extension.json` appends twelve definitions, refines Encounter `31d1` with eight
explicit external-input members and appends four ordinary Enemy-context programs.
The prior external-membership and program-owner Partial gaps remain unchanged.
There is no contributor-coverage closure, receiver or final metric binding.

`native-inputs.json` identifies the exact source name, Boolean presence input,
optional raw PercentagePoints input, BASE contribution stat, program and injected
default for each family. Allocation order is:

| Family | Presence | Raw value | Contribution |
| --- | --- | --- | --- |
| Fire | 31d2 | 31d3 | 31d4 |
| Cold | 31d5 | 31d6 | 31d7 |
| Lightning | 31d8 | 31d9 | 31da |
| Chaos | 31db | 31dc | 31dd |

All quantities use existing unit `0002` (PercentagePoints), including the bounded
raw range -1,000,000 through 1,000,000. This range is adapter admission scope, not a
claim that the source clamps values to these endpoints. The ordinary program
selects raw when presence is true and the injected Pinnacle default otherwise:
50/50/50/0 respectively. A missing presence or a demanded missing raw value stays
unresolved. An absent raw value on the false branch remains absent; zero, negative
and above-cap explicit values are retained without clamping.

The pinned source is revision
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. `ConfigOptions.lua` has four
`countAllowZero` callbacks (2165–2180) emitting EnemyConfig-owned resistance BASE
records. Pinnacle supplies their default placeholders (2068–2072). The independent
complete lifecycle witness is `owned_configuration_inputs_source.rs`; its runtime
receipt, not this README or saved placeholder text, is the publication authority.
The witness passes 23 complete loads per JIT mode, including the five originals
and 18 controls. Its cross-mode evidence is byte-identical after excluding only
the cursor blink clock and unrelated downstream database order; full local
snapshots remain checked. The injected native programs match all 36 resistance
components from the five originals and four numeric controls, including exact
EnemyConfig-owned records and their MAIN/CALCS reductions. This is component
parity, not a complete build evaluation.

`CalcOffence.lua:685–698` consumes the BASE records but also consults explicit raw
Input values to calculate maximum resistance, and combines overrides, modifiers,
shared elemental resistance and bounds. Raw presence therefore must survive this
component. These programs do not imply those later producers are covered. Armour
and Evasion are excluded: their Pinnacle defaults depend on enemy-level tables,
and the current native read contract has no EnemyLevel read.

The Engine target `owned_configuration_resistance` loads these program bodies
unchanged. A separate finite fixture permits checking just this component; a
production-Partial assertion requires the contribution reduction to remain
unresolved. The optional parity test reads independently verified source JSON via
`POE_OPTIMIZER_TEST_CONFIGURATION_SOURCE`, comparing the five originals and four
source controls to exact EnemyConfig BASE records and MAIN/CALCS reductions. It
does not load PoB or establish Import admission for malformed source controls.
