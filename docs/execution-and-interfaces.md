# Parallel execution, interfaces, and visualization

Status: proposed implementation architecture. The current executable remains a minimal
CLI scaffold; Rayon integration, core crates, run artifacts, reports, and a GUI are not
implemented. These requirements extend [the main design](design.md).

## Reusable libraries with a CLI first

Implement the optimizer as a Cargo workspace with reusable Rust libraries and thin
application layers. Move functionality into these boundaries as it is implemented:

| Planned package | Owns | Dependency direction |
| --- | --- | --- |
| `poe-optimizer-core` | Problem/candidate types, metric and evaluator interfaces, scoring, search, execution limits, run events/results | Independent of CLI, Tauri, webviews, and a particular Lua host |
| `poe-optimizer-pob` | PoB game adapter, Lua worker supervision, metric mappings, XML import/export | Implements core interfaces; depends on core |
| `poe-optimizer-report` | Versioned artifact encoding and presentation models, JSON/CSV export, HTML report generation | Depends on core result types; never owns calculation or search rules |
| `poe-optimizer-cli` | Flags/config loading, composition of adapters, terminal progress, exit codes, report commands | Calls core, PoB adapter, and reporting APIs; binary remains `poe-optimizer` |
| Future Tauri application | Goal editor, run control, charts, build comparison and export | Calls the same libraries through a small Rust application layer |

The names are proposed; do not populate empty packages before their implementation starts.
Create the core/CLI boundary with the first working engine functionality, rather than
implementing search inside the CLI and extracting it later. Keep pure data/model modules
separate from scheduling within core; split further if concrete dependency needs justify it.

The library accepts typed inputs and returns typed errors/results. It must not print to
a terminal, exit the host process, parse application arguments, or require a webview.
The composition layer owns runtime/resource lifetimes and supplies the evaluator and
artifact destination. Multiple runs in one host share a resource manager rather than
each silently claiming all cores.

Problem data must express multiple required skills and exact equipped items, independent
locks, and the joint class/ascendancy/tree/gear/support/supporting-skill domain. Class/ascendancy
alternatives are target-build plans; do not imply an executable transition from the imported
character without separately validating that transition.

Proposed library operations are validate/resolve problem, list capabilities, start run,
observe progress, cancel, retrieve result, and export a verified candidate. A run handle
provides cancellation, a progress receiver, and completion. These are logical contracts,
not finalized Rust signatures. Keep domain types independent of a chosen async runtime
and of Tauri command macros.

## Multicore execution is an initial engine requirement

Parallelize independent work throughout the pipeline:

- Candidate generation and legality across class/ascendancy, passive, equipment, support-gem,
  and supporting-skill proposals, with task-local scratch state.
- Canonicalization, hashing, and independent score calculations.
- PoB evaluations across isolated worker processes.
- Independent search starts/islands, sharing the same evaluation service and resource budget.
- Required scenario/skill-selector calculations and final-candidate verification when their
  dependencies allow it; initial benchmarks cover bossing and mapping.

Use [Rayon](https://docs.rs/rayon/1.12.0/rayon/) for Rust CPU work. Construct an explicitly
sized local pool with [ThreadPoolBuilder](https://docs.rs/rayon/1.12.0/rayon/struct.ThreadPoolBuilder.html);
a reusable library should not configure the host's global pool. Pool lifetime belongs
to the engine runtime, shared across its runs. Keep worker IPC, file writes, and blocking
process waits on the supervisor/I/O path rather than occupying Rayon compute threads.

PoB's mutable Lua globals remain isolated: one live VM per evaluator process and one active
calculation per process. Adding threads around one locked Lua VM would not parallelize
the oracle. An embedded Rust/LuaJIT evaluator must preserve these ownership rules and
process isolation until measured evidence justifies changing them.

### CPU, memory, and admission control

Expose `jobs = "auto"` or an explicit positive count for the total CPU-work concurrency
budget. Auto starts from the host's available logical parallelism and reports the resolved
value; it is an estimate, not a physical-core or performance guarantee. Allow a user to
reduce usage for interactive work or select a higher limit explicitly.

Also expose evaluator-worker and memory limits. Resolve one shared budget across runnable
Rust tasks and active Lua calculations; do not independently allocate all cores to both
pools, or create a new full-size pool per restart. The coordinator admits bounded compute
batches and evaluator requests under that budget. It may partition overlapping stages or
run wider batches while the other stage is idle. Preserve a one-core execution path.
Never acquire blocking resource permits inside Rayon jobs that can hold resources needed
by another stage to finish.

Size evaluator processes from both CPU capacity and measured worker memory. Keep explicit
headroom for tree/data snapshots, candidate archives, cache, queues, and reports.
A worker-memory estimate is an admission heuristic, not a hard memory guarantee: monitor
process memory and stop admission/recycle or terminate workers under the configured policy.
If an isolated fresh process cannot fit, fail clearly instead of repeatedly launching it.
Record requested and resolved limits, the estimate used, peak usage, and any throttling.
A process lifetime limit may be needed if long runs expose memory growth.

The M1 spike measures cold/fresh evaluation and verifies persistent-worker reset behavior.
The first search release uses a bounded multicore evaluator pool; fresh-process evaluation
remains the correctness reference. Parallelism must not wait for a rewrite of calculations
in Rust.

### Work distribution and shared state

Use bounded proposal, evaluation, and result queues so fast generation cannot consume
unbounded memory. Keep sufficiently many independent candidates or search starts available
to fill workers. Schedule work dynamically in throughput mode, so one expensive build does
not make every other worker idle. Batch cheap Rust operations to amortize scheduling overhead.

Share immutable problem/tree data. Keep mutable search state local to each island, with
a coordinator merging feasible archives and optionally exchanging a few diverse candidates.
Independent starts are logical tasks, not dedicated threads. Island count is a search
parameter and can exceed worker count; it must not multiply the CPU or evaluation budget.

The cache needs an in-flight reservation per canonical evaluation key. Concurrent duplicate
requests join the same calculation instead of spending the budget repeatedly. Publish a
complete typed result atomically, release reservations on failure, and never hold a global
cache/archive lock while evaluating or waiting for IPC. Fresh finalist verification uses
an explicit verification path that bypasses completed-cache reuse and in-flight deduplication,
starts an independent fresh worker, and consumes its reserved evaluation attempts. Prefer short critical sections
and local batches; profile contention before introducing more elaborate sharding.

Assign evaluation-budget tokens centrally before dispatch for every actual PoB calculation,
including extra skill-selector/scenario passes, retries, and final
verification. An attached duplicate/cache hit does not consume a second token. Once a
calculation is dispatched, its attempt counts even if cancelled or failed. Queue entries
must not conceal unaccounted running attempts. All islands use the same deadline and
reserved final-verification budget described in the main design.

Cancellation stops new admission, signals CPU tasks cooperatively at bounded chunk
boundaries, and terminates evaluator processes that exceed their allotted grace period.
Reap child processes and complete the run with a typed termination reason. Never report
a partially verified candidate as a verified recommendation.

### Reproducibility and throughput

Provide two explicit scheduling modes using the same engine:

| Mode | Selection behavior | Expected use |
| --- | --- | --- |
| `throughput` | Consume completed evaluations and keep workers supplied; search choices may depend on completion order | Normal runs seeking the best result within elapsed time |
| `deterministic` | Fixed logical batches/epochs and candidate IDs; process completed batch results in stable order | Regression tests and algorithm comparisons |

Derive random streams from the run seed and stable logical task identifiers, never OS thread
IDs. Deterministic mode uses a fixed evaluation budget and batch/island settings independent
of worker count, with stable reductions/ties. Fix the initial cache to empty or a recorded
snapshot, and order cache publication/eviction and evaluation-token admission by logical
candidate/batch order. Completion-driven cache changes must not alter how much search fits
the attempt budget. Matching results additionally requires the
same versions, inputs, floating-point behavior, and absence of timing-dependent failures.
Wall-clock cutoffs, cancellation, and timeouts can truncate different work; do not claim
identical outcomes from a seed alone. Record effective scheduling parameters in each run.

Keep a single-worker reference mode for debugging, alongside multicore correctness checks.
Both modes evaluate the same candidate semantics and obey the same hard constraints.

### Scaling acceptance

Benchmark representative fixtures at 1, 2, 4, 8, and higher worker counts available on
the test machine. Include physical/logical core configuration, memory, OS, and runtime
versions. Measure oracle throughput, end-to-end time, CPU utilization, worker idle time,
queue/cache contention, memory growth, cancellation latency, and time to first feasible result.

Separate fixed-candidate evaluation scaling from search quality. Equal-evaluation-budget
comparisons test algorithm quality; equal-wall-time comparisons test the benefit of more
cores. Run several seeds and show variance. Cold startup, memory duplication, sequential
selection, and IPC may limit scaling, so set performance targets from measurements.
Include a multicore smoke test in engine CI and run larger scaling benchmarks separately.

## Run data and visual output before a GUI

Use versioned structured output as the shared source for CLI reporting, offline visualization,
and the eventual frontend. Proposed artifacts in each run directory:

| Artifact | Contents |
| --- | --- |
| `project.json` | Portable resolved seed references, allowed domain, goals, named scenarios and dependency identities |
| `manifest.json` | Resolved problem, source/runtime/schema versions, seed, requested/resolved resource limits and scheduling policy |
| `result.json` | Completion state, explicit comparison baseline IDs, verified alternatives, metrics/units, constraints/slack, coverage status, mutation summaries and diagnostics |
| `progress.jsonl` | Sequenced progress snapshots and incumbent changes with elapsed time and evaluation counts |
| `candidates.csv` | Optional flattened comparison data with metric IDs, units, scenario IDs and verification status |
| `builds/*.xml` | Verified candidate exports with stable IDs linked from results |
| `report.html` | Optional offline visual report generated from saved artifacts |
| `checkpoint.json` | Versioned, atomically written recovery state; explicitly declares warm-start versus exact-resume capability |

Persist bounded/sampled progress and retained alternatives by default, not every candidate
or full build on every event. Make detailed traces an explicit option with storage limits.
Use stable run/candidate/scenario IDs and record metric definitions so configurable goals
remain interpretable outside the original process.

Proposed event kinds include started, progress, incumbent-changed, verification, warning,
and completed. Each envelope carries schema version, run ID, sequence number, and timestamp.
Separate replaceable progress snapshots from terminal results and important diagnostics.
Coalesce/drop redundant progress under load; persist important events or retain them in
the authoritative run state so a slow/disconnected UI cannot block the optimizer or lose
completion. Consumers can request a fresh snapshot after a gap.

Start visualization with an optional offline HTML report created by the CLI:

- Before/after tables for the user's chosen metrics, constraints, and slack.
- Best feasible objective and constraint violation over evaluations and elapsed time.
- Candidate comparisons and, when supported, Pareto scatter plots with user-selected axes.
- Class/ascendancy choices, passive additions/removals, item changes, support assignments,
  and supporting-skill/effect changes; add a tree overlay once tree-layout export exists.
- Assumptions, source provenance, and clear verified/infeasible/error states.

Labels and chart axes come from the metric registry, not fixed DPS/EHP field names. Plot
infeasible and unverified samples distinctly. Report generation reads completed artifacts
without rerunning the search, so users can regenerate views or later open old runs in a GUI.

## CLI and later Tauri application

The CLI is the first application over the libraries. Proposed commands include capability
listing, problem validation, evaluation, optimization, and rendering saved-run reports.
Human-readable progress goes to stderr; machine-output modes reserve stdout for structured
data. Ctrl+C maps to core cancellation, and errors/exit codes distinguish invalid input,
evaluation failure, and a completed search with no feasible result.

Include baseline-aware candidate comparison and saved-result reranking in the CLI workflow.
Reranking must validate stored metric compatibility and identify the retained candidate set;
it does not imply a new search. Recovery initially warm-starts a new run from saved
candidates. Also retain evaluated-only recovery seeds with explicit status, since a crash
may happen before final verification; freshly validate them before trusted reuse in a new
run and independently verify all recommendations. Exact resume requires a compatible checkpoint of search/random state, budget
ledger, and pending work as described in the [product review](prior-art-and-product-review.md).

No new commands or output flags are implemented by this design update. The example
configuration describes future execution/output options alongside the existing user goals.

For the later desktop application, Tauri is a candidate because it combines a Rust backend
with a webview frontend ([architecture](https://v2.tauri.app/concept/architecture/)).
A thin Rust command layer would call the same engine/report libraries. Long runs execute
off the UI thread and return a run ID/handle promptly; bounded progress travels through
a bridge such as [Tauri channels](https://v2.tauri.app/develop/calling-frontend/#channels).
The frontend never has to parse terminal output or reimplement feasibility and scoring.

The first GUI should import a build, discover supported metrics, edit objectives/constraints,
choose resource limits, start/cancel runs, inspect progress and candidate changes, compare
trade-offs, and export a chosen verified build. Reuse report presentation models where useful.
Choose the web frontend framework when this milestone begins.

Confirm Tauri's packaging, native Lua worker/ABI distribution, platform prerequisites,
and UI responsiveness in a small prototype before committing to the framework. Keep
Tauri dependencies in the application package so CLI/library builds remain independent
of the desktop toolchain.
