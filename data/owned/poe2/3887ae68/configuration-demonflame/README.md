# Configured Player Demonflame count

This packet admits the raw `demonFormStacks` count as Player Scenario input
`3398`, using existing Count unit `295a`. Its predecessor is the Player activity
package `2582adba98346ce8c6e7fd5b25c7bf97bf61269fd3eebba7764926d7ac694cae`.
The injected range -1,000,000 through 1,000,000 bounds the transport grammar;
it is not a game limit. Negative, fractional and large raw values remain
unchanged within that transport domain.

## Ownership and source contract

Demonflame is the Player's transformation-state counter. The pinned Demon Form
description says it accrues while the Player remains transformed. Its original
buff consumers use the same counter for Spell damage and Life degeneration.
The actor owns this counter; the supplying skill still owns its buff values,
activation and delivery. Importing the count does not enable a missing skill.

ConfigOptions lines 355–361 define a separate Form activation control and the
count callback. The activation control has a Life threshold of 2. The count
callback writes one Player multiplier with a DemonForm condition. The exact
source-02 catalogue row 95 has type `count`, Placeholder 10 and UI default state
0. Input is absent in both retained constructions. Each recorded callback takes
10 and appends exactly one full Player record; all other Config state and
modifier inventories must remain unchanged.

The existing nonzero-Input projection preserves the original selection:
nonzero Input wins, Input zero falls through to Placeholder, and absent
Placeholder uses constructor 10. Saved Placeholder zero is a known raw
candidate. The original dispatcher skips that callback, so it does not imply
an emitted zero multiplier. The `ifSkill` field controls visibility and is not
a dispatch or native activation gate.

Source proof reuses the existing authenticated ConfigOptions, ConfigTab and
source-02 reports. The added compressed `other.lua` is the complete normalized
manifest-pinned skill catalogue, retained for the Player-state description and
buff consumer boundaries. Its 1,172,527 raw bytes require a narrowly scoped
2 MiB offline archive allowance for this exact source path; other paths retain
the existing 1 MiB allowance. Native package and evaluator limits are unchanged.
The upstream license is the existing configuration-accounting evidence license.
No new PoB execution occurs.

## Publication scope

One definition and two exact source-lane accounting rows are added. Input and
Placeholder receive checked receipts, including the nonselected lane when
appropriate. Malformed or duplicate source fields remain pending. The complete
all-five draft/sidecar inverse must preserve prior values, IDs, usage preferences,
110 saved queries and every selected import obligation.

The verified Original05 change is 21 to 20 configuration-linked origins, with
7 to 6 inside Config and 14 outside unchanged; its known assumptions increase
from 31 to 32. Original05 source ordinal 456 is the saved Placeholder 10.
No native rule, Form activation, stack cap, damage, Life loss, duration, supplier
closure or complete build is admitted by this input transport.

## Validation

The published package is
`runs/owned-configuration-demonflame-publication-01/package`:

- Input: `3b3e135ec63ce4abb92105efe56ad1bb7cb802759f8473070a11811f2d1a6e7d`.
- Definitions: `9555a59e5ef17379c86f9e0a87654b607b985edb338fbe69ca36907f5f6c0f3a`.
- Compiled rules: `b0d428cbdbe8441b709456d908024d30a15c27881b5d7bdc34da62d2ec23fcd5`.
- Registry ends at `3398`; 18 files total 64,455,304 bytes.

Four new ordinary tests passed in 2.02 seconds; the four existing Player
activity tests passed in 2.22 seconds as a shared source-authentication
regression (`runs/owned-configuration-demonflame-ordinary-01.log`). Strict Clippy
for both targets passed in 1.09 seconds
(`runs/owned-configuration-demonflame-clippy-01.log`). The all-five publication
and byte-identical rebuild passed in 62.83 seconds, including 20 source/Encounter
controls and two duplicate-frame controls
(`runs/owned-configuration-demonflame-publication-01.log`). All five selected
issue counts remain `[106, 117, 109, 123, 4]`; no complete build is admitted.

The target is `owned_configuration_demonflame_input`; the opt-in publication
uses `POE_OPTIMIZER_TEST_DEMONFLAME_PRIOR` and
`POE_OPTIMIZER_TEST_DEMONFLAME_OUTPUT`.

The controls distinguish source identity, exact callback/Player ownership,
archive bounds, raw zero/fractional/above-cap inputs, malformed winning and
losing lanes, fresh Encounter guards and duplicate frames. Native input
mutations are not additional original-source executions or downstream parity.
