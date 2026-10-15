# Configured critical-weakness count on the Enemy

This packet adds external input `3390`, using the existing Count unit `295a`, to
the selected scenario's Enemy assumptions. It preserves the configured count
candidate before activation and transformation. It adds no native contribution,
critical-chance formula, usage policy or coverage claim.

The predecessor is reporting-inclusion publication input
`e1c2abd299aead48ff1da38aa517c4fcbcaacd2671f754deb921364c0cad4b39`.
The one new definition extends the existing Encounter's input inventory without
completing it. Existing rules, receivers, query membership and input obligations
remain unchanged.

## Source authority and limits

The source evidence reuses authenticated, portable original PoB files and source02
observations. No new PoB process or source mutation was run for this packet.
The original `ConfigOptions.lua` lines 1908–1913 separate the enabling condition
from the count callback. The count callback receives the selected number, clamps
it to 0–20 and divides by two, producing an Enemy `SelfCritChance` record tagged
`ApplyCriticalWeakness`. `CalcOffence.lua` lines 3876–3885 show the subsequent
critical-chance read. None of those transformations or effects is implemented by
this input packet.

Both retained constructor phases have no authored Input, a UI default state of
zero, and effective Placeholder 20. Both retained callback observations receive
20 and append exactly one Enemy record with value 10. The proof compares the
complete before/after Config state, preserving record order, tags and recipients.

The existing `nonzero_input_then_placeholder` selector preserves these roles:

| Saved values | Imported candidate |
| --- | ---: |
| Input 30, Placeholder 20 | 30 |
| Input 0, Placeholder 80 | 80 |
| Input 0, no Placeholder | 20 from the constructor |
| No Input, Placeholder 0 | 0 |
| Neither saved lane | 20 from the constructor |

A zero candidate is known input data. The original `count` dispatch skips its
callback when the selected candidate is zero; this packet does not turn that
case into an observed emitted zero modifier. UI `ifOption` visibility is not an
import applicability condition. No PoB UI or calculation-mode flag enters the
native model.

Negative, fractional and above-20 candidates remain unchanged within the
explicit transport range ±1,000,000. This is an admitted import domain, not a
claim about legal gameplay counts. Activation, clamping, critical-effect delivery,
downstream numerical parity and whole-build completion remain separate work.

## Verification

The ordinary Rust target checks exact authored schema, both saved-lane receipts,
original callback identity and constructor/delivery observations. Mutation cases
reject altered pins, target, source span, callback count, argument, recipient,
record value and condition tags.

The ignored publication test uses the production CLI to publish and reproduce
all 18 package files, then imports all five unchanged builds. Its complete
draft/sidecar inverse permits only the one new Enemy assumption and the checked
source-leaf disposition. All local IDs, prior preferences and values, 110 query
bytes and selected issue counts must remain exact. Original05's saved Placeholder
ordinal 448 moves from `01f2` to the exact Scenario link; the unresolved origin
census becomes 29 (14 inside Config and 15 outside). All four selected input
obligations remain Pending.

Seventeen explicit fallback/range/malformed-lane controls, three existing
Encounter guard controls and two duplicate-frame controls exercise normalization.
Every saved lane in a failed or duplicate frame must retain its issue link and
must not acquire a Scenario link. No baseline scalar zero is treated as proof
that a supplier or effect is absent.

Reproduction uses `tests/owned_configuration_enemy_critical_weakness_input.rs`;
set `POE_OPTIMIZER_TEST_CRITICAL_WEAKNESS_PRIOR` to the authenticated predecessor
package and `POE_OPTIMIZER_TEST_CRITICAL_WEAKNESS_OUTPUT` to a fresh output
directory for its ignored publication test. The project Cargo profile and serial
execution conventions apply.

The verified publication is `runs/owned-configuration-critical-weakness-publication-02/package`,
with input identity
`1084b2615f8ec4b8d5960077b6dda429d5220dc2a27628f32b1121ec2a23cf05`
and compiled identity
`0bf62d5976442cd682f66d90dae992d1786d66a22d0c8e0c9c707c6179700c81`.
It contains 18 files (64,435,356 bytes), 188 provenance rows and 110 queries.
All-five selected issue counts remain `[106,117,109,123,4]`.

Validation logs are `runs/owned-configuration-critical-weakness-ordinary-03.log`
(three tests, 2.02 s), `runs/owned-configuration-critical-weakness-publication-02.log`
(all-five publication and 22 controls, 61.74 s), and
`runs/owned-configuration-critical-weakness-clippy-01.log` (strict target Clippy,
40.95 s). The retained ordinary-01 failure exposed that an expanded review had
replaced a historically referenced evidence file. The original `d8f7abf…` review
was restored byte-for-byte; its quest-reconciled successor remains separately
retained. This input requires only the original review. Diagnostic publication01
bound the expanded review, while final publication02 uses the original reference.
Their 17 artifact files are byte-identical; only release provenance differs. No
original acquisition bytes, native arithmetic or input behavior changed.
