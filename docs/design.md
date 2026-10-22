# Build optimizer product design

Accepted direction, consolidated 2026-10-22. This document owns product
requirements. The [domain architecture](domain-architecture.md) owns implementation
boundaries and the [living plan](implementation.md) owns delivery/status. The
[documentation index](README.md) identifies historical and supporting material.

## Problem and boundaries

Build construction is a large coupled search over character choices, passives,
equipment, skills and supports. Exhaustive search is usually impractical, and
small local changes miss important interactions. The product finds and explains
good legal alternatives under user-defined objectives, constraints and budgets;
it does not promise a global optimum or that a simulation predicts actual clear
time without assumptions.

Target Path of Exile 2 first, with PoE1 possible through separate injected data and
rule profiles later. Core libraries and native evaluation are Rust. CLI comes
first, with structured visualizable output and a later desktop/browser interface.
PoB is an optional acquisition/parity reference, not a required native runtime.
Game mechanics and build inputs are data-driven, never selected by fixture names.

The first complete unchanged build must run on the new DSL/data/runtime path.
Remove superseded paths at migration cutover. A one-axis native optimization demo
follows that first build; it is an engineering milestone, not the full product.

## Product workflow

1. Import a build/share code or author a project using the same model. Preserve
   originals, independent presets and explicit source diagnostics.
2. Select a seed, allowed candidate pools, changeable dimensions and exact locks.
3. Select scenarios and inspect assumptions, metric availability and legality gaps.
4. Configure objectives, hard constraints, soft preferences and run budgets.
5. Run, inspect progress, cancel/checkpoint, and compare best-found alternatives.
6. Independently verify finalists, explain changes and trade-offs, then save/export
   supported builds with faithful reimport behavior.

Unsupported imports/metrics need actionable diagnostics. A supported external
build must not require user edits or simplification to work. Preserve known input
facts even when other mechanics remain unresolved. Never silently repair the
seed or treat a cached PoB output as an expected current calculation.

## Joint domain and required locks

The first usable optimizer searches **class/ascendancy, passives, equipment,
support gems and supporting active skills together**. The seed class/ascendancy
is not an implicit lock. Users can require 1..N skills and retain 1..N exact
items, with independent locks for other decisions. Support and skill interaction
is central, not an optional later extension.

Finite pools bound a run without eliminating required search dimensions. Explicit
configuration distinguishes owned inventory, supplied alternatives and later
acquisition candidates. Unknown item prices/availability are not assumed free.
Keep physical inventory copies, rolled descriptors, socket configurations and
equipped uses distinct. Enforce stock and mutually exclusive assignments.

Tree operators respect class roots, ordinary and ascendancy point pools,
connectivity, weapon states and exceptional access. Separate ascendancy trees
are not invalid merely because they are disconnected from the ordinary tree.
Every mutation/repair checks the selected game's injected legality rules and all
locks. Removing a source removes its effects/grants and invalidates exact dependent
selectors. Import-time proof is not post-mutation legality.

Supports, levels, quality, alternate skill configurations, participation and
resource demands belong to the full candidate state. Coordinated changes must
handle support eligibility and generated actors/actions without item/spell-specific
logic. Candidate effects are recalculated; external encounter assumptions stay
fixed for a comparison.

## Objective and constraint contract

For a scalar objective, maximize `f(E(build, scenario))` over legal states subject
to the user's constraints (negate for minimization). Damage, resistance, EHP,
movement, resource cost and price are examples, not an exhaustive or implicit set.
A metric may be a goal, constraint, both or neither.

| Assessment | Meaning |
| --- | --- |
| Invalid | Violates known game rules or locks |
| Unsupported / unavailable / error | Required meaning, metric or legality coverage is unresolved or evaluation failed; not a feasible recommendation |
| Infeasible | Legal and evaluated, but at least one user hard constraint fails |
| Feasible | Supported, legally validated and every hard constraint passes |

Preserve same-attempt requirement assessment beside metrics. Available diagnostic
numbers do not imply a complete valid build. Missing or non-finite values must
not become zero or silently pass constraints.

Support single scores, explicitly normalized weighted composites, ordered
priorities and Pareto alternatives as declared capabilities. Soft preferences do
not override hard feasibility. Pareto ranking needs an archive/diversity policy,
not a hidden weighted sum. Reject unavailable modes rather than silently reducing
them to DPS. Metric-bound objective expressions are typed, unit-checked and
bounded; arbitrary Lua or general scripting is unnecessary.

Support strict/non-strict comparisons and explicit ranges; reject contradictory
bounds, unknown metrics and invalid units. Use unrounded values for feasibility.
Tie-breaking, tolerance, normalization and deterministic ordering are explicit.
Raw evaluation caches are independent of ranking: a threshold change can reuse
compatible measurements without treating an old feasibility result as current.

The [illustrative objective configuration](../examples/objective.toml) is a design
example, not a claim that all policies/providers are already implemented.

## Metric and scenario semantics

Queries identify exact actor/action/part, scenario, units, aggregation and output
meaning. Final stats, selected-action damage, full damage reporting, skill count
and actor population are different concepts. Definitions declare availability and
required assumptions. Incomplete effect reach stays blocking unless a separately
approved scoped-coverage proof establishes independence.

Benchmark bossing and mapping from the start. Multi-scenario objectives specify
weights, priorities or worst-case aggregation rather than silently averaging.
Encounter/enemy/environment assumptions remain external; buffs, resources, charges
and conditions caused by the candidate are recomputed. State uptime and damage
proxy assumptions, including their limits. Avoid comparing different scenarios or
measurement definitions under the same metric label.

## Search strategy

Start with bounded deterministic candidate generation and best-found archives,
then combine beam/local search, restarts and diverse starts/islands as measurement
justifies. Search the coupled problem with larger coordinated operators, not only
alternating single-axis hill climbing. Allow clearly labeled infeasible exploration
when useful, while recommending only verified feasible candidates.

Canonicalize/deduplicate complete candidates, validate locks and cheap legality,
prepare/evaluate through the native library, assess objectives and update the
archive. Keep caches bounded and content-addressed; test cold/warm equivalence.
Track promising alternatives and diversity. Never store a failed evaluation as a
valid zero. Independent finalist reconstruction/re-evaluation must not merely read
back the search cache.

Use direct Rayon evaluation with immutable shared packages/plans and worker-private
scratch. Bound jobs, memory, queue length and evaluator work across starts and
runs. Integrate results by proposal sequence for reproducible count-budget runs.
Wall-clock runs record their different stopping semantics. Cancellation and partial
results do not require killing a per-candidate process. Optional PoB validation is
outside the native search loop.

The quality target is configurable **5–30 minute runs**. Compare random, greedy and
alternating-domain baselines under equal budgets and multiple seeds. Report
quality, evaluations, failure rates, memory and throughput separately. Measure
fixed-candidate scaling and end-to-end search; more threads alone is not a quality
claim. User budgets also bound CPU/memory and optional expensive features.

## Optional seeded passive transformations

Investigate seed-dependent jewels/runes as an opt-in search dimension, including
seed/socket/path interactions and useful trade opportunities. First establish
actual PoE2/PoE1 version applicability, compatible seed data and cost. Do not assume
similarly named mechanics share algorithms. Preserve exact radius, overlap,
replacement/removal and legality behavior. The
[seeded-jewel investigation](seeded-jewel-feasibility.md) defines the evidence,
provider, bounded search, reporting and independent quality/cost stages. It does
not displace the first build or joint optimizer.

## Results and validation

Results identify baseline, candidate, semantic dataset/operation contract,
scenario, metrics, objective policy, budgets and status. Explain passive/item/
skill/support changes, constraint margins, cost where known, and active effects
that caused the difference. Show uncertainty/unsupported coverage explicitly.
Expose JSON/CSV and a comparison/report model suitable for later visualization.

Keep immutable raw originals and independent expected observations. Validate
full native outputs/availability against stable PoB references under explicit
metric tolerances; independently test laws and controlled mutations. Narrow
known PoB bug exceptions are visible exclusions, not passing rows. Deterministic
native tests include serial/parallel and fresh/cached execution. No retry-until-pass.

Development begins with five supplied builds, closest-complete-first. Add holdouts
with different mechanics/interactions, progression and non-meta builds, using
curated sources such as poe.ninja after verifying export support. Corpus builds
must pass the ordinary import/evaluate/optimize path unchanged. Test structural
generality; data-only coefficient support is not whole-build breadth.

Recovery may save unverified candidates with explicit status. Freshly validate
before trusted reuse. Distinguish exact resume from a warm start and declare
semantic/budget compatibility. Supported export/reimport preserves owned meaning,
not incidental source UI state.

## Visualization and application interfaces

Expose typed APIs for project edits, validation, evaluation, optimization,
progress, cancellation and results. UI does not duplicate formulas or repair data
silently. Later compare a Tauri desktop host with shared web/desktop UI candidates
such as Yew or Dioxus. Rust/Wasm portability, tree rendering, accessibility,
startup/data memory and cancellation latency belong in that evaluation.

Keep evaluation off the UI thread. Browser hosts explicitly supply worker,
filesystem and progress capabilities; support single-worker operation when shared
memory is unavailable. JavaScript/TypeScript is permitted for a justified UI,
not for new core/data tooling. Autocomplete uses bounded queries/indexes over
loaded definitions rather than a database dependency.

## Acceptance criteria

- First complete original: new format only, ordinary CLI/library path, complete
  selected input/mechanics/requirements, deterministic native results and stable
  reference parity; old production paths removed.
- Native search milestone: one genuine legal axis, locks preserved, independently
  verified improvement or correct no-improvement result, measured costs.
- First usable optimizer: all joint dimensions, at least two required skills and
  two exact item locks, allowed class/ascendancy changes and legal exports. Include
  an exhaustively checked synthetic case requiring a coordinated move when all
  single-change improvement paths stall.
- Breadth: remaining originals and independent holdouts use no fixture-specific
  path; report coverage and failures by mechanic intersection.
- Performance/quality: recorded hardware, 5/15/30-minute comparisons, several
  seeds, equal-budget baselines, bossing and mapping assumptions and scaling data.
- Distribution: reproducible versioned packages, no native PoB/Lua/authoring
  requirement, reviewed license/notices, same shared behavior in CLI and GUI.

## References and deferred decisions

The [prior-art review](prior-art-and-product-review.md), including other game
optimizers, informs search/product comparisons; it does not override these
requirements. The [architecture review](architecture-review-2026-10-22.md) drives
the current migration. The [implementation plan](implementation.md) records the
remaining public model decisions, exact checkpoints and next action.

Use [PoE2DB](https://poe2db.tw/us/) and [PoEDB](https://poedb.tw/us/) for targeted
lookups only; do not scrape or mass extract. Source versions/provenance and
independent calculation evidence still matter. Distribution licensing, GUI choice
and live trade acquisition policy are deferred decisions, not blockers to the
accepted migration and first build.
