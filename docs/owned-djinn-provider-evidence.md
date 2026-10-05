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

The published Djinn declarations now express a nonphysical Direct root, a child
Skill grant for its command, and an Actor grant for its minion actions. The
source-action witness below establishes their correspondence. Two unrelated
Direct uses would lose the observed shared source. Topology does not establish
complete input, support or numerical coverage.

The accepted [typed occurrence inputs](owned-skill-occurrence-input-proposal.md)
address raw input storage and producer authority. They do not by themselves
provide once-per-source supported-property aggregation, final input assembly,
action preferences, usage execution or preparation readiness. Provider ancestry
alone is not proof of physical or nonphysical source membership. Those consumers
must retain their own checked dependencies and numerical parity tests.

The owner accepted the occurrence-input contract on 2026-10-02. At that contract
checkpoint, complete/draft binding and native consumption passed component tests;
exact source-backed import and data integration were the next work. Direct
raw-input import and topology are now published. The bounded target proof below
uses those same contracts; it does not require a completed calculation in order
to identify a support assignment's source root. The [living plan](implementation.md)
records checked publication and the current remaining selected issues.

### Exact manual support targets

Import's `PobManualSingleDirectRootV1` policy binds the digest of the already
authenticated Direct V2 policy. Successful admission must identify one manual
Direct root in its actual saved group, with no competing physical active root,
unknown child, generated sibling or ambiguous source. The existing physical
support census and order collector supply assignments to that exact authored
root. This is the same assignment target used by native preparation; child
Commands and minion Actions continue through declared grants and receiving paths.
No additional source matcher or native occurrence type is introduced.

Manual target authority requires the `source` attribute to be absent. An explicit
empty value follows PoB's unmatched generated-group cleanup and cannot certify
that target. The older raw-input adapter may retain the saved row with Pending
membership; the new target proof does not promote it. See the
[source distinction audit](legacy-retirement.md#bounded-generated-source-import-coverage-2026-10-05).

The policy resolves only target correspondence. Raw input values, enabled state,
loadout scope, usage, support-origin completeness, receiving and numerical owners
retain their independent obligations. Unknown raw numbers or a disabled use do
not erase an otherwise known target identity. Repeated manual sources stay
distinct, including when the corresponding passive allocation is removed.
The normalizer spends the old Pending issue ID before refining the target, so
all subsequent IDs and allocator state survive; only its exact issue link is
retired. The source-format sidecar is V20 when a target is attached. This is
separate from schema V6 and runtime operations V19.

The original provider, source-action and admission witnesses in this document
establish root/child correspondence. They do not close the whole support plan.
Integrating complete input producers and contributors remains a separate gate.

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
admission witness below captures initial skill/minion type sets and flags separately
from support-added final types. Its source `gemData` flag is not a claim of native
physical Gem ownership. The [living plan](implementation.md) records the package
publication and the unchanged complete-build gate separately.

## Generated saved settings and quality

The additional test
`generated_skill_usage::complete_generated_skill_usage_preserves_source_consumers`
in `owned_authored_skill_membership_source.rs` passes forty-six cases in both JIT
modes: all fifteen original/archived selections, thirty independent generated-group
controls and a repeated Original05. Each case retains fresh load and two requested
rebuilds. It reuses the exact ownership observer; the existing manual Djinn controls
remain independent evidence rather than being duplicated.

The new reports at `runs/owned-generated-skill-usage-source-01/source-jit-{off,on}.json`
are byte-identical, 64,295,778 bytes each, SHA256
`75ca2ba05fdecc465d4727f43b2a31358ffcd2fb134a80281534bc4d24a867d5`.
The final parent passed in 193.73s with exact prepared-quality/effect-set
assertions. Actual source functions, object joins and outputs
are checked for preservation. Reporting rows without occurrence identity are
explicitly unjoined; display names do not establish attribution.

For generated Sand, Water and item Firebolt, saved count3 reaches the original
count helper, and present group0/group4 overrides it for the exact supplied
effects. Reconstruction forces global1 true even when the saved value is false.
Disabling the source or group removes the corresponding non-main active effects
in the observed contexts. These are setting/reconstruction facts, not an alternate
native activation rule or a proof of complete usage.

A saved quality12.5 survives reconstruction and reaches prepared quality12.5 for
all three generated primaries, including both Djinn Commands. A new generated
entry may start at zero, but that does not authorize a universal provider quality
zero. Raw quality needs an explicit occurrence input producer with the accepted
shared typed-slot authority; it must not be inferred from item/tree identity or
misrepresented as already prepared data.

The generated Djinn summons have HasReservation but not MultipleReservation;
their Commands and Firebolt have neither in this observed frame. Count-helper
values alone therefore do not justify reusing the skeletal reservation multiplier.
The report retains actual per-action reservation fields and unjoined FullDPS rows.
Source-attributed aggregation, native numerical consumers and complete contributor
coverage remain separate gates. No canonical parity lifecycle is chosen here.

## Original Djinn support preparation and admission

The optional Rust test `complete_djinn_support_preparation_observes_original_admission`
in `crates/poe-optimizer-pob/tests/owned_djinn_support_preparation_source.rs`
observes the original `createActiveSkill` and
`canGrantedEffectSupportActiveSkill` through bounded read-only call hooks.
Neither business function is replaced or reimplemented. Twenty pinned files
and independent XML occurrence checks authenticate the source and input joins.
The hook is removed before inspecting the resulting graph, and the existing
complete-source lifecycle witness verifies preserved methods, selections and outputs.

The twelve cases in each JIT mode cover all five originals, six independently
disabled support occurrences in selected Original05, and a repeated Original05.
Each retains fresh load and two requested rebuilds: 24 loads and 72 snapshots
across both modes. Each mode retains 1,296 exact contexts. The expanded witness
passes in 189.60 seconds (`runs/owned-djinn-support-preparation-source-03.log`).
Both reports at `runs/owned-djinn-support-preparation-source-02/source-jit-{off,on}.json`
are byte-identical: **16,410,774 bytes**, SHA256
`2b43bd0ae0c52ac2cfa63978735f28df75a07153c62714cd40ef6e873d1a742d`.

Initial definition types, mutable types at each original predicate call,
parent prepared types, constructor-return types and later final types are
recorded separately. Child actions have no socket group or source instance of
their own; exact parent and shared-support object identity establishes their
origin. Their admission predicate uses the summoner's prepared types. Commands
have no such summoner relationship. The source's Gem-data flag on a manual
Djinn does not turn its native Direct occurrence into a physical Gem.

In the selected Original05 contexts, each manual summon and all eight child
actions accept their three local supports. Both Commands accept only Bidding II;
allocated Djinn siblings have no candidate supports. Water Frost Nexus adds
CreatesGroundEffect during preparation, although it is absent from the initial
summon definition. The six disable controls compare all remaining exact
candidates and accepted source ordinals against the original minus only the
disabled occurrence, and check removal of the Frost Nexus type. This prevents
a control from passing merely because every support disappeared.

The initial four-definition witness covered Bidding II, Magnified Area I, Muster
and Frost Nexus. Original01 contains six additional candidates: Bidding III,
Magnified Area II, Prolonged Duration II, Hulking Minions, Kurgal's Leash and Rapid
Casting II. The expanded witness authenticates all ten definitions and proves
each appears in actual predicate calls. Hulking Minions uses a conjunction;
Rapid Casting II has exclusions. The first four definitions remain exactly
unchanged. Prior four-definition reports/logs are retained under
`runs/owned-djinn-support-preparation-source-01/` and source logs `-01`/`-02`;
they are historical evidence, not the current ten-definition authority.

## Reviewed preparation data and native component parity

The [checked-in packet](../data/owned/poe2/3887ae68/djinn-support-preparation/README.md)
uses the existing `SupportPreparationInput` contract: ten supports, 34 concrete
type symbols, ten effect symbols and eight families. Source operator tokens do
not become concrete types. Raw declarations and constructed definitions drive
authoring; runtime predicates contain only owned types and identifiers. Original
admission results are independent expectations in the full authenticated reports.

The canonical standalone preparation identity is
`f3afe031c10d9e7a6927a4b06db3665e9e710f92b06f35a5bde084fb5e39af5b`, bound to
release `179b0cd3decbfc79bd0c52ae087597ae5e7a3fc008b35c2638519c5236cf2191`.
It changes no release file or registry ID and supplies no complete evaluation
bundle. Ordinary CI authenticates checked-in facts without local reports. The
optional full check also authenticates source bytes, original XMLs and the exact
release before constructing the immutable native artifact.

All four tests in `tests/owned_djinn_support_admission_native.rs` pass in 2.32s.
They replay every one of the 1,296 contexts without filtering Original01. Exact
saved Gem/variant mappings and physical support roles authenticate candidate
correspondence; existing Action/tree declarations authenticate receiving paths.
Initial own types are inputs, native prepared parent types feed children, and
source acceptance/constructor-final types are expectations. The fixture's local
IDs are explicitly test-owned; this does not claim a complete project binder.

Controls cover exact source disabling, duplicate level/quality requirements,
unknown activation/type facts, a real unreviewed support, bounded failures and
Rayon reuse of the immutable artifact and selection. Selection/admission is
call-local; this does not claim reusable worker scratch for that component.
The passing log is `runs/owned-djinn-support-admission-native-01.log`.

Both data tests also pass (`runs/owned-djinn-support-preparation-data-02.log`,
10.33s): canonical codec roundtrip, stale rules rejection and fresh normalization
of the unchanged five originals. All eighteen release files and 110 queries are
unchanged; unresolved counts remain 113/116/108/121/11 and complete builds 0/5.
Numerical support delivery, complete contributors, source-property assembly and
actual support-plan readiness remain separate gates. No selected target issue
is retired and no canonical reference lifecycle is chosen.
