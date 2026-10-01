# Fine Belt numerical inputs

This profile adds the two missing physical source families on the original
Fine Belt: ranged `Has (1-3) Charm Slot` and fixed decimal
`Flasks gain 0.17 charges per Second`. The recipes accept bounded source values;
neither amount is a runtime constant. Existing fixed `Has 1/2/3 Charm Slots`
lines remain outside this ranged profile.

The checked append uses 53 IDs, `3172` through `31a6`:

| Family | Modifier | Required rolls | Units and effective output |
| --- | --- | --- | --- |
| Ranged charm slots | `3172` | 25, through `318b` | Existing Count `295a`, Modifier output `295b` |
| Fixed flask charge rate | `318c` | 24, through `31a4` | New Rate `31a5`, Modifier output `31a6` |

Both families retain the existing 20 source properties, unscalable flag,
corruption factor and exact source category alongside the raw amount. Charm has
an additional Boolean for the exact source tag `charm`; it is not an alias for
an existing property or inferred from the English line. All source flags without
a destination remain rejected. Each owner contains three authored scalar
programs and one program from the existing modifier-value compiler. Its rule
coverage stays Partial.

Charm uses precision 1 and display precision 0. Flask charge rate uses precision
60 and display precision 2. The source first formats fixed decimals too: raw
`0.125` becomes `0.13`, and `0.001` becomes a complete independent BASE 0 record.
The canonical raw amount retains the original decimal before the native
formatter. Fixed flask zero is admitted; negative text is not. Charm endpoints
remain positive because the source zero range produces no CharmLimit record.
Unsupported scaling tags, unconsumed properties, unknown source members and
unproved whole-item categories withhold the canonical emission.

## Preserved boundaries

The charm component is local to the item. The source consumes it into derived
item capacity and later delivers that capacity once. This profile does not add
a CharmLimit contribution or treat the `Charm Slots` display header as a second
producer. That existing, scoped header observation is preserved. Flask rate also
has no new contribution, final consumer or metric binding here. Native component
tests use the exact compiled programs in an explicitly finite test topology.

The resulting Proven layout can admit the existing Life line and the reviewed
absence of item level and ordinary quality. The absence facts apply only to the
Fine Belt template and only through the existing source-default proof. Physical
parameter inventory, modifier inventory and order remain Pending: the actual
two-implicit/one-explicit shape is outside the existing singleton and paired
membership profiles. Template, owner and routing gaps remain unchanged.

`authoring.json` binds the exact Sapphire predecessor and nine files in the
complete pinned source manifest. The optional source test executes unchanged
source loading, parsing and modifier-list methods in both JIT modes. It separates
fresh parsing from fixed-line cache behavior after later scalar mutations; this
profile does not generalize those cache histories.

## Reproduction

```powershell
$env:POE_OPTIMIZER_TEST_FINE_BELT_PRIOR = 'runs/owned-sapphire-item-inputs-03/package'
$env:POE_OPTIMIZER_TEST_FINE_BELT_OUTPUT = 'runs/owned-fine-belt-modifiers-02'
cargo test --locked --test owned_fine_belt_modifiers -- --include-ignored
```

Use a new output directory. The helper compiles both numerical programs, performs
the checked membership append and prior-binding transition, then installs the
new checked source policies. It restores every permitted change and compares the
entire predecessor input. Publication tests verify rebuild bytes, all five
original imports and saved selections, every prior query byte, and the unchanged
predecessor inventory. Among the unchanged originals, only original-05 source590
matches these two new recipes. No original build is declared complete.

## Verified publication

The default authoring test and explicit real publication test passed, including
23 source/header/value probes. The successful output is
`runs/owned-fine-belt-modifiers-02`; the first attempt is retained separately as
evidence of a comparison-harness correction. No authored data or runtime guard
was changed to resolve that expectation.

| Commitment | SHA-256 |
| --- | --- |
| Full input | `9b9f25511f05f2963c5f46cd0cea0b921ab2c1335407a95b0045453183dbc074` |
| Definitions | `ff79ab2c30af23f56354e3fbf585a493c285c8cb4119613fe979d94d7fd2e8fb` |
| Registry | `d569199bbf5cf8ff74a32932a619041a141f05160e8ed7500c1547abe3fce3c4` |
| Item-line policy | `5cb648ed82a1ea203f26159643703bd0e2ebc48e2c4362fa5b7aa467f3b79822` |
| Item-source policy | `7dfba1d50e10f4aed84e01ffb3f3228f85de76dcafb5a07b872d9000ca4b6de5` |

The package has 18 files, 59,793,497 bytes and 31 provenance entries. Rebuilding
produces identical bytes, all 110 queries are unchanged, and the predecessor
inventory remains unchanged. The five selected issue counts are
**127 / 128 / 120 / 158 / 32**. Only Original05 changes: three new canonical
members (CharmLimit, FlaskChargesGenerated and existing Life) and exactly two
retired item-level/quality absence obligations, from 34 to 32. Its parameter
inventory, modifier inventory and order remain Pending. All five original
builds remain Pending, calculation `not_run`, with zero complete native build
evaluations.

The exact reports are `validation.json`, `package/release.json` and
`original-01-selected-report.json` through `original-05-selected-report.json`
inside that output. The publication log is
`runs/fine-belt-publication-tests-02.log`. Numerical component validation is
recorded separately and does not change these original-build results.
