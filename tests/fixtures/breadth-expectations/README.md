# Diagnostic expectations for the supplied originals

`originals-v1.json` freezes the existing five saved Pinnacle/level-82 observations from
`runs/number-factories-corpus`. It executes no evaluator and introduces no production build
model or profile. All cases retain the original XML bytes, inactive content and saved
selection. **Whole-build parity is not established.** These are development cases, not holdouts.

The manifest records every existing public measurement: 22 per build, 110 total (93 finite,
one positive infinity and 16 unavailable). Coverage includes selected player/minion hit DPS
and average hit, maximum Life/Mana/ES/Spirit, capped resistances, Armour/Evasion ratings,
PoB aggregate EHP, five maximum-hit types, and movement/action speed. It does not define
population totals, sustained rotations, Full DPS, or new per-hand/average-mode semantics.

Each case binds the input and reference report hashes, existing backend/runtime identity,
complete recorded context and selected actor/action records. Context keeps config inputs,
placeholders and conditions separately; null requested options mean preserve the saved
request. Authored selectors retain raw strings, including literal `"nil"` weapon-state
values. Passive positions, set IDs and runtime actor indices remain distinct evidence;
they are not a proposed stable production selector API. Paths resolve relative to this file.

The current source pin is `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. The schema25 data
digest under `source.native_baseline_data_sha256` belongs to the historical native attempt,
not to PoB's reference calculation. A newer native package must not relabel that evidence.
The reference was the production CLI running original PoB calculations; this is not a
new independent-host calibration or certification of game mechanics.

Availability is an expected result. Crossbow's saved selection produces no hit-damage
measurement; Whirling Assault uses average mode and has no exposed average-hit value in
this adapter. Attack averages may reside in per-hand output. Kelari's chaos maximum hit
is positive infinity. Preserve those states instead of replacing them with zeros or a
different selection. Unavailable reason text is diagnostic, not a portable equality key.

The optional finite-tolerance comparison uses
`abs(actual - expected) <= max(1e-8, abs(expected) * 1e-9)`, the
existing [independent-host calibration policy](../../../docs/calibration-reference.md).
This is a declared future comparison policy, not evidence that these builds pass it.
Existing stricter native golden checks remain unchanged. Stored artifact hashes, bindings,
units, statuses and nonfinite kinds are exact; the generation audit also preserves finite
number bits. The [generic checker](../../../scripts/check-build-expectations.py) compares
numbers exactly by default; `--finite-tolerance` opts into the declared tolerance. Its
success means the required recorded observations match, not native completion or equality
of every raw snapshot field. It exposes backend/report identity differences separately;
source-only resource observations are retained outside its public metric comparison.

`source_only_resources` separately preserves raw player `LifeUnreserved`, `ManaUnreserved`
and `SpiritUnreserved` observations. These are outside the public metric check and do not
establish full feasibility. The Sniper's unreserved Spirit remains -67. No legality result
is inferred from a finite output or from these three observations.

Before R3/R5 completion, add separately identified mapping cases, implement general native
preparation, run fresh full-document reference/native comparisons, and establish mutation
and reused-worker parity. Full effective dependency graphs, population/uptime semantics
and complete resource/requirement legality are not supplied by this manifest. Required
coverage can grow through explicit manifest revisions; failed rows must not be deleted
to manufacture a pass. See the [rollout](../../../docs/real-build-rollout.md) and
[API proposal](../../../docs/real-build-api-proposal.md), whose architecture direction remains pending.

The one-time generation and source-binding audit is recorded locally in
`runs/breadth-expectations-generation.json`. It checks each public row against the frozen
raw attachment and existing source mapping without calling PoB, changing original fixtures,
or treating reference output as production input.
