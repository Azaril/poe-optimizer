# Manual and allocated Djinn occurrence evidence

This is a source-correspondence checkpoint, not native numerical parity. The five
unchanged original requests still have **124 / 125 / 117 / 154 / 21** selected
issues and **0/5** complete native evaluations. Their package and all 110 queries
are unchanged. The next Djinn import change depends on a native input contract;
it must not merely replace Pending support targets with Known skill names.

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

The proposed [typed occurrence inputs](owned-skill-occurrence-input-proposal.md)
address raw input storage and producer authority. They do not by themselves
provide once-per-source supported-property aggregation, final input assembly,
action preferences, usage execution or preparation readiness. Provider ancestry
alone is not proof of physical or nonphysical source membership. Those consumers
must retain their own checked dependencies and numerical parity tests.

The immediate implementation gate is owner agreement on the occurrence-input
contract, followed by exact complete/draft binding and native consumption. Keep
all six selected support-target obligations until their full target paths and
input obligations can be represented. Re-finalize the unchanged originals after
that implementation, and choose the next blocker from the resulting reports.

Execution evidence is in `runs/owned-djinn-provider-source-01/`; unchanged package
and selected-request checks are in `runs/owned-djinn-provider-checkpoint-01/`.
The final Windows run passes in 51.12 seconds, with thirteen complete loads per
mode and equal JIT-off/on snapshots. Focused strict Clippy and formatting pass.
This new source target has not yet been verified on hosted Linux or in a complete
workspace runtime run. CI retains both source snapshots and logs on failure.
