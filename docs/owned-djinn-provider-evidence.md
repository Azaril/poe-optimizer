# Manual and allocated Djinn occurrence evidence

This document records source correspondence, not native numerical parity. At the
**historical provider-source checkpoint**, the five unchanged original requests
had **124 / 125 / 117 / 154 / 21** selected issues and **0/5** complete native
evaluations; their package and all 110 queries were unchanged. These counts are
not current status: see [the living implementation document](implementation.md).
That checkpoint identified the need for an explicit native input contract before
Djinn import could advance; Known skill names alone cannot resolve support paths.

## Observed source relationships

The Rust integration witness `owned_djinn_provider_source` runs the unchanged
pinned PoB calculation lifecycle. It joins runtime records by their actual source
objects, including support effect membership, rather than matching display names.
Its source is revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

Original05 selects SkillSet4, Spec3, ItemSet2 and ConfigSet1. The original MAIN
selection remains Skeletal Sniper. The selected manual Sand and Water Djinn
sources have source ordinals 216 and 229. The allocated equivalents come from
nodes 13289 and 32705 and occupy separate generated groups.

| Relationship | Manual occurrence | Allocation-generated occurrence |
| --- | --- | --- |
| Raw source level | 20 | 1 |
| Prepared summon/command level in this build | 22 | 3 |
| Summoned actor level in this build | 44 | 6 |
| Physical support candidates | Three, from that exact manual group | None; generated group has `noSupports` |
| Source when the matching allocation is removed | Remains present | Disappears |
| Source when its saved generated group is removed | Remains independent | Reconstructed from the allocation |

Each occurrence supplies two player effects: its summon and a separate command.
Both effects share the exact source object and prepared inputs. The command has
no summoned actor of its own. The summon owns one actor with three Sand or five
Water child actions. Every child refers to that exact summon and its support
candidate list. Candidate origin and actual acceptance are separate observations.

For the unchanged manual occurrences, all three candidates are accepted on the
summon and its child actions. The command accepts Bidding II and rejects the other
two candidates. Disabling Bidding removes that exact candidate; it does not alter
the observed prepared level. The witness therefore **does not attribute the +2
level increase to supports**, or establish a general supported-property formula.

The controls separately remove each allocation and saved generated group, disable
each manual group, change each manual raw level to one, disable each Bidding
support, change only Sand's MAIN minion selector, and change archived manual
groups. The original plus these twelve controls are fresh complete loads. MAIN
and CALCS settings remain distinct. Archived changes preserve selected runtime
groups and outputs.

Removing a saved generated group can change the action occupying a numeric MAIN
index. The witness records that actual source behavior; it does not silently
rewrite another field to restore Sniper in the modified control. The unmodified
original retains its saved selection and all original XML files retain their
recorded hashes.

## Native design implications

The manual source cannot be inferred to be an Allocation provider merely because
its definition was originally a tree skill. Its independent inputs and supports
must survive the observed allocation-removal control. Conversely, generated
occurrences must retain their exact allocation identity and activation.

Existing declarations can express a nonphysical Direct root, a child Skill grant
for its command, and an Actor grant for its minion actions. That is a candidate
topology; declarations and source correspondence still need explicit authoring
and validation. Two unrelated Direct uses would lose the observed shared source.

The accepted [typed occurrence inputs](owned-skill-occurrence-input-proposal.md)
address raw input storage and producer authority. They do not by themselves
provide once-per-source supported-property aggregation, final input assembly,
action preferences, usage execution or preparation readiness. Provider ancestry
alone is not proof of physical or nonphysical source membership. Those consumers
must retain their own checked dependencies and numerical parity tests.

The owner accepted the occurrence-input contract on 2026-10-02. At that contract
checkpoint, complete/draft binding and native consumption passed component tests;
exact source-backed import and data integration were the next work. Keep
all six selected support-target obligations until their full target paths and
input obligations can be represented. Re-finalize the unchanged originals after
that implementation, and choose the next blocker from the resulting reports.

Execution evidence is in `runs/owned-djinn-provider-source-01/`; unchanged package
and selected-request checks are in `runs/owned-djinn-provider-checkpoint-01/`.
The historical Windows run passed in 51.12 seconds, with thirteen complete loads
per mode and equal JIT-off/on snapshots. Focused strict Clippy and formatting
passed; hosted Linux and a complete workspace runtime run had not yet verified
that source target. CI retains both source snapshots and logs on failure.

## Raw input boundary witness

The additional ignored Rust test
`complete_djinn_raw_inputs_preserve_source_boundaries` in
`crates/poe-optimizer-pob/tests/owned_djinn_raw_inputs.rs` passed both JIT modes in
76.45 seconds. Each mode attempts 21 complete source lifecycles: the original,
19 independent controls and a repeated original. Six controls intentionally fail
inside the source lifecycle; they are recorded as failures, not prepared builds.

The test reuses the original provider witness's function authentication and exact
source-object joins. An observational call hook reads the original `LoadSkill`
argument at its call to the original `ProcessSocketGroup`, before level
validation. It neither replaces a business function nor changes an input. The
hook is removed after the saved Djinn inventory, or during failure cleanup. The
report keeps XML attributes, loaded numeric fields and types, processed source
fields, and prepared summon/command values separately.

`SkillsTab.lua:352–353` uses `tonumber` independently for level and quality. The
observed finite controls establish the following distinctions:

| Saved input | Loaded raw value | Processed source value |
| --- | --- | --- |
| Level `0` or `-1` | Preserved | `1` |
| Level `41` | `41` | `40` |
| Level `40` | `40` | `40` |
| Level `1.5` or `20.5` | Preserved fraction | Natural maximum `20` |
| Quality `12.5`, `17.25`, `-1` or `101.5` | Preserved | Unchanged |
| Missing, malformed or textual `nil` level/quality | Lua `nil` | Complete lifecycle fails |

The pinned level table contains entries `1..40`. `CalcTools.validateGemLevel`
clamps to that range and falls back to `naturalMaxLevel` for an unresolved
in-range fractional value; it does not floor the raw value. Nil level fails at
`CalcTools.lua:64`; nil quality fails at `CalcTools.lua:163`. Quality has no
corresponding loader clamp, and the finite quality controls also survive unchanged
in the prepared summon and command. This is a finite correspondence witness,
not a proof that every finite input permits a complete calculation.

Raw values therefore need their own typed slots: Count quantities for level and
percentage-point quantities for quality. Missing or malformed values must retain
Pending import obligations. Source normalization and final input preparation are
separate consumers. In this build raw level `20` prepares to `22`, while a raw
`41` first becomes `40` and prepares to `40`; neither prepared result belongs in
the raw import data. No general quality or supported-property formula is inferred.

For each successful case, the summon and command share their exact source object
and prepared inputs. Other manual occurrences and allocation-generated sources
remain distinct; generated summon/command levels stay `3` with quality `0`.
Changing archived manual inputs preserves selected inputs and outputs. The
repeated original matches, saved selections remain intact, and all five original
fixture files retain their bytes.

From the repository root in PowerShell, reproduce with the optional pinned PoB
checkout available:

```powershell
Remove-Item Env:POE_DJINN_RAW_INPUTS_CHILD -ErrorAction SilentlyContinue
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_OPT_LEVEL = '2'
$env:CARGO_PROFILE_TEST_DEBUG_ASSERTIONS = 'true'
$env:CARGO_PROFILE_TEST_OVERFLOW_CHECKS = 'true'
$env:CARGO_INCREMENTAL = '0'
cargo test -p poe-optimizer-pob --test owned_djinn_raw_inputs --locked -- --ignored
```

The parent runs bounded children for JIT off and on, with a 300-second deadline
per child. `runs/owned-djinn-raw-inputs-01/source-jit-off.json` and
`source-jit-on.json` are byte-identical: **8,026,991 bytes** each, SHA-256
`00fbab0d71fbb8a3b0054399f28918d274ae2865645fdbfc82298d92de9bb6b9`.
The corresponding logs retain complete source errors; the JSON records stable
source error sites. The authored receipt in
`data/owned/poe2/3887ae68/direct-skill-inputs/authoring.json` commits both reports,
the original XML hash and pinned source manifest. This witness establishes no
native build parity or release-publication result; consult
[implementation status](implementation.md) for those checkpoints.

## Complete manual occurrence and action witness

The expanded Rust witness
`owned_sniper_actor_actions::djinn_families::complete_manual_djinn_occurrences_preserve_source_actions`
passes **81 cases in both LuaJIT modes**: 77 complete loads, each retaining fresh
and two requested rebuild observations, plus four exact source failures. Both
complete reports are byte-identical. All eleven manual sources across Original01
and Original05 are covered (six Sand, five Water), including separately activated
archived presets. The original five XML files and saved selections are unchanged.

The witness follows actual loaded source objects into the summon, separate
Command, actor and all eight child Skills. It checks the twelve constructed
Command/child stat sets, including Hidden labels, and keeps allocated siblings
separate. MAIN and CALCS actor/action selectors and nested maps are exercised
independently. Changing the group's main action to Command is distinct from
choosing a summoned actor's child action. Reference fallback and clamping are
recorded as source behavior; unknown native correspondence still remains Pending.

Controls cover independent raw level/quality on duplicate occurrences, archived
edits and activation, source/group enablement, count and global flags, group
full-DPS settings, corruption fields, absent/legacy selectors and malformed maps.
Disabling a source removes both supplied effects in the tested contexts; an
explicitly focused disabled group remains evaluated because of PoB's main-group
behavior. These are finite source observations, not universal activation rules.
The import packet still admits only its reviewed neutral corruption guards and
keeps usage semantics Pending.

The four failing controls retain their actual source errors: each family's
malformed map key fails in `SkillsTab.LoadSkill` at line 388; each unavailable
stat set fails in `CalcTools.buildSkillInstanceStats` at line 178. A failed load
never receives a successful lifecycle record. The temporary loader observer is
removed from those failures without replacing their original errors. Normal
source methods and historical physical observation paths remain unchanged.

Reports: `runs/owned-djinn-actor-action-source-01/source-jit-off.json` and
`source-jit-on.json`, each **45,100,464 bytes**, SHA-256
`03fad450dd3d61d502a5bb8a1632a4256c73711eeb94783f344fcda8ddebbb7d`.
The compact encoding preserves every observation within the unchanged 64 MiB
report limit. Fourteen source files are pinned, including CalcTools. The passing
run is `runs/owned-djinn-actions-source-04.log`; earlier attempts are diagnostics,
not publication authority. No canonical parity lifecycle is chosen.

This witness authorizes reviewed correspondence and intrinsic-field accounting,
not native numerical or support-admission completeness. In particular, the next
admission witness must capture initial skill/minion type sets and flags separately
from support-added final types. Its source `gemData` flag is not a claim of native
physical Gem ownership. The [living plan](implementation.md) records the package
publication and the unchanged complete-build gate separately.
