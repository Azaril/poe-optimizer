# Configured Enemy ailment stack counts

This packet admits three raw Enemy Scenario assumptions from the saved
`ShockStacks`, `ScorchStacks` and `ChillStacks` controls. It builds on the Critical
Weakness package `1084b2615f8ec4b8d5960077b6dda429d5220dc2a27628f32b1121ec2a23cf05`.
External inputs `3391`, `3392` and `3393` respectively reuse Count unit `295a` and
an injected transport range of -1,000,000 through 1,000,000. This range bounds the
import grammar; it is not an asserted game limit.

## Source and ownership

All three original callbacks append their record to `enemyModList`. Each has
constructor Placeholder 1 and a separate UI default state of 0. Shock and Chill
use the existing nonzero-Input selection: Input 0 falls back to a saved
Placeholder or constructor 1. Scorch uses Input-then-Placeholder and preserves
Input 0. A raw candidate of zero remains a known input. It does not assert that
a source callback emitted a record: the count dispatcher skips zero for Shock
and Chill, while Scorch's countAllowZero callback may emit a zero-valued record.

The original callbacks attach an Effective condition to their records. UI
ifFlag/ifOption visibility is not a dispatch gate. Neither visibility nor PoB's
preview mode is imported as native gameplay state. Ailment activation, stack
limits, recipient delivery, strengths and actual effects remain separate work.
Negative, fractional and above-cap candidates remain unchanged within the
admitted transport range.

The source proof reuses the authenticated source-02 acquisitions and existing
complete ConfigOptions, ConfigTab and CalcPerform archives. It checks all three
catalogue entries, both constructors per setting and both actual callbacks per
setting. Every callback must append exactly one full Enemy record, including
source, tags and position, with all other Config fields and record inventories
unchanged. It performs no new PoB execution. Mutated-input CLI tests validate
native input transport, not new observed source executions or downstream parity.

The retained downstream body is an explicit boundary, not an accepted native
ailment recipe. It includes the apparent Chill breakdown inconsistency at
CalcPerform line 3258: that branch references ShockStacks/ShockStacksMax while
line 3256 computes CurrentChill from ChillStacks. No mechanics conclusion or
compatibility rule is adopted here. No direct ScorchStacks runtime reader was
identified in this bounded source review; that does not prove its effect absent.

## Publication and limits

The publication adds three definitions and six checked source-lane accounting
rules. Each Input and Placeholder is accounted against its own exact external
input. One Scenario link per saved leaf is preserved, including nonselected
projection responsibilities. Malformed or duplicate source frames remain
pending. A test-only explicit expectation partition covers every authored
output once and rejects overlapping source names, output IDs or leaf removals.

The shared CLI harness checks all five original XML builds, all 110 queries,
existing IDs, preferences, values and complete draft/sidecar inverses. Its prior
singleton and two-output distance behavior is separately regressed. The new
packet's verified Original05 change is 29 to 26 configuration-linked origins
(14 to 11 inside Config; 15 outside unchanged), with 24 to 27 known assumptions.
The four selected import obligations remain pending; no owner or query coverage,
native rule, ailment effect or full-build admission is added.

## Validation

The published package is
`runs/owned-configuration-ailment-stack-publication-01/package`:

- Input: `3ceae31848fd5c1b2c0c67532a440a8f95f0666b2333cdc0ac7dcb77717e8843`.
- Definitions: `ad2fc18e6bd61a446f28f655b4a544c563a7d5a8608759a93af15be3d5374fc5`.
- Compiled rules: `1ffc6341e470df1063a4fb3ba0c47d20d5421e7f6893ed2e93b69f2b9b598d3b`.
- Registry ends at `3393`; 18 files total 64,442,635 bytes.

Ordinary tests passed: four new tests, three existing distance source tests and
three existing Critical Weakness tests (`owned-configuration-ailment-stack-ordinary-02.log`).
Strict Clippy for all four targets passed in 17.41 seconds
(`owned-configuration-ailment-stack-clippy-01.log`). The all-five publication and
byte-identical rebuild passed in 104.54 seconds, including 60 per-source and
Encounter controls plus six duplicate-frame controls
(`owned-configuration-ailment-stack-publication-01.log`). All five selected issue
counts remain `[106, 117, 109, 123, 4]`; the complete-build count remains zero.

The first ordinary log is retained: it caught missing IDs in the newly authored
accounting rows before publication. Each row now has a distinct checked ID.
No original source evidence changed. The pinned singleton publication regression passed in 62.10 seconds
(`owned-configuration-ailment-stack-singleton-regression-01.log`), and the two-output
distance publication regression passed in 56.10 seconds
(`owned-configuration-ailment-stack-distance-regression-01.log`). Both reproduced
all 18 files of their prior published packages byte for byte, including their
original input and compiled identities.

The opt-in target is `owned_configuration_enemy_ailment_stack_inputs`; set
`POE_OPTIMIZER_TEST_AILMENT_STACK_PRIOR` to the authenticated predecessor package
and `POE_OPTIMIZER_TEST_AILMENT_STACK_OUTPUT` to a fresh output path.
