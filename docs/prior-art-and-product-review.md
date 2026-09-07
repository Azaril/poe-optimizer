# Product review and WoW optimizer prior art

Reviewed: 2026-09-07. This is a design review, not an implementation report or an audit
of the referenced optimizers' internals. The existing [design](design.md) and
[execution/interface plan](execution-and-interfaces.md) remain the implementation proposal.

## Review conclusion

The calculation boundary, configurable goals, legality/feasibility distinction, parallel
runtime, and CLI-to-GUI path are already covered. The main missing pieces are the workflows
around the search: selecting a meaningful baseline, checking whether a build is supported,
comparing concrete options, explaining recommendations, iterating on goals, and recovering
long-running work.

The user has confirmed that the first usable optimizer must jointly search class,
ascendancy, passives, equipment, support gems, and supporting active skills. It must support
1..N required skills and 1..N exact equipped item instances, with independent locks on other
choices. Finite allowed pools bound the problem; they must not silently remove requested
search dimensions. Internal implementation milestones may be narrower, but do not constitute
the first usable optimizer until the joint workflow works.

The initial quality target is 5–30 minute runs with configurable budgets and benchmarks
covering both bossing and mapping. This is a target for useful best-found results, not a
promise of an optimum or a fixed runtime. Prioritize trustworthy evaluation, coupled moves,
comparisons, and recovery before adding more elaborate search algorithms. Cross-class
search is a confirmed requirement; its adapter capabilities are not implemented or proven.

## What to borrow from WoW tools

The facts in the second column come from official documentation or public source schemas.
The proposed adaptations are our design recommendations. Some references document historical
versions; they establish prior art, not a promise about current feature availability.

| Reference | Verified pattern | Adaptation for this project |
| --- | --- | --- |
| [Ask Mr. Robot Classic](https://www.askmrrobot.com/classic) | The interface separates Best in Bags, Upgrade Finder, and Best in Slot; it also exposes minimum-stat and enchant-change preferences. | Give owned-inventory optimization, prospective upgrades, and hypothetical item-pool exploration distinct modes with clear availability assumptions. |
| [AMR customization tutorial](https://blog.askmrrobot.com/optimizer-customization-tutorial/) (2019) | Users can lock or exclude choices and re-optimize around practical preferences. | Support independent locks, exclusions, permitted mechanics, and change-cost preferences. Presets must expose their resolved settings. |
| [Raidbots Top Gear sidegrades](https://support.raidbots.com/article/61-top-gear-sidegrades) (updated 2023) | Groups statistically similar alternatives and keeps the equipped setup prominent when it qualifies. | Offer practical near-equal alternatives and retain the baseline. Use explicit user tolerances for presentation/preferences, not WoW simulation error formulas. |
| [Raidbots Droptimizer](https://support.raidbots.com/article/59-droptimizer-how-does-it-work) (updated 2023) | Ranks possible drops by acquisition source, changing one item at a time; the documentation explains that this does not search all combinations. | Label one-change upgrade comparisons separately from joint tree/gear optimization. Show affordable upgrade bundles later when inventory and cost inputs support them. |
| [Raidbots baseline mismatch report](https://support.raidbots.com/article/73-great-vault-and-optimized-current-gear) (updated 2026) | Documents misleading comparisons when some rewards are measured against optimized owned gear and others against equipped gear. | Put baseline ID, scenario identity, and evaluator identity on every reported delta. Never silently mix comparisons against the seed and against an optimized incumbent. |
| [Raidbots Smart Sim](https://support.raidbots.com/article/55-smart-sim) (updated 2023) | Uses staged simulation precision to spend more iterations on promising candidates. | Transfer the idea of allocating computation selectively, using legality checks, cache reuse, and validated proposal heuristics. Do not pretend repeated deterministic PoB evaluations improve statistical precision. |
| [Raidbots stat-weight warning](https://support.raidbots.com/article/66-beware-of-stat-weights) | Warns that local stat weights miss interactions and recommends evaluating concrete combinations. | Keep marginal scores as proposal hints. Verify whole builds, especially changes that cross thresholds or depend on other changes. |
| [Raidbots unsupported fight styles](https://support.raidbots.com/article/43-unreliable-fight-styles) (updated 2025) | Documents scenario/model combinations that can produce unreliable or meaningless results. | Show support coverage, metric applicability, and conditional assumptions before search and preserve that information in reports. |
| [SimulationCraft ProfileSet](https://github.com/simulationcraft/simc/wiki/ProfileSet) | Supports named variations of a baseline, selectable metrics, summary reports, and parallel profiles with an explicit division of the thread budget. | Use named candidate comparisons and a shared resource budget. Keep scenario identity and profile changes explicit; avoid nested full-machine pools. |
| [SimulationCraft output](https://github.com/simulationcraft/simc/wiki/Output) | Supports machine-readable JSON and richer HTML output separately. | Preserve versioned run data as the source for CLI, offline reports, and GUI views. |
| [WoWSims API schema](https://raw.githubusercontent.com/wowsims/mop/master/proto/api.proto) | Bulk settings include item candidates and frozen slots; request types include progress/abort and split/combine operations. | Make domains/locks portable inputs and job control a first-class API. Its stochastic iteration splitting is not our candidate-evaluation parallelism. |
| [WoWSims UI schema](https://raw.githubusercontent.com/wowsims/mop/master/proto/ui.proto) | Saves gear, talents, encounters and settings; pairs requests/results and current/reference runs; represents caps and breakpoints. | Save complete optimization projects and references. Add typed piecewise preferences when richer scoring arrives, keeping them distinct from hard constraints. |

### Where the analogy stops

WoW and PoE have different search domains, availability models, skill interactions, and
evaluation semantics. A workflow pattern does not establish that its algorithm scales to
PoE's connected tree, support interactions, or rolled rare items. We have not reverse
engineered AMR or audited the complete WoWSims optimizer implementation.

SimulationCraft exposes sampling-error/confidence settings in its
[options documentation](https://github.com/simulationcraft/simc/wiki/Options#error-confidence).
Our PoB oracle is intended to produce the same result for identical resolved inputs.
Repeated runs initially check reset/isolation and parity; separate search seeds measure
search variability. Neither yields a probability that a modeled improvement will occur
in gameplay. Keep numerical tolerance, search variability, mechanic coverage, and scenario
sensitivity separately labeled.

Likewise, “upgrade source” information needs explicit PoE item-pool and acquisition inputs.
Do not infer drop rates, availability, crafting cost, or expected farming value from a
calculator's ability to represent an item.

## Recommended product additions

Confirmed scope requirements below belong in the first usable joint optimizer (M3).
Other additions retain proposed priorities. The decision register records the user's
answers; implementation and fixture validation remain separate work.

| Priority | Gap/addition | User value and acceptance criterion | Proposed stage |
| --- | --- | --- | --- |
| Required | Joint class/build domain and multiple locks | Search classes, ascendancies, passives, equipment, support gems, and supporting active skills together. Retain 1..N required skills and 1..N exact equipped item instances. Separate skill requirements, setup locks, and slot locks; reject unsupported requested capabilities rather than silently freezing them. | M2 schema; M3 acceptance gate |
| High | Preflight support and assumptions report | Identify selected groups/parts/sets, game/data versions, unsupported mechanics, required metric availability, and manually enabled conditions before spending the search budget. | M1 capability evidence; M3 user workflow |
| High | Explicit comparison baselines | Every delta identifies exactly which build and scenario it is measured against. “Equipped” and “best known from owned items” comparisons never silently mix. | M2 result model; M3 reports |
| High | Evaluate and compare supplied candidates | Users can compare a few manual alternatives before broad search is complete; preserve actual changes and common/explicitly differing assumptions. | M1 evaluation; M3 comparison workflow |
| High | Persistent project and recovery checkpoint | Save resolved inputs, verified alternatives, and evaluated-only seeds with explicit status atomically. Freshly validate evaluated-only seeds before trusted reuse. Label warm start separately from exact continuation. | M3 for 5–30 minute runs; exact resume later |
| Required | Support gems and supporting active skills | Explore explicit pools jointly with class, ascendancy, passives, and gear. Validate compatibility, counts, requirements, resources, and effect provenance; recompute candidate-derived buffs and other effects. | M3 acceptance gate |
| High | Actionable build-change explanations | Show class/ascendancy, node, item, and skill changes, point/category totals, travel costs, prerequisites, grouped changes, and objective/constraint deltas. Identify cross-class alternatives explicitly rather than implying every result is a direct respec plan. | M3; deeper ablations later |
| Medium | Rerank saved candidates | Reuse compatible stored measurements after goal edits, reporting that the retained set was reranked rather than newly searched. Missing metrics require explicit reevaluation. | M3 scalar workflow; richer policies grow later |
| Medium | Distinct alternatives and practical sidegrades | Retain a small configurable set of structurally different results; offer a user-defined improvement threshold and change-cost preferences while preserving exact scores. | M3; Pareto diversity with richer policies |
| Required / Medium | Bossing and mapping benchmarks / finalist sensitivity | Validate search quality in both explicit contexts. Optional sensitivity views expose gains that depend on a scenario; a mixed benchmark suite does not silently change the user's objective to a robust aggregation. | M3 benchmark coverage; robust policies in the later scoring stage |
| Medium | Useful failure diagnosis | Report each failed constraint, shortfall, unsupported input, and whether the seed was already infeasible. Suggestions never silently relax requirements or claim a proven minimal relaxation. | M3; proven conflict analysis only for tractable subproblems |
| Medium | Versioned import/old-run behavior | Keep old reports readable with original provenance; require explicit compatible reevaluation/migration before comparing against a new game/data version. | M3; migration support later |
| Later | Ranked upgrades and joint purchase plans | Distinguish one-item marginal gains from gains after joint re-optimization; record costs, inventory and baseline identities. Finite equipment search itself is already required in M3. | Additional workflows after M3; acquisition data later |
| Later | Practical playstyle preferences | Support declarative restrictions such as permitted mechanics or resource-use limits when measurable; unknown proxies do not become fabricated clear-speed or comfort metrics. | Add providers with capability evidence |
| Later | Shared inventory across several builds | Avoid claiming an item is disposable merely because one build does not use it. Multi-build allocation is a separate domain with explicit item multiplicity. | Only after a demonstrated user need |

### Support and scenario preflight

A successful PoB calculation is not a complete support certificate. Preflight should report
what the adapter can check, known unsupported/ignored effects, unvalidated mechanics, and
assumptions inherited from the import. Absence of warnings is not proof that all mechanics
are supported. The initial recommendation path is limited to the documented supported
fixture/domain scope; unsupported required metrics fail validation.

Keep calculation parity/fresh verification separate from mechanic-coverage status. If
experimental evaluation is later offered, preserve its explicit status in ranking, artifacts,
and exports rather than promoting it into the supported recommendation set.

Treat “use these skills,” “keep these supports,” “keep these equipped items,” “lock this slot,”
“keep this class/ascendancy,” and “avoid this mechanic” as independent choices. Required
skills use stable identities rather than one main-skill field. Required item instances must
remain equipped with their concrete modifiers/rolls; inventory membership alone is insufficient.
A required skill does not implicitly freeze its supports or make it an objective. Per-skill
metrics need explicit actor/group/part selectors, and multiple DPS values must not be summed
without a supported usage/inclusion model.

Freeze external encounter and usage assumptions, while recomputing candidate-derived effects
from the chosen skills, gear, passives, class, and ascendancy. Removing a supporting skill
must not preserve its buff through an inherited manual flag. Declared uptime assumptions
apply only where their mechanism is available. The optimizer must not enable arbitrary
assumptions to improve its score. Report provenance, applicability, and unsupported inputs.

The first usable release must exercise every confirmed search dimension. A supported finite
pool is a bound on choices, not permission to freeze classes, ascendancies, equipment, or
skills by default. Class/ascendancy changes require versioned legality, starting-tree and
point accounting, requirements, and export validation. Preserve all user locks or reject the
candidate; do not repair it by discarding a lock.

### Joint-search acceptance

Internal implementation stages may start with a single dimension to prove the evaluator.
The first usable optimizer additionally requires:

- Complete-candidate evaluation and export covering all six search dimensions, with fixtures
  retaining at least two required skills and two exact equipped item instances.
- Coupled proposals and dependency-aware repair across dimensions, including class/ascendancy
  changes. Single changes and alternating domain optimizers remain useful baselines, but
  cannot be the only search mechanism.
- Interaction fixtures where each one-change path stalls and coordinated changes improve
  the result; tiny exhaustive domains provide ground truth. Preserve legal infeasible
  exploration independently from verified feasible incumbents.
- Correct supporting-skill effects and resources after additions, removals, and replacements,
  without stale buffs or silently altered external assumptions.
- Search-quality comparisons against random, greedy, and alternating-domain baselines across
  several seeds, equal evaluation budgets, and representative 5, 15, and 30 minute budgets.
  Include explicit bossing and mapping contexts and report uncertainty in search quality.

A full candidate's legality is distinct from a playable sequence of intermediate respecs.
Cross-class alternatives must describe their class change and constraints clearly; the
initial optimizer does not promise an in-game transition sequence.

### Comparison and recommendation contract

A comparison record needs baseline/candidate IDs, game/evaluator/metric-schema versions,
scenario fingerprints, source metric values, and calculation/coverage status. Show
absolute changes and meaningful relative changes; a zero baseline cannot produce a
valid percentage gain. A comparison of deliberately different scenarios is an explicit
sensitivity view, not a single common-condition upgrade ranking.

Saved-result reranking creates a new view/problem identity linked to the parent run. It
checks that required measurements and version semantics match. It does not claim the old
search explored choices appropriate to the new objective; users can start a new search
from the retained candidates to improve coverage.

Near-equal result grouping is a presentation or soft-preference policy. It never changes
hard-constraint comparisons or hides raw scores. Report the tolerance and retain a clear
ordering according to the selected objective policy. A “worth changing” decision can
consider acquisition/respec costs only when those costs have explicit data providers.

Explain coordinated changes as a group. A resistance loss compensated by a second change
must not appear as two independently good upgrades. Optional ablation measurements should
be labeled as conditional comparisons; their deltas need not add up and do not prove
causal contributions.

### Persistent projects and long runs

A project bundles or references the immutable seed, allowed domain/inventory, goal
specification, named scenarios, and source identities. Saved runs attach results and
resource provenance to that project. Preserve relative-path portability or record missing
external dependencies explicitly; do not silently import different local files.

M3 recovery should persist the resolved problem, verified incumbents, and evaluated-only
retained seeds using atomic, versioned checkpoints. Record each candidate's actual status;
useful mid-run recovery must not depend on finalist verification having completed. An
evaluated-only seed is untrusted saved input: freshly reevaluate it and validate its domain,
locks, and feasibility before trusted reuse, and complete normal fresh export/re-import
verification before recommending it. Recovered verified results also require identity and
compatibility checks; never silently promote stale or mismatched measurements.

A warm start begins a new run with a new budget and parent-run identity. Revalidation
attempts consume the new run's budget. It must not claim to resume the original random/search
sequence. The 5–30 minute target makes this recovery path part of the first usable workflow.

Exact resume requires a more complete checkpoint: populations/frontier, random stream state,
stable logical IDs, archive/cache policy, attempts already charged, pending-request disposition,
remaining budgets, and evaluator/runtime compatibility. No live Lua state is portable.
Interrupted dispatched attempts remain charged; repeating them is a new attempt. Define
whether elapsed-time limits count paused time and record that policy. Exact resume remains
a later capability; revisit its priority if longer runs become a demonstrated requirement.

Named budget presets may be useful after benchmarks exist, but should remain editable.
Show estimated throughput and progress against the requested work budget, not an invented
percentage of the entire build space searched.

### Scenario robustness without changing the user's goal

Both bossing and mapping belong in the initial benchmark suite. Each benchmark has explicit
scenario settings and supported metric semantics; the user has not selected exact fixture
metrics or a mandatory aggregation policy for all runs.

Optional sensitivity checks operate on a separately declared scenario list and reserved
evaluation budget. Do not silently change external conditions or reinterpret an existing
single-scenario objective as a worst-case one. Candidate-derived buffs may change through
legal build choices and are recalculated consistently. Label sensitivity measurements clearly.
A robust recommendation requires constraints in all user-required scenarios and an explicit
aggregation policy; statistical confidence is not a substitute.

If robustness checks are requested but cannot finish within the budget, label them incomplete.
Do not display the candidate as having passed them merely because its primary scenario passed.

## Decision register and remaining input

D1–D6 have been answered by the user. These are confirmed product requirements, not claims
that the evaluator or optimizer already supports them.

| ID | Decision | Confirmed answer | Implementation consequence |
| --- | --- | --- | --- |
| D1 | First usable search scope | Joint passives, equipment, support gems, and supporting active skills; retain 1..N required skills and 1..N exact equipped items | These dimensions and multiple locks are first-release acceptance criteria, not later expansion options |
| D2 | Initial representative build | User supplied [this build reference](https://pobarchives.com/build/Dfz36mCq), followed by local exports `example.build` and `example.import.txt` | PoE2 XML decoded and structure/tree IDs inspected; source bytes preserved; Lua evaluation and metric parity remain unverified |
| D3 | Primary runtime expectation | 5–30 minutes, with configurable budgets | Benchmark useful best-found quality over that range and provide warm-start recovery; no runtime/optimality guarantee |
| D4 | Equipment priority | Equipment optimization is required alongside the other joint dimensions | Reinforces D1; gear is not deferred to a later domain milestone |
| D5 | First benchmark context | A mix of bossing and mapping | Include explicit fixtures/scenarios for both; exact metrics and any per-run aggregation remain configurable |
| D6 | Class and ascendancy scope | Search across classes and ascendancies from the first usable release | Include both in candidate identity, coupled mutations, game legality, provenance, and verified exports |

The user supplied `example.build` (planner JSON) and `example.import.txt` (PoB export)
after the archive page could not be retrieved. The latter decodes as URL-safe base64 plus
zlib to [PoB XML](../tests/fixtures/builds/pobarchives-Dfz36mCq.xml): a PoE2 level 96
Sorceress / Disciple of Varashta with tree version `0_5` and a selected Kelari minion
skill. [Metadata](../tests/fixtures/builds/pobarchives-Dfz36mCq.metadata.json) records
source hashes, 130 allocated node IDs present in the pinned tree, 19 skill groups, and
16 referenced item records. The originals and decompressed XML bytes remain unchanged.

This verifies decoding and structure, not evaluator parity. Full DPS inclusion flags,
three named skill entries without stable IDs, and granted/manual group provenance need
adapter resolution. Cached source damage values must not be treated as fresh measurements.
The planner's 0.5.5 title is a source claim, not independent game-patch verification.
No product-scope answer is pending. Exact benchmark metric selections, usage assumptions,
and run-specific skill/item locks remain explicit configuration when running the optimizer.

User goals, exact thresholds, locked choices, and resource budgets remain per-run
configuration. They are not global decisions the project needs to fix for every user.
Windows development, Rust core libraries, configurable objectives, multicore execution,
a CLI first, and a later GUI are already established direction.

Decisions that can wait: distribution license before publishing; desktop frontend framework
and Tauri packaging before GUI implementation; trade data sources before acquisition workflows;
PoE1 support after the PoE2 interfaces are proven. No additional user approval is needed
to research or implement routine details within the agreed scope.
