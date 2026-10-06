# Original Player attribute pipeline evidence

This source witness records the six original attribute evaluations for the five
unchanged imported builds and an exact attribute-choice control. It supports the
[staged contribution design](owned-contribution-stages.md); it does not publish a
native attribute reducer, close a contributor inventory, or choose a MORE grouping
policy.

## Validation status

`runs/owned-attribute-pipeline-source-05` passed in 303.46 seconds. Each fresh JIT
lane completed all seven cases, their hooked/unhooked comparisons, and the exact
independent Original05 restoration. The parent required the two aggregate reports
to be byte-identical. Both are 103,745,090 bytes with SHA-256
`1db2e0d271c386c932e268a4c9a04e522aa57a052d7ec9de4badf57c2e330764`.
The census below describes this passing source evidence, within the explicit
observation boundary.

The driver authenticates all nine report paths before loading builds, using the
manifested runtime file `src/TreeData/0_5/tree.lua`. Its normalized SHA-256 is
`e3350cd64e50976911cdc9fd2fc5f4cd650039697affefe33861cf63930cc66c`.

The earlier `runs/owned-attribute-pipeline-source-04` completed all seven JIT-off
case comparisons and exact independent restoration, then failed while constructing
its report because the driver requested an unmanifested `tree.json` pin. Its JIT-on
child never ran. The fail-fast pin correction changed report construction, not the
observer: all seven source05 JIT-off raw cases exactly match source04 as parsed JSON.
Runs 01–03 failed earlier while traversing nested non-scalar Player output objects;
the boundary correction is documented below. All failed logs and raw evidence remain
retained and are not substituted for the passing source05 reports.

## Observation boundary

The optional PoB test is
`crates/poe-optimizer-pob/tests/owned_attribute_pipeline.rs`, backed by
`tests/support/attribute_pipeline_source.lua` in the same crate. It authenticates
the original complete bootstrap and function identities, then observes original
`calculateAttributes`, `calcLib.val`, and `ModDB` reductions through a debug hook.
It does not replace business methods or calculate an expected result from a copied
formula.

For each selected MAIN and CALCS environment, the witness captures:

- all six original evaluations in their executed order, including before/after
  attribute and comparison snapshots;
- the complete relevant attribute/condition record ancestry, including tags,
  source identities, flags, zero-valued records and observed store depths;
- actual original BASE/INC/MORE return locals and any original getter invocations;
- every scalar top-level Player output field, with a complete sorted key/type
  inventory of excluded non-scalar fields and unchanged top-level identities.

Getter records distinguish an observed invocation from its diagnostic same-state
original replay. They do not claim the replay value is a captured return local.
Replay requires the observed attribute ancestry and scalar output to remain unchanged,
and getter targets must belong to the Player ancestry.

Hooked and hookless executions are compared within the same mode. MAIN and CALCS
are not required to match each other's complete output. Nested non-scalar output
objects are excluded; no full output graph or deep identity claim is made. Failed
runs 01–03 were diagnostic graph-traversal failures on Original02, before this
boundary was narrowed to the existing scalar-output witness pattern. Their logs
and raw evidence remain retained.

## Passing source05 census

Both passes return the same three attributes in every observed case. All six
queries execute BASE, INC and MORE; every observed MORE result is 1. All recorded
attribute ancestry is at Player store depth zero.

| Unchanged build | Strength / Dexterity / Intelligence | Nonzero increased attribute query |
| --- | --- | --- |
| Original01 | 82 / 32 / 156 | Intelligence: BASE 147, INC 6 |
| Original02 | 92 / 178 / 96 | Dexterity: BASE 165, INC 8 |
| Original03 | 85 / 147 / 117 | Dexterity: BASE 136, INC 8 |
| Original04 | 111 / 65 / 161 | None |
| Original05 | 27 / 7 / 105 | None |

Original05 has 25 untagged BASE records: class values 7/7/15 and 22 selected
attribute-choice contributions of 5. Strength receives four choices, Dexterity
none, and Intelligence eighteen. INC is zero in all six queries. No GetStat or
GetCondition invocation occurs during these evaluations. Its local entry comparison
fields are absent but unqueried; that does not authorize a universal false C0.

The control changes only active tree preset 3's attribute override for node 15782
from Strength to Dexterity. It preserves allocation IDs, topology, class,
ascendancy and all other XML bytes, with an exact inverse. The observed record
change in both modes is precisely `Tree:15782 BASE +5 Str` becoming
`Tree:15782 BASE +5 Dex`. Both passes then return 22/12/105. The separate restored
original reproduces the entire original05 hooked host exactly.

MAIN and CALCS Original05 provenance agrees after excluding lifecycle query sequence
counters. Those counters retain actual observation order and are not attribute
semantics.

Original02 has ten getter calls per mode: eight `WeaponSet1` calls replay a single
nil result, and two `WeaponSet2` calls replay true. All target Player depth zero;
none reads an attribute-comparison C0 condition or GetStat. Tagged passive
occurrences therefore remain a real activation input even when the observed pass
outputs agree. The report preserves nil separately from false.

Across the other originals, increased attributes and derived ring/amulet records
show why a general class-plus-choice subtotal is incomplete. Original03 includes
ring-derived Strength +8 and Intelligence +7. Zero-valued derived amulet records
also remain present in the source inventory; they must not disappear from a
contributor-membership census merely because they do not change a sum.

## Implications for native adoption

Original05 demonstrates a useful actual-build numerical slice using existing class
and choice occurrences. The source record order is not a universal node-ID sort.
These finite, bounded positive integer additions are exact, but this does not prove
that arbitrary fractional or condition-dependent contributions may be regrouped.
The current Count cutover preserves individual contributions without publishing
that broader law.

Before a native final attribute receiver can be called complete, its stage-local
membership and activation inputs need explicit authority. Missing inputs must not
become zero or false. The all-five trace also does not establish general C0
initialization, query-limit semantics, a MORE grouping policy, or native final Life.

## Reproduction

Use a fresh output directory and the ignored PoB integration test:

First freeze Rust/data writers and compile without executing tests:

```powershell
cargo test -p poe-optimizer-pob --test owned_attribute_pipeline --no-run
```

After Cargo exits, take the exact executable path from that compilation's
`Executable` line. Invoke that binary directly; do not select a binary by wildcard
or modification time:

```powershell
$env:POE_OPTIMIZER_TEST_ATTRIBUTE_PIPELINE_SOURCE_OUT = 'runs/owned-attribute-pipeline-source-NN'
$attributeWitnessExe = '<exact executable path emitted above>'
& $attributeWitnessExe --ignored --exact actual_attribute_pipeline_preserves_stages_and_choice_control --nocapture
```

The parent test runs fresh JIT-off and JIT-on children, each with a 600-second bound
and fourteen complete loads. It retains seven raw cases per lane before asserting
the case comparisons, then requires the two complete report files to be identical.
Repository work serializes Cargo and source executables. Do not start Cargo while
this witness is running, and retain the observer/driver bytes used by its compiled
binary unchanged until it exits.
