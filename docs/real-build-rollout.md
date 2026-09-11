# Real-build rollout and breadth gates

The next integration priority is working real builds through one shared native preparation
and evaluation path. Parser rules, data catalogs and helper parity remain prerequisites;
their test counts are not the measure of breadth. This delivery plan uses the
[general build-model proposal](general-build-input-proposal.md), whose architecture direction
is now accepted, prioritizing structurally correct boundaries over narrow profile reuse. The [concrete API proposal](real-build-api-proposal.md)
uses all five originals to specify that boundary. The [implementation log](implementation.md)
tracks execution.

## Starting position

All five supplied originals import and can be inspected with caller-driven definitions.
Pinned PoB evaluates all five. Native whole-build evaluation admits **none of these five**:
lines 1/3/4 stop at the one-Skill profile guard, and lines 2/5 at one-SkillSet. The current
native preparation enum contains Spark and Mace only. Adding more parsed modifiers does
not remove that structural limitation or supply the missing calculations. The general
item-loading pipeline currently feeds inspection; native profile preparation still uses
the narrower equipment parser. Connecting those stages to shared native plans is required,
not merely completing more inspector-side parsing.

The existing inspection covers 15 saved skill sets, 200 groups, 541 gem occurrences,
116 authored items and 16 passive specs. These are development cases, not holdouts.
The G4 inventory-item frontier has 109 assembly, two parser, four rune and one affix stop
across 116 records. Explicit program permissions advance 26 records compared with the
same package with permissions withheld; these are first stops, not complete item assembly
or a complete dependency inventory. See the
[breadth inventory](breadth-validation.md) and
[observed mechanisms](breadth-mechanism-inventory.md) for source-backed details.

| Original | Existing reference selection | What it tests in the shared API | Role |
| --- | --- | --- | --- |
| Line 2, Gemling Legionnaire | Twister, skill set 6 | Ordered saved sets, Accuracy-dependent speed, buffs and granted actions | First player-action numerical integration target |
| Line 5, Disciple of Varashta | Skeletal Sniper minion Basic Attack | Skill set 4 independent of item set 2, summoner/minion ownership, offering modifiers, multiple actions and grants | First minion integration target, developed alongside line 2 |
| Line 3, Martial Artist | Whirling Assault, average-damage mode | Attack timing versus average hit, Living Lightning's owned actor, parent attributes, ES from body-armour Evasion | Continuous model test, next numerical target |
| Line 4, Gemling Legionnaire | Crossbow Shot; saved selection has no hit-damage output | Multiple active/supporting skills, item grants, enemy debuffs, distinct triggered and independent Tornado instances | Continuous model/trigger stress test; unavailable damage is an explicit expected observation |
| Line 1, Disciple of Varashta | Kelari and selected Sand Djinn action | Distinct manual/tree-granted instances, unresolved labels, allocation-provider jewel and disconnected ordinary notables | Continuous stress case; preserve unresolved identities and reference warnings |

Reference selections are evidence, not inferred player intent. None of these builds marks
a group for Full DPS. Preserve selected action, part, stat set and metric meaning; do not
invent a summed damage objective. Line 4's saved Crossbow selection has no hit-damage
output; line 3 uses average-damage mode, and both selected damage queries are unavailable
in the frozen reference report. These remain useful semantic coverage cases and cannot
serve as ready damage benchmarks. Do not change their selections
without an explicit request. Original bytes, inactive sets and supporting skills stay
intact. Reduced reproductions are additional tests, never replacements for these originals.
All five frozen observations use a Pinnacle encounter at enemy level 82. They do not
establish mapping coverage. R3 must add explicit, separately identified mapping scenario
variants with fresh reference evidence while retaining the originals' bossing observations.
Scenario configuration remains caller-supplied data, not a build-specific runtime default.
The selected Twister/player and Sniper/minion reports have finite hit-DPS observations.
The Sniper build also reports negative unreserved Spirit (-67): preserve that result and
classify feasibility separately. Matching PoB's calculation does not certify a legal or
feasible character, and a negative resource must not be clamped into an apparent success.

## Delivery sequence

| Gate | Work | Exit evidence |
| --- | --- | --- |
| R1: shared real-build contract | Map all five originals to source-bound instances and independently selected views. Preserve ordered entries, exact identity bindings and unresolved/deferred frontiers. Route existing Spark/Mace through the same outer boundary. | Executable original-source comparisons for selections, precedence/fallback and duplicate/reference cases; unchanged existing numerical goldens. Source controls establish that actor/action/provider relationships fit the contract; only implemented native producers yield effective nodes. General preparation reaches named dependencies instead of profile-count guards. Complete effective graph parity remains R2/R3. |
| R2: native preparation along real paths | Advance both first targets through effective configuration, equipment/passives, actors, skills/supports and modifier state. Add only the complete mechanisms encountered on those paths, reusing shared operations. | Paired stage snapshots against unchanged complete source methods, including ordering, ownership and partial failures. A per-build dependency report identifies completed stages and exact next blockers. No frozen PoB state is injected as production input. |
| R3: two complete native real-build evaluations | Evaluate the original Twister and Skeletal Sniper views through shared native plans, including all active dependencies affecting the requested outputs. | Fresh whole-document PoB/native comparisons for explicitly declared damage, resource and defence outputs in both bossing and separately identified mapping scenarios; exact actor/action/context binding and disclosed reference limitations. No dropped active effects, fixture-derived metrics, native subprocesses or per-build/per-skill profile dispatch. Partial metrics remain labelled partial. |
| R4: interactions and search realization | Change classes/ascendancies, supports/supporting skills, items, passive allocations and providers through the same model; verify required 1..N skills/items and separate point budgets. | Fresh full-build comparisons after controlled changes, including provider removal and a non-additive interaction. Materialize and reimport results. Serial and Rayon runs agree on outcomes and selected identities. Baseline-only agreement does not pass. |
| R5: remaining originals and new holdouts | Bring the other three original views through the same path, then expand mechanism families using independently sourced builds. | Per-build completion and explicit gaps, first-run holdout results retained, no new profile variant to accommodate an example, and regression of earlier targets after every new mechanism family. |

R1 must establish only the representation needed by these concrete cases; it must not
become an attempt to design every future mechanic upfront. Add minimal data and API seams
when a real input or an original source consumer demonstrates the need. Keep them versioned
and revise them when a contrasting case disproves an assumption. The shared actor/action
model must represent ownership from the outset: even the supplied weapon builds can
create other actors or triggers. R1 does not infer a completed effective graph from source
identity lookup. Its explicit deferred producer frontier is resolved during R2/R3.

R2 may require several dependency commits. At each integration checkpoint, rerun the paired
real paths and publish what moved. A helper is complete only for its own declared scope;
R3 cannot be checked off because a parser, catalog or item-prefix suite passes. Broader
standalone component work needs a named dependency of an active target or a demonstrated
cross-build regression. The already-started item-policy extraction is bounded supporting
work and does not take priority over the shared real-build path.

## Coverage report and anti-overfitting checks

Track each real input separately through source import, resolved view/ownership, native
preparation, numerical parity, mutation parity and parallel/search realization. Bind every
observation to exact input, definitions, backend, scenario and selected outputs. Record
Pass, Blocked or Not run, with the dependency and evidence behind each state. Never promote
an inspected identity or source-only snapshot to native calculation coverage.

Before native implementation of each numerical target, freeze a versioned expectation
manifest for its complete original view and each explicit scenario. Declare required
metric selectors, units, reference availability, comparison tolerances and feasibility
outcomes. All required entries must agree for that build/scenario to pass R3 or R5;
blocked, partial and not-run entries stay in the denominator. An unavailable reference
output passes only as an explicit matching availability outcome, not as a numeric zero.
Add newly required outputs by recording a manifest revision, never by deleting failures
or narrowing the target after seeing native results. Publish completed/total builds and
scenarios alongside the metric results.

The first [fixed diagnostic manifest](../tests/fixtures/breadth-expectations/README.md)
preserves all 110 existing public measurements, selected identities and contexts from
the five originals. Its artifact comparator keeps missing/failed entries in the denominator.
This is a starting reference contract, not R3 completion: mapping cases, general native
preparation, full resource semantics and fresh native comparisons remain explicit gaps.

During R1/R2, a blocker report should include every independently discoverable active
prerequisite and its dependency path, not just the first profile error. It must distinguish
unimplemented semantics from unresolved source identities and reference limitations. It
need not guess dependencies behind an unexecuted opaque callback. Unknown remains unknown.

Use the same public caller-input path as ordinary users. Fixture identities and expected
values belong in tests. Rename labels, reorder unrelated saved sets, select alternatives,
add duplicate instances and inject equivalent definitions to expose incidental coupling.
Do not add a third `NativeInput` profile for a new skill. Complete-build fixtures must
exercise the shared instance/resolution/plan seam and cross-catalog ownership checks.

At least one admitted real path must also run end to end with deliberately changed
numerical definition data and its new data identity. Verify that the corresponding
calculation changes as expected and that restoring the original definitions restores the
baseline. Equivalent-data injection alone does not prove the evaluator consumes the
injected values. Compare with an equivalent controlled reference change where supported;
otherwise identify the test as a native data-consumption check, not PoB parity.

R4 must compare reused worker state with fresh preparation for candidate A -> B -> A,
provider removal/re-addition and switches between explicit views. Repeat those sequences
in shuffled Rayon scheduling order. Shared stale state can make serial and parallel runs
agree incorrectly, so agreement between those two runs alone is insufficient. Verify
selected identities, metric availability and values after every transition.

Maintain a small breadth dashboard at the top of the implementation record. Its headline
is the number of complete real builds and interaction cases that pass, followed by blockers.
Unit/source comparison counts and microbenchmarks are secondary evidence. Performance is
measured on those admitted real paths: preparation, repeated evaluation, worker storage and
whole search separately. No speed claim based only on Spark/Mace establishes generality.

## Holdouts and missing families

The five development imports overlap and omit important families. Reserve new whole-build
holdouts before implementing their mechanics, prioritizing damage conversion, damage over
time/ailments, sustained triggers/resource constraints, distinct minion ownership, and
uniques or jewels that change allocation or skill behavior. Keep bossing and mapping
scenarios explicit. Class/ascendancy variants and alternate saved sets do not by themselves
supply independent archetype coverage.

Acquire builds through supplied exports or individual public share links, retaining version,
source and permission context. Do not scrape the PoE database sites. Capture the first
holdout result before adjusting code; a failing holdout becomes a documented regression
case rather than being removed from the denominator. Replace consumed holdouts as needed.
Request additional exports when the current originals reach useful native coverage, or
sooner if a concrete missing family is needed to resolve an API question.
