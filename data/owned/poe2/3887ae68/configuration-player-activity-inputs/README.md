# Configured Player activity counts

This packet admits four raw Player Scenario assumptions from saved activity
counts. It builds on the empty Party publication
`34ba87395f5fa7fdaa2c89fd9eb653b2e064f63e6b693b29b303e9c15b7bae2d`.

| Saved control | External input | Original05 source ordinal |
| --- | --- | --- |
| `multiplierWarcryUsedRecently` | `3394` | 439 |
| `multiplierStunnedRecently` | `3395` | 440 |
| `multiplierDifferentAmmoFired` | `3396` | 445 |
| `multiplierDifferentGrenadeFired` | `3397` | 449 |

All four use the existing Count unit `295a`, Player ownership and an injected
transport range of -1,000,000 through 1,000,000. The range bounds the input
grammar; it is not a game limit. Negative, fractional and above-cap values
remain unchanged.

## Source and selection

The authenticated original ConfigOptions callbacks append to the Player
modifier list. Each control has type `count`, constructor Placeholder 1 and a
separate UI default state of 0. A nonzero Input takes precedence. Input 0 falls
through to a saved Placeholder, or constructor 1 if no Placeholder is saved.
A saved Placeholder 0 remains a known raw candidate. The original dispatcher
skips its callback, so candidate 0 does not assert an emitted zero modifier.

The source proof reuses the existing source-02 reports and complete, pinned
ConfigOptions, ConfigTab, ModParser and ModStore archives. It checks all four
catalogue entries, both constructors and both actual callbacks per control in
each retained report. Every observed callback must append exactly one full
Player record, including ordered tags, source and position, while preserving
every other Config field and record. No new PoB execution is performed.

The retained bodies also document the unimplemented boundaries:

- Warcry and Stunned callbacks clamp at 100. Their records require Combat and
  respectively UsedWarcryRecently or StunnedRecently. These conditions have
  separate source controls.
- Ammunition and grenade callbacks forward the raw value with an Effective
  condition. Generic modifier recipes can cap their multipliers using AmmoTypes
  or GrenadeTypes; the UI labels describe activity in the past ten seconds.
- Generic multiplier consumption can read parent stores, override values,
  actor-specific limits and nested modifiers. This input packet adopts none of
  those effect or recipient semantics.

UI `ifOption` and `ifMult` visibility are not callback dispatch gates. They do
not become native activation flags. The source-body excerpts mark these future
responsibilities; they are not a claim of native downstream parity.

## Publication contract

The publication adds four external inputs and eight checked source-lane rules.
Each saved Input and Placeholder must receive its own exact output receipt,
including a checked nonselected role when appropriate. Malformed values,
duplicate fields and unproved fresh-frame selection remain pending. The
existing multi-source harness requires a complete, ordered and nonoverlapping
partition of the four inputs; it preserves every unrelated output during each
source-specific control.

The all-five CLI publication checks all 110 saved queries, prior IDs, values,
preferences and full draft/sidecar inverses. The verified Original05 change is
25 to 21 configuration-linked origins (11 to 7 inside Config; 14 outside remain
unchanged), with 27 to 31 known assumptions. All four selected import
obligations remain pending. No native program, owner or query closure, activity
effect, resource obligation or full-build admission is added.

## Validation

The published package is
`runs/owned-configuration-player-activity-publication-01/package`:

- Input: `2582adba98346ce8c6e7fd5b25c7bf97bf61269fd3eebba7764926d7ac694cae`.
- Definitions: `3924e86b0b459b7f26a67c13b3749503ecd1004136182819e19ad1acb46849d7`.
- Compiled rules: `517f3302f1fd132cb6f1d6d208695e85ea7038efb6fc6dac7dbe8d0447646c65`.
- Registry ends at `3397`; 18 files total 64,452,721 bytes.

Four ordinary tests passed in 2.31 seconds
(`runs/owned-configuration-player-activity-ordinary-02.log`). Strict target
Clippy passed in 1.14 seconds
(`runs/owned-configuration-player-activity-clippy-01.log`). The all-five
publication and byte-identical rebuild passed in 125.33 seconds
(`runs/owned-configuration-player-activity-publication-01.log`), including
80 source/Encounter controls and eight duplicate-frame controls. All selected
issue counts remain `[106, 117, 109, 123, 4]`; zero complete builds are admitted.
The retained first ordinary log records a test-helper macro syntax error,
corrected before these checks without changing data or production code.

The opt-in target is `owned_configuration_player_activity_inputs`. Set
`POE_OPTIMIZER_TEST_PLAYER_ACTIVITY_PRIOR` to the authenticated Party predecessor
and `POE_OPTIMIZER_TEST_PLAYER_ACTIVITY_OUTPUT` to a fresh output path.

The publication controls cover each selector's absent/zero/fractional and
above-cap paths, malformed winning and losing lanes, exact Player ownership,
fresh Encounter guards and duplicate frames. These are native import controls;
they are not additional original-source observations.
