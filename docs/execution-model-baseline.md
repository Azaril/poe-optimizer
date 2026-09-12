# Execution-model baseline

A1 is **in progress**. The initial footprint and runtime matrix use
`9c756224f7c0063b71e277e4ef2ffe33a4c91d19` (2026-09-12). Subsequent measurements below
record their own source and executable identities. These checkpoints do not choose a
replacement or establish full native build parity:
**0/5 supplied originals complete native evaluation**.

The [investigation brief](rule-execution-model-investigation.md) defines A1–A4;
[semantic boundaries](execution-model-semantics-inventory.md) identify actual consumers
and proof obligations; the [performance inventory](execution-model-performance-inventory.md)
separates reusable harnesses from historical results. The [implementation record](implementation.md)
remains the living resume point.

## Implementation footprint

The [inventory script](../scripts/inventory_execution_model.py) reads committed Git blobs,
assigns each tracked entry one documented category, retains exclusions and hashes, and
reports dependency edges. At the baseline revision it reconciles **770 tracked entries:
713 included, 57 excluded, zero unclassified**. Inputs, vendored dependencies and the PoB
submodule are excluded from the logic inventory, with their entries still accounted for.

| Included role | Files | Physical lines or bytes |
| --- | ---: | ---: |
| Rust production-module locations | 216 | 95,956 lines |
| Dedicated Rust/tool test files | 320 | 139,839 lines |
| Lua source fixtures | 48 | 3,332 lines |
| Other included test data | 8 | 12,135 lines |
| Host/reference Lua | 9 | 1,364 lines |
| Scripts and example tooling | 18 | 3,717 lines |
| Generated definition payloads | 5 | 28,969,456 bytes |
| Authored data/acquisition policy | 6 | 8,323 lines |
| Documentation | 68 | 19,695 lines |
| Manifests, lock and repository configuration | 15 | 1,060 lines |

These are location-based size measures, not parsed executable LOC, maintenance hours or a
complexity ranking. Seventy-one production-module files contain lexical test/cfg markers;
inline tests, comments and strings remain in their parent counts. Generated JSON is data,
not implementation logic. The report records physical/nonblank lines separately.

Useful navigation categories include the source runtime (7,587 lines), modifier parser
(3,322), observer (4,776), lowerer (3,117), verifier (4,326), shared IR schema/compiler/owner
binding (2,192), parser acquisition (3,296), and definition acquisition (10,156).
Do not interpret their sum as removable overhead. Source compatibility also occurs in
item numeric/string helpers, scanning, import and validation. Conversely, the calculation
and preparation categories include the explicitly restricted Mace/Spark profiles; they
are not an inventory of universal game algorithms. Mixed-seam annotations identify these
overlaps without counting files twice.

The native crate's declared nonoptional normal internal dependency closure is core, data,
engine, import and native; it contains no Lua edge. The CLI's optional/default PoB backend
and Lua test dependencies are separate. This is a manifest inventory; the prior native
runtime dependency check remains the stronger build-level evidence.

## Fresh measurement protocol

The native-only release CLI was rebuilt outside the timed window using Rust 1.98.1 on
Windows 11 Pro 10.0.26200 x64, AMD Ryzen 9 9950X3D (16 physical cores, 32 logical processors).
The unchanged package is schema 29, 26,286,752 bytes. No PoB process participated in these
measurements. There were no overlapping project benchmark/build/test workloads; OS
background activity, file cache, frequency and scheduler placement were not controlled.
"Fresh" means a new process, not cold storage.

The measured matrix contains **51 fresh processes**:

- All five [original XMLs](../tests/fixtures/builds/breadth-20260908/index.json), three
  `prepare-build` repetitions each: 15 processes. The corpus index's source-import hash
  matches the supplied multiline `example.import.txt`.
- Two admitted diagnostic profiles, Mace and Spark, in prepared/document modes at
  1/4/32 workers, three repetitions each: 36 processes. Prepared mode makes 10,000 calls
  per run; document mode makes 1,000, for **198,000 completed calls**, zero failed/late.
  Worker order rotates between repetitions and mode order alternates. Pilot runs are
  recorded separately and excluded. No uncounted calculation warmup is applied.

The external observer polls Windows `GetProcessMemoryInfo` every 25 ms and records the
maximum reported `PeakWorkingSetSize`; the final query succeeded for every child. This
is the OS process working-set high-water mark, including transient loading/validation,
not live dataset size, retained heap or cumulative allocated bytes. CPU/allocation
profiling is still needed to attribute it. External wall times also include completion
detection by this polling loop: up to a poll interval plus scheduling delay after child
exit. The CLI's internal initialization/iteration timers are unaffected by that delay.

## Preparation of the supplied originals

| Original | Median observed wall time, ms (range) | Median peak working set, MiB | Reported issues |
| --- | ---: | ---: | ---: |
| 1 | 2,029.6 (2,029.5–2,030.3) | 716.9 | 111 |
| 2 | 2,031.2 (2,028.9–2,054.2) | 717.1 | 94 |
| 3 | 2,030.3 (2,004.9–2,032.0) | 717.1 | 106 |
| 4 | 2,034.4 (2,030.3–2,058.2) | 716.9 | 115 |
| 5 | 2,030.8 (2,029.6–2,055.3) | 717.2 | 67 |

Every report is **incomplete**, with calculation not run and full parity not established.
The time includes loading, reached preparation stages and report publication. Repetitions
retain the same issue/stage counts per original. Those counts describe reached diagnostic
instances, not the total missing mechanics or a progress percentage. General configuration
activation, skill/support/actor production and other assembly stages remain open; see the
[semantic inventory](execution-model-semantics-inventory.md) and [rollout](real-build-rollout.md).

## Repeated restricted-profile evaluation

Rates below are medians of three runs, in full typed API calls/second (min–max).

| Profile / mode | 1 worker | 4 workers | 32 workers |
| --- | ---: | ---: | ---: |
| Mace / prepared | 4,132 (4,128–4,158) | 13,746 (13,380–13,908) | 40,926 (39,785–41,087) |
| Mace / document | 433 (430–433) | 1,509 (1,476–1,536) | 4,480 (4,191–4,486) |
| Spark / prepared | 5,131 (5,080–5,157) | 17,027 (16,290–17,266) | 48,100 (47,058–48,169) |
| Spark / document | 516 (514–516) | 1,863 (1,854–1,877) | 6,733 (6,618–6,853) |

Prepared mode reuses validated profile input but still recalculates and constructs the
full result, including metrics, diagnostics and XML export. Document mode repeats parsing
and preparation. Both include worker/accounting/checksum overhead; neither measures the
pure numeric kernel or the optimizer's mutating-candidate path. Initialization and pool
creation are outside iteration timing. Result report serialization follows the iteration
window. The fastest windows are roughly 0.15–0.25 seconds: these are exploratory scaling
observations, not stable machine-capacity or long-run regression thresholds.

Across those 36 processes, backend/data initialization had a median of **1,989.3 ms**
(range 1,963.9–2,031.0). It is already included in the preparation timer, not an additional
cost. Median peak working sets for the profile/mode/worker groups were 716.2–716.4 MiB.
The later stage split below attributes this aggregate at public API boundaries; finer
loader/validation and memory costs remain unmeasured.

Canonical sample measurements agree within each profile across every mode, worker count
and repetition. Spark additionally has a finite metric digest for **every completed result**;
all per-result metric digests and all 18 run digests agree. Mace's default metric catalog contains unavailable
metrics, so its full finite checksum is absent: only its reported samples establish cross-run
measurement agreement. These digests do not cover XML exports, diagnostics or the rest of
the evaluation result. Completion counts do not establish per-call Mace numeric bit identity.

## Changing candidates: admission versus reuse

The unchanged [assembly harness](../examples/benchmark_assembly.rs) was separately rebuilt
in native-only release mode at the same revision/package. Three additional fresh processes
rotate worker order (1/4/32, 4/32/1, 32/1/4) and alternate mode order. Each contributes one
sample per mode/worker pair, calibrated to approximately 1,200 ms. These **18 samples**
are separate from the 51-process full API matrix above.

The supplied [diagnostic problem](../examples/action-timing-search.json) produces the same
1,806 admitted selections in every process: 129 allocations, 20 equipment selections,
seven support loadouts and 1,519 distinct output checksums. There are no rejected proposals.
These are deliberately bounded test axes, not all legal builds or a search-quality result.
The harness consumes only the problem's schema, template and equipment; its own corpus and
point/support budgets govern the run.

| Measured operation | 1 worker | 4 workers | 32 workers |
| --- | ---: | ---: | ---: |
| Fresh admission + measurement, calls/s | 44,864 (44,332–48,545) | 164,461 (162,487–169,106) | 411,986 (392,454–415,152) |
| Already-admitted measurement, million calls/s | 5.46 (5.42–5.56) | 20.99 (20.94–21.13) | 99.18 (98.95–99.45) |

Values are medians (min–max). Fresh mode clones the rotating selection, validates/resolves
it, recomputes actor resources and requirements, allocates/drops a handle and calculates
metrics. Reuse mode calculates metrics from already-admitted handles. Neither caches final
evaluation results. Both include checksum consumption; both exclude XML, JSON, diagnostics,
exports, deadlines, owned measurement conversion, objective scoring and proposal generation.
These rates therefore cannot be called whole-evaluator or optimizer throughput.

All 398,228,144 timed calls and 39,085,452 separately recorded calibration calls completed.
Actual samples span 1,005.5–1,264.2 ms; the 32-worker reuse mode hits the existing
100-million-call cap, hence its shorter windows. Every calibration/sample matches its
preflight rotating-sequence checksum. That wrapping integer checksum consumes metric
values/statuses and receiving, movement and action fields, but has collision risk and
excludes unavailable reasons and some timing fields. It is neither complete-result identity
nor independent parity. Sixteen materialized documents per process also match the native
measurement/evidence path outside timing; those are native self-consistency checks, not PoB
comparisons.

Median setup costs are 1,985.6 ms for dataset/backend, 7.83 ms for catalog construction,
3.17 ms for fixed component preparation, 6.93 ms for proposals, 43.58 ms for initial
admission of all 1,806 selections, 0.69 ms for initial measurements and 47.94 ms for the
16 document comparisons. Observed process peaks span 716.0–716.7 MiB; the same OS peak
qualification applies. Lower admission rates than reused calculation identify a meaningful
cost boundary within this workload, without demonstrating which execution model would
improve it.

Replay after building the example, rotating orders and using new stdout destinations:

```powershell
cargo build --release --no-default-features --locked -p poe-optimizer-cli --example benchmark_assembly
.\target\release\examples\benchmark_assembly.exe --problem examples/action-timing-search.json --sample-ms 1200 --repeats 1 --jobs 1,4,32 --modes admit-measure,measure
```

Evidence is `runs/a1-baseline/assembly/{summary,aggregate,run-1,run-2,run-3}.json` with
per-process stderr logs and `runs/a1-baseline/measure_assembly.py`. The report retains
source/problem/template/package/corpus hashes, calibration counts and exact commands.
Executable SHA-256: `a852f29d2efdaeeb2f793f679636a8c05762df804a0a31665c4e5ae65e74c082`.

## Attribution of dataset setup

Follow-up measurements use `be06c7f` plus the recorded timer-only example diff. Production
Rust/Lua, injected data, corpus generation, samples and checksums are unchanged. The
original `dataset_ms` still surrounds the whole setup; new fields partition snapshot
loading/construction, numeric data compilation and backend binding. Their small timing
and `Arc` overhead remains in the outer total. Each row is aggregated separately, so its
median need not sum with the other stage medians to the median outer total.

Three fresh native-only release processes on the same host retain the earlier data,
backend, problem, template and 1,806-selection corpus identities. Each performs the same
16 native document comparisons and one 100-ms-target, one-worker reuse sample, including
recorded calibration. Those short samples are successful harness checks; they establish
no new throughput claim.

| Setup stage | Median ms | Min–max ms |
| --- | ---: | ---: |
| Snapshot load, validation and catalog construction | 1,977.758 | 1,970.643–1,990.826 |
| Numeric/profile data compilation | 0.255 | 0.254–0.290 |
| Backend construction and first identity initialization | 4.356 | 4.348–4.840 |
| Outer dataset setup | 1,982.852 | 1,975.289–1,995.429 |

Snapshot work accounts for **99.74–99.77%** of the outer dataset setup in these runs.
It includes byte hashing, bounded JSON decoding, schema/semantic validation, section
checks, first-use reviewed passive capability keys and owned snapshot/catalog construction.
It is neither pure validation nor disk I/O. Compilation prepares the current numeric,
profile, passive and support data; it is not general source-program/parser compilation.
Backend construction includes first-use implementation identity hashing. No process caches
were reset. These first-use costs must not be mixed with reused-process measurements.

The source review identifies narrower profiling targets inside snapshot work: owned JSON
and typed-package representations during discarded-field checks; repeated serialization
and validation passes; canonical tree authentication; the first-use capability-key parse;
and section clones plus indexes retained beside the original package. Some repeated parser
validation streams borrowed data rather than allocating another whole JSON representation.
These are candidate explanations to test, not measured cost or memory shares. No validation
has been removed, and this checkpoint does not select a storage format or execution model.

Evidence is `runs/a1-baseline/stages/{summary,run-1,run-2,run-3}.json`, stderr logs,
`instrumentation.diff` and `runs/a1-baseline/measure_stages.py`; the hypothesis map is
`runs/a1-loader-attribution-review.json`. The reports bind the changed example source and
executable hashes. The same CLI as the preceding assembly replay applies, with
`--sample-ms 100 --repeats 1 --jobs 1 --modes measure`. Build the example first, then run
three fresh processes. Peak working set remains a whole-process observation, not retained
or allocated heap attributed to a stage.

## Attribution inside dataset loading

The next A1 experiment measures the original `a6670c7` loader in an isolated Git worktree.
The committed developer script, `scripts/profile_game_data.py`, inserts 34 pairs of timing
statements around intact source statements. Removing the inserted lines must recover the
original loader exactly. It changes no original scopes, validation calls, clone expressions
or error paths. A fixed thread-local counter array is initialized before the load; reporting
occurs afterward. The main project's loader, schema and package remain unchanged.

Three fresh Windows release processes per variant each perform two direct `bundled_snapshot`
loads. The control uses the original loader without timing calls. Both variants use the same
developer harness and unchanged schema-29 package. Run order alternates between control and
instrumented variants; there is no overlapping project build/test/benchmark workload during
the measurement matrix. This repeats the host/toolchain described above. OS cache state and
scheduler placement remain uncontrolled.

| Inclusive interval | First load median, ms | Later load median, ms |
| --- | ---: | ---: |
| Original control: full load | 2,006.157 | 1,707.374 |
| Instrumented: full load | 1,996.795 | 1,694.872 |
| Bounded JSON representation | 189.685 | 190.292 |
| Typed decode, discarded-field proof and scope cleanup | 744.000 | 742.832 |
| Package validation and scope cleanup | 957.423 | 665.806 |
| All catalog clone/construction calls combined | 88.660 | 86.521 |

The two full-load rows are separate variants. The following rows subdivide the instrumented
load and exclude its roughly 17 ms byte/trust phase, identity work and small parent residual.
Medians of separate intervals need not sum to the median total. The modest control/instrumented
differences are not evidence of a speedup: instrumentation changes code generation and binary
layout, and three samples do not isolate those effects from noise.

Inside typed decoding, creating a round-trip JSON value, checking discarded fields and
dropping that temporary takes 468.686 ms on the first load. Typed deserialization takes
176.681 ms; the parent residual is 98.578 ms, including cleanup of the supplied JSON value
and instrumentation overhead. These intervals do not independently measure allocation counts
or attribute every residual instruction to destruction.

Inside validation, manifest/section digest checks take 337.762 ms, including another whole
package JSON representation and its temporary cleanup. Creating the JSON representation for
numeric checks takes 165.107 ms, while the numeric walk itself takes 0.044 ms. That representation
remains alive until validation returns; validation's 97.467 ms residual includes its cleanup
and unmeasured/observer work. Catalog calls retain their original clone-plus-constructor scope;
the experiment does not separately measure clone, index or repeated-validation shares.

First-use passive capability validation takes 291.582 ms, including a 190.726 ms parse of the
trusted compiled package to obtain its capability keys. The inner parse runs exactly once in
each fresh instrumented process and zero times on its second load. Later passive validation
takes 1.012 ms. Its cold residual includes key extraction and temporary cleanup; the measured
whole-load difference is not exclusively attributable to the cache because allocator state
also changes. This identifies a bounded candidate for a more selective trusted-data decode.

All twelve snapshots retain exact canonical input bytes, identities, reviewed trust, all eight
catalog/package equalities and parser owner binding/digest. The checks run outside the load
timer, followed by explicit snapshot destruction (roughly 59–61 ms). A later load therefore
follows both verification and destruction of its predecessor; it is not an isolated hot-cache
microbenchmark. The four existing loader regression cases run in both variants and pass:
embedded/external equivalence, recursive duplicate/resource limits, unknown nested/ambiguous
fields, and snapshot ownership. Strict isolated-example Clippy and Rust 2024 formatting pass.
These checks cover the modified execution path, not every dataset or full native build parity.

The evidence points to bulk JSON construction, proof passes and lifetime costs as major setup
work. It does not measure candidate calculation or show that removing the source interpreter
would remove those costs. A1 still needs allocation/retained-owner measurements and a recorded
update exercise before it can compare total architecture cost. No storage or execution model
is selected, and complete native coverage remains **0/5**.

Reproduce with a new directory under `runs/`, reviewing the generated diff before measurement:

```powershell
python scripts/profile_game_data.py prepare --revision a6670c730195c6d38d2d090b8a61507be44ead96 --output runs/loader-profile-new
python scripts/profile_game_data.py measure --output runs/loader-profile-new --repeats 3 --loads 2
```

The tool leaves its detached worktree and results for review and never removes or modifies
the main checkout's loader. Source anchors are scoped to their actual functions and fail
before worktree creation if they are absent or ambiguous. An initial setup attempt exposed
a shared statement in the authoring path; no measurements used that attempt.
Evidence is `runs/a1-loader-phases-02/{prepared,results,summary}.json`, `instrumentation.diff`,
the two release binaries and build/check logs. `prepared.json` binds each interval to its
original source lines and statement digest; the result binds both executable hashes and
producer sources. Memory attribution and executable-size comparisons are outside this run.

## Trusted passive-key loader simplification

The phase measurements identified an avoidable full JSON tree used once per process to
obtain the trusted package's passive capability keys. The production cache now uses a private
Serde projection over those same compiled bytes and retains the same `BTreeSet`. It does not
load caller-supplied data through this projection. The public bounded decoder, duplicate and
unknown-field checks, digests, tree authentication and capability-set validation are unchanged.
No source pin, schema, package, dependency or evaluator admission changes.

A separate comparison reruns the preserved original executable alongside the optimized loader
using the identical developer harness, counter module and data. Neither loader contains timing
instrumentation; all counter readings remain zero. Three fresh processes per version each load
two snapshots, alternating version order, without concurrent project builds/tests/benchmarks.
The same Windows host/toolchain is used; OS cache and scheduler placement remain uncontrolled.

| Version | First load median (range), ms | Later load median (range), ms |
| --- | ---: | ---: |
| Original | 1,994.655 (1,989.415-1,995.042) | 1,737.306 (1,730.488-1,744.541) |
| Selective keys | 1,707.719 (1,707.327-1,710.532) | 1,718.174 (1,712.835-1,754.116) |

First-load median falls by **286.937 ms (14.4%)** for this host/package. Later-load ranges
overlap, so no reused-load improvement is established. Every one of the twelve snapshots
preserves exact canonical package bytes, identity/trust, all eight catalog equalities and
parser owner binding/digest. Verification and destruction precede the later load as in the
original protocol. This does not measure allocations or candidate/build calculation speed.

Three private projection tests compare the exact bundled key set with the former full-JSON
algorithm, exercise set semantics and reject invalid keys. Seven focused loader integration
cases pass, including both capability removal and structurally valid expansion rejected by
the exact capability guard. Existing accepted custom values remain accepted. Strict DATA
all-target Clippy, DATA WebAssembly compilation, workspace formatting and native-only release
builds pass. The profiler accepts both loader versions and restores their exact statements;
its new `cache.reviewed_passive_projection` interval measures deserialization, while final
set collection remains in the parent interval. These are local checks, not a hosted CI result.

Evidence is retained in `runs/passive-key-projection/`: `benchmark-inputs.json`, raw reports,
`benchmark-results.json`, `benchmark-summary.json`, test/build logs and `validation.json`.
The isolated worktree starts at `12d02a4`; the original executable is the preserved control
from `runs/a1-loader-phases-02/`, rerun for this comparison. Independent reviews are
`runs/a1-passive-projection-review-by-lowerer.json` and
`runs/a1-passive-projection-benchmark-review-by-lowerer.json`. This is a measured simplification
within the current architecture, not an A2 model selection or additional complete native build.

## Supported data-update replay

The existing `tests/dataset_search_cli.rs` diagnostic problem supplies two weapons and two
support choices. A retained release replay copies its literal problem and Mace fixture into
a fresh scratch directory, edits only Wooden Club's `physical_minimum`/`physical_maximum`
from 6/10 to 60/100, and uses the real `seal_package` authoring helper to refresh section
digests and validate the result. Only the weapons section digest changes. The shipped data,
fixtures, source pin and native algorithms remain unchanged.

One fixed native-only executable then runs five exhaustive searches: embedded baseline at
1/4 workers, identical external baseline at one worker, and custom data at 1/4 workers. Each
finishes with six evaluations including one fresh verification, no evaluation failures and
a consistent winner. Embedded/external baseline identities, feasible results and winners
agree. For each dataset the serial/Rayon feasible results and verified winners agree:

| Dataset | Verified alternative | Selected-hit DPS |
| --- | --- | ---: |
| Reviewed baseline | `smith/none` | 18.208693999999998 |
| Custom numeric data | `wood/brutality_i` | 130.0621 |

The custom result retains `custom_unreviewed` trust and its new package/catalog identities.
A sixth CLI invocation reevaluates the exported winner with that same dataset and obtains
exactly 130.0621 DPS. Export metadata, backend/data identity and XML hash agree. The executable
and all supplied inputs retain their hashes across all commands; PoB is not a native runtime
dependency. Fixture-specific edits belong to this developer replay, not the production input path.

All seven commands, including sealing, pass. Recorded single-run wall times are 3.235 s for
sealing and 1.814-2.041 s per CLI invocation. These include startup and reporting; they are
reproduction observations, not isolated evaluator rates or engineering-effort estimates.
The initial pretty-printed raw package was 53,150,688 bytes and correctly exceeded the
32 MiB authoring limit. Compact JSON reduces it to 26,286,755 bytes with identical decoded
content. No loader limit or production code changed to make the replay pass.

Evidence is `runs/a1-update-measured-02/report.json`, raw/sealed packages, exact argv, logs,
problem, XML and metadata. The failed attempt and its original driver are retained in
`runs/a1-update-measured/`; the corrected local driver is `runs/a1-update-replay.py`.
The same supported contract is covered by the committed regression
`custom_data_changes_ranking_consistently_across_workers_and_survives_export` in
`tests/dataset_search_cli.rs`. Reproduce with a native-only release CLI and release
`poe-optimizer-data` example `seal_package`, preserving one binary across data variants.
Custom package SHA-256 is `6cc0744c82a96c21bf17a2e5ef838f55941cd356427ec9e528fda3cb4023a3a2`.

This proves a supported balance change in the closed weapons projection. The wider source
item catalog remains unchanged. It does not establish a coherent upstream release refresh,
new effect-family implementation, migration/review hours or full-original native parity.
The following checkpoint executes the inconsistent-identity repair and unchanged-pin acquisition exercises.

## Identity reconciliation replay

The next replay changes only the numerical Mace profile's skill ID and the corresponding
attribute in the existing test XML. The unchanged native executable rejects preparation with
`Authored skill resolution differs from the closed numerical adapter`: one preparation
attempt, no search/winner, and neither XML nor metadata export. Package sealing succeeds;
the inconsistency is detected when authored and numerical skill resolution meet.

Repair operates on that exact specimen, preserving its edited profile and XML bytes. It
updates six reference values and one lookup key across `skill_identities` and
`skill_preparation`, including the declared canonical gem order. Source provenance remains
unchanged; reference renames reject collisions. Only those two payload sections and their
digests change relative to the failed specimen. Searches at one/four workers both complete
six evaluations with four legal alternatives, one consistent fresh verification and no
failures. Their feasible results and verified winner agree. The exported `smith/none` winner
reimports at the same 18.208693999999998 selected-hit DPS.

The same repaired object then receives the existing quoted/Unicode identity recipe and
renamed quest keys. This additional reference transformation touches 56 values and four
lookup keys. Locked `wood/brutality_i` searches at one/four workers each complete three
evaluations with one legal alternative and consistent verification. Escaped identities and
quest keys survive XML export; reimport obtains the same 13.656520499999997 selected-hit DPS.
These values come from the original numeric fixture, not the earlier tenfold damage edit.

All ten commands pass: three seals, the expected preparation rejection, and search/search/
reimport for each repaired case. Each custom package retains its own identity and
`custom_unreviewed` status; supplied inputs and executables remain unchanged. Evidence is
`runs/a1-reconciliation-01/identity/`, including the producing driver, raw/sealed packages,
XML, exact commands, complete reports and edit counts. The driver derives its input from
`tests/dataset_search_cli.rs`; no fixture-specific behavior enters production code.

This demonstrates data-only identity reconciliation in the admitted diagnostic model. The
coordination spans a legacy numerical profile and independently loaded source definitions;
it is not evidence that the interpreter caused the duplication. Counts describe this edit,
not engineering hours, minimum general update effort or a completed upstream migration.

## Pinned-source acquisition and rejection

Two fresh processes use one PoB-enabled release executable at `cfbde4e` and explicit
600-second completion limits. The first uses the default source path from the repository;
the second uses an absolute source path from the output directory. Both emit the identical
26,286,752-byte reviewed package, all 29 section digests and matching stable evidence.
Whole-command elapsed times are **17.804 s and 17.336 s**, including verification, process
startup, extraction, validation, serialization and publication. They are two reproduction
observations on the same Windows host, not stage timings or a performance comparison.

Independent checks authenticate the exact 129 consumed-source entries, all 1,082 files in
the source manifest and the extractor digest derived from 56 implementation/configuration
inputs. Each 966-entry gem setup/search order is a complete permutation. The two processes
have different orders, but their recorded owner/variant maps are equal and correctly replay
from the respective order. Only these validated traversal observations vary; they are kept
out of reproducible package semantics.

Static inspection identifies five full `GameDataLoader::from_bytes` calls along successful
public extraction: review generation, pinned-output validation, worker validation, parent
artifact validation and CLI loading. Three execute in the worker and two in the parent.
Existing reports expose no timing for these calls. Their separate costs and the necessity
of each check at its trust boundary must be established before proposing consolidation.
The isolated loader benchmark cannot be multiplied by five to attribute acquisition time.

A separate control copies only the complete manifested runtime source, verifies every copied
file, then changes Wooden Club's physical range from 6/10 to 60/100 in that copy. Extraction
rejects `src/Data/Bases/mace.lua` for its changed normalized byte length before publication;
stdout is empty and neither package nor evidence is written. Only that copied file differs.
The real submodule, manifest, data and executables remain unchanged. This validates the
source-pin boundary; it does not approve or implement a new upstream revision.

Evidence is `runs/a1-reconciliation-01/{build.json,acquisition/,source-control/}` and the
independent `runs/a1-acquisition-review-by-lowerer.json`. The exact producing acquisition
driver is retained beside its outputs. Review found that its unreached outer observer timeout
needed process-tree cleanup; the future driver adds that cleanup and lock-file recording,
separately from the completed runs. No timeout occurred and that new cleanup branch was not
exercised. The lock hash matches the prior checkpoint throughout this work.

These replays require no production changes. Sealing custom data remains separate from
acquiring authenticated source; normal CLI extraction also enforces the reviewed output
digest. A maintainer regeneration path still requires source compatibility review. Actual
upstream migration and new effect-family work remain open for A1/A2. The following checkpoint
measures dataset ownership separately. Complete supplied-original native coverage remains **0/5**.

## Dataset allocation and ownership

The developer-only [allocation example](../crates/poe-optimizer-data/examples/profile_allocations.rs)
measures the unchanged loader at `c9c3711c2b7fa1fa484bb71ba4f0c6dae5bb2931` with a counting
wrapper around Rust's `System` allocator. It adds no production instrumentation or dependencies.
One release executable passes its counter self-check in a separate process, followed by three
fresh sequential measurement processes on the Windows host described above. No build or test
overlaps those measurements. All 11 lifecycle phases have identical allocation counts and byte
totals across the three runs; elapsed times vary.

These are **requested Rust heap bytes**, not RSS or allocator-reserved memory. Reallocation
traffic counts the full old and new requested sizes, while live/peak accounting uses their
size difference; temporary copies inside the allocator are invisible. Cold means the first
dataset load in that process, not cold OS caches.

| Operation | Change in live requested bytes | Interpretation |
| --- | ---: | --- |
| First bundled snapshot load | +150,497,847 | Snapshot plus process state retained after loading. |
| Direct owned `GameDataSnapshot::clone()` | +77,584,683 | Copies package data; catalogs still share owners. |
| Drop that owned clone | -77,584,683 | All allocations made by this clone are released. |
| Move snapshot into `Arc` | +8,568 | One allocation for the shared snapshot. |
| Clone/drop an additional `Arc` handle | 0 / 0 | No new heap allocation; both handles reference the same snapshot. |
| Clone eight catalogs and the parser-program view | +88 | Two small metadata allocations; catalog/program data pointers remain shared. |
| Drop final snapshot `Arc`, retaining those owners | -77,729,659 | Releases snapshot-owned storage; retained catalogs remain usable. |
| Drop all retained catalog/parser owners | -72,732,828 | Releases the storage those owners kept alive. |
| Later load, then drop without extra owners | +150,453,831 / -150,453,831 | Returns to the same 44,016-byte process remainder. |

The first load requests 1,773,873,107 bytes of cumulative allocation/reallocation traffic
(about 1.65 GiB), with a peak growth of 647,813,440 requested bytes (617.80 MiB). Its
instrumented median is 1,912.696 ms; the later load's is 1,897.079 ms. These timings include
atomic counter overhead and do not establish a speedup. The 44,016 bytes remaining after
teardown are not attributed to a particular cache or called a leak. Per-statement allocation
attribution and allocator fragmentation remain unmeasured.

The production loading path returns `Arc<GameDataSnapshot>` and compiled engine data retains
that shared owner. The direct owned clone is an API contrast, not a measured worker operation.
Consequently these results support cheap definition sharing but say nothing yet about per-build
mutable state or parallel scaling. Snapshot/package duplication and transient loading storage
are comparison inputs; they do not establish interpreter overhead or a preferred replacement.

Every loaded/owned-copy snapshot is checked against the exact canonical package, identity,
reviewed trust, all eight catalog contents and parser owner digest/binding. Retained owners
preserve their original data pointers after snapshot teardown. Witness serialization/hashing
buffers are dropped before subsequent measured phases, although allocator/cache effects remain.
The counter self-check exercises successful allocation, zeroing, reallocation growth/shrink
and deallocation: 152 bytes requested and released, 96-byte peak growth, zero residual growth.
Failure injection and concurrent counter snapshots are outside this protocol.

Reproduce after building once, preserving the same executable for every invocation:

```powershell
cargo build --release --locked -p poe-optimizer-data --example profile_allocations
.\target\release\examples\profile_allocations.exe --self-check
.\target\release\examples\profile_allocations.exe
```

Run the last command in three fresh processes and retain each JSON output separately. Local
evidence is `runs/a1-ownership-01/`, including the producing driver, executable, build logs,
self-check, raw measurements and audited aggregate. Executable SHA-256 is
`28fff7c7870bc07eb0dfb088a1c4daefe3b33d81eb0a7e38b7b5d34ab803bcf2`; example SHA-256 is
`0eff5b7f8087008f14face45b59538b8d4cc0bc34e3050ac822c03eb85338a0a`.
Strict example Clippy and the release build pass. The data library, schema-29 package,
dependency lock, supplied inputs and PoB pin are unchanged. Full-original coverage is **0/5**.

### Source-session lifecycle measurements

The opt-in [lifecycle test](../crates/poe-optimizer-pob/tests/profile_source_sessions.rs) now
captures the actual initialized parser state from all five unchanged original builds. It
shares observation/lowering setup with the existing scanner regression through one
[test helper](../crates/poe-optimizer-pob/tests/support/source_program_parser_capture.rs).
The scanner retains its existing fixture dictionaries and legacy/closure modes; the lifecycle
corpus includes only the actual dictionaries and initialized cache.

Each child verifies a source-host lifetime marker is still rooted after acquisition, then
drops every retained Lua handle and verifies host destruction before native timing. The owned
carrier is also required to be Send + Sync under the current non-send mlua configuration.
The executable still links Lua for acquisition. Source histories explicitly use the pinned
interpreter after JIT disable/flush; no warmed-source equivalence is claimed.

Each original contributes 11 successful native public calls, two matching source-error
cases and six positive source parses that still fail native traversal. Thus PoB succeeds
17 times per original, while native succeeds 11 times. Seventy-three additional probe calls
and 35 checkpoints validate cache aliases, return-copy independence, miss/hit/eviction
histories and failure prefixes. Fresh private sessions preserve the original cache while
another session is mutated, and foreign handles reject. Raw output reimport is checked only
for selected plain graphs, not closure/class/iterator/traversal checkpoint restoration.

One fresh process per original produced these elapsed times in milliseconds:

| Original | Compile programs | Deep-clone input | First private import | Restart after drop | Drop first session |
| --- | ---: | ---: | ---: | ---: | ---: |
| 01 | 2.230 | 21.703 | 84.834 | 90.833 | 19.410 |
| 02 | 2.195 | 22.307 | 88.217 | 88.368 | 19.234 |
| 03 | 2.429 | 22.056 | 88.245 | 88.965 | 19.716 |
| 04 | 2.300 | 22.401 | 86.637 | 87.294 | 19.461 |
| 05 | 2.368 | 23.605 | 90.363 | 91.509 | 20.116 |

These are five distinct input observations, not repeated samples of one workload. The inputs
contain 44,080–44,173 tables and 1,697–1,705 closures. The initialized successful cache key is
selected from actual source data; all five selected the same key in this run. That is one
cache case, not five distinct interaction families. The retained transcript records the
actual scalar inputs, selected key index, operation/reference order and source expected-result,
checkpoint and error digests, with their encoding. It is not a serialization of the full session.

The shared compiled-library clone is separate from deep input cloning. There is no
`ProgramSession` clone/reset API: restart means fresh import and old-session teardown.
`import_session_input` appends state. These measurements establish that private import costs
more than sharing compiled definitions in this corpus; they do not establish candidate
throughput, an architecture winner or a need to reimport before every candidate. Argument
construction, oracle checks and report work are outside invocation timers but affect allocator
reuse. The 359 recorded phases per build include snapshot/drop work separately. Resource
usage fields are cumulative budget charges, not measured live allocations or RSS.

Reproduce using a fresh direct child of `runs`:

```powershell
$env:POE_A1_SESSION_OUTPUT = Join-Path (Get-Location) 'runs/session-lifecycle-new'
cargo test --release --locked -p poe-optimizer-pob --test profile_source_sessions -- --ignored --exact profile_original_source_session_lifecycles --nocapture --test-threads=1
```

The supervisor runs five sequential children with a 300-second limit each. Final evidence is
`runs/a1-session-measurement-02/` and `runs/a1-sessions-01/lifecycle-02/`, produced by executable
SHA-256 `433553c986455c9901d3e676beb1223dd69ae8ee3545a66e6418c11bb4778719`.
The first run passed semantic checks but omitted retained workload identifiers; its artifacts
remain in attempt 01. Only the final attempt is counted here. Two earlier lint findings were
limited to an unused extraction import and a constant release-mode assertion. The targeted
Clippy/release rebuild and final all-five replay pass. No additional full native build is admitted.

### Requested allocation layouts for private sessions

The lifecycle harness now wraps the test executable's Rust `System` allocator. Every measured
phase records successful/failed allocation calls, full requested/released traffic, signed
live-byte change and interval peak. Counting starts at process startup; dropping an allocation
from an earlier phase therefore produces a valid negative delta. Report strings and vector
growth occur after the interval closes. The same forwarding implementation passes an isolated
alloc/zeroed/grow/shrink/free self-check: 152 bytes requested and released, 96-byte peak, zero
final live storage, plus a separate pre-interval free with a -32-byte delta.

Five fresh original-build processes pass the existing histories after verified Lua-host
teardown. Across these distinct inputs, the median requested byte counts are:

| Phase | Requested traffic | Live-byte change | Peak growth above phase start |
| --- | ---: | ---: | ---: |
| Compile source programs | 3,167,532 | 1,920,700 | 1,942,148 |
| Deep-clone coherent input | 21,688,656 | 21,688,656 | 21,688,656 |
| First private import | 104,461,684 | 59,218,964 | 66,980,572 |
| Restart by fresh import | 104,461,684 | 59,218,964 | 66,980,572 |
| Drop final compiled library/definitions | 0 | -4,841,800 | 0 |

Compiled-library and session-handle alias cloning/dropping record zero allocator calls and
zero byte changes in all five runs. This distinguishes shared handles from private state;
it does not require fresh import before each candidate or establish parallel throughput.
The private-session representation and reuse strategy remain important A2 comparison inputs.
First initialized cache hits request 15,527,288–15,560,024 bytes, while the repeat hit requests
11,128 bytes in every run. Positive misses still ending in Unsupported request
12,254,890–12,936,745 bytes. These call intervals expose preparation/reuse costs in the captured
parser histories; unsupported misses do not represent successful evaluation throughput.

These are requested Rust layouts, not allocator usable sizes, Lua/C allocations, physical
memory or RSS. The counters are process-wide; no native workers run during these serial
measurements, and snapshots assume quiescent boundaries. Atomic accounting also changes the
elapsed timings. The retained scalar inputs, expected-result histories and phase sequences
match the prior timing corpus, but owner/program hashes and some late cumulative VM charges
differ between source acquisitions. This is not an instrumentation-only timing comparison.
Budget charges still have their separate meaning; they are not live storage.

Evidence is `runs/a1-session-allocations-01/` and `runs/a1-producer-01/lifecycle-01/`, executable
SHA-256 `8d7b3f94c31ee642b3b063913e1ddaf9c6b212c05a6f41f272d71a098c0c011e`.
The same ignored lifecycle command above reproduces the allocation protocol with a fresh
output directory. Outcomes remain 55 native public successes, 10 matched source errors and
30 positive source parses that are still native Unsupported. Full native original coverage
remains **0/5**. Per-stage loader/observation allocations and general candidate invalidation
are not measured by these private-session intervals.

### Transferring staged private import storage

The native importer now moves validated staged containers into empty destination arenas/maps,
while populated destinations retain the existing append behavior. Validation, identity offsets,
coverage/traversal metadata, class/closure associations and cumulative logical charges are
unchanged. This is a bounded optimization of the existing implementation, not a different
execution model or a copy-on-write state representation.

The same five original inputs and 359 lifecycle phases per input pass after the change.
All five recorded source histories, scalar inputs, phase-name sequences and initial logical
import usage match the prior requested-layout run. Freshly acquired owner/program/creation
identities differ, as do some late cumulative VM charges (final byte deltas within ±2,352 and
step deltas within ±294). The original input/reference/result/checkpoint histories and
non-cost native outcomes match. This is not a frozen-artifact timing comparison.

Median requested bytes across the five distinct inputs:

| First private import | Prior run | Staged-container transfer |
| --- | ---: | ---: |
| Allocation traffic | 104,461,684 | 85,196,548 |
| Released traffic | 45,242,720 | 25,977,584 |
| Retained live-byte increase | 59,218,964 | 59,218,964 |
| Peak growth above phase start | 66,980,572 | 59,219,468 |

The independently created session and fresh-import restart have the same medians. This is
about 18% less requested allocation traffic during these imports, with lower temporary peak
growth and unchanged retained storage. It does not establish faster candidate evaluation,
RSS savings or full-build throughput. The explicit input-clone exercise remains unchanged
at median 21,688,656 requested/retained bytes; ordinary session creation already borrows the
input and does not require that deep clone. Shared handles still avoid payload copies.

Evidence is `runs/r2w-tail-01/lifecycle-01/` and `runs/a1-session-import-move-01/`, compared
with `runs/a1-session-allocations-01/`. No build or other source test ran alongside the
measurement. Sixty-eight engine unit tests and 162 source-runtime integration tests pass,
including staged storage, alias/offset, equal-charge and failed-import regressions. These
checks supplement the original-source lifecycle comparisons; full native build parity is
still incomplete.

## Native allocation-origin checkpoint

An independent, opt-in native diagnostic now records each executed Table expression against
the actual allocated session table. The bounded record arena is charged/reserved when enabled;
records persist across invocations until disabled or restarted. Returned witnesses retain the
exact compiled library, its catalog/facets and the table handle. Missing evidence remains
`NotObserved`; foreign handles and non-tables reject. The diagnostic admits no new layout.

All 30 canonical positive-miss cases from the scanner regression now tie the failed
`cache[line][1][1]` table to `ModParser.lua:6966–6973`: the 259-byte modifier-record expression
with fields name/type/value/flags/keywordFlags and `unpack(tagList)`. Its function-relative range
is 11592..11851. The original function source span is 6619–7036, with function offsets
6..14115 inside that span. The test verifies both span/function hashes and exactly one
matching Table node in the retained catalog; it does not apply those offsets to the whole file.

Each actual allocation has no admitted constructor site. That is missing metadata, not proof
of a particular Lua opcode or a lost traversal observation. Diagnostic ordinals vary from
7 to 10 and remain session/window-local. The earlier cache/result graphs, failure paths,
native call frames and matched source copy identities remain unchanged. The legacy
`success_miss` field mirrors the first of six `success_misses`; it is not a seventh case.

Validation passes 92 focused ENGINE tests, including seven new origin/bounds/identity cases,
and all five scanner tests covering both observation modes on all five originals. Strict
workspace/native lint and portable library compilation pass. Evidence is
`runs/a1-sessions-01/{engine-validation/,scanner-01/,origin-summary.json}`. The scanner binary
SHA-256 is `9303504dff5861203484043d93281280ac3a5b81752fe26570db5836778c67a7`.

That checkpoint left the original producer/store and opcode/template unobserved. The next
checkpoint closes that identity gap. A native source-expression witness alone still cannot
establish original Lua allocation or traversal layout.

## Original constructor and producer checkpoint

An explicit pre-observation diagnostic ticket now retains the exact actual Lua Function.
The source observer binds that ticket to its owner/callback during normal graph observation;
a later same-name, same-span or equivalent function cannot substitute for it. A bounded query
maps the complete lexical constructor inventory to original instructions, including rejected
keyed sites. The optional template witness retains its actual host identity, original raw
key order and scalar/self-marker rows. Normal captures retain no extra diagnostic Lua handles.
The diagnostic is separate from shared constructor admission and grants no native layout.

All 30 positive-miss cases now bind the native expression to the original TDUP at PC 1111
(word 6038069, destination register 34, constant index -93). Its actual template contains five
reserved named keys: name, type, value, flags and keywordFlags, each with an exact self-marker.
The query occurs before the hooked parse and verifies unchanged identity/content afterward.
Raw template order varies across fresh hosts; every matched source copy in this corpus uses
that host's observed five-key sequence. This is observed behavior, not a general traversal rule.

The same scoped hook observes the exact inner parser's visible modList/i/name locals, retains
all source row identities, and joins them directly to later copy inputs. Original TSETV at
PC 1147 stores the constructor register through list/index registers 28/32, matching debug-local
slots 29/33. A separate conservative check authenticates the complete contiguous source-line
6974 region, PCs 1148–1158: no alternate entry bypasses allocation/store and no intervening
operation overwrites the constructor register. The hook reveals a line, not an exact PC;
retained row contents are serialized after the call, not at constructor completion.

There are 30 producer activations and 35 row-store observations: added-fire damage creates
two records per build. Only 30 records join the particular failed copy input. Synthetic cases
also preserve equal distinct records, repeated parser calls and later row replacements.
Combined event/depth/text limits, sticky errors, exact wrapper identity and scoped hook cleanup
remain enforced. Source execution is interpreted after jit.off/flush, with no warm-path claim.

The 127 source-program unit tests include five new diagnostic tests. Eight scanner tests
cover both observation modes on all five originals; six standalone copy-witness tests pass.
Workspace/native strict lint, portable libraries and formatting pass. The final scanner
report corrects a stale prior `allocation_origin_proven` flag; attempt 01 remains retained,
with final evidence under `runs/a1-producer-01/scanner-02/`. No dataset/schema/dependency or
supplied-build change is made, and full native original coverage remains **0/5**.

The next semantic dependency is reserved-key template traversal and mutation history. Before
admitting a generic family, prove the variable tail's actual result count and writes, define
invalidation for new/deleted keys and array growth, and compare this compatibility cost in
A1/A2. Final absence of numeric keys does not prove an empty TSETM tail or physical capacity.
No source order is supplied as a runtime fallback, and no physical table layout is admitted.
The [R2w follow-up](#original-parser-tail-call-checkpoint) below now establishes the actual empty tail for these observed records.

## Original parser tail-call checkpoint

The R2w source observer now authenticates the result-producing `unpack(tagList)` call for
all 35 generated modifier records in the 30 canonical positive-miss cases. Each record has
one original line-entry token, one exact primitive call and one later row store, joined by
actual constructor identity and activation. Added-fire damage retains both records. Every
observed pack has zero results; this comes from the actual caller table's raw length at the
call, not the absence of integer keys in a final row.

Two diagnostics retain the same original Function, template and catalog. The complete
line-6972 inventory is GGET 1143, MOV 1144, CALL 1145, TSETM 1146. The opt-in `global_name`
query reads the original GGET constant through privately retained reflection; it is `unpack`.
Before lookup, the line-entry guard checks the actual environment is plain and its raw
binding is the original primitive. This excludes a metamethod or loaded wrapper delegating
to that primitive with different arguments. The fresh token is consumed once at the actual
C call; leaving the region or unwinding abandons pending evidence. A post-return same-line
event cannot rearm it.

The bytecode binds the single argument from source register 24 (debug slot 25), through
register 37 with the pinned two-slot frame convention, and the allocated table in register
34 (slot 35). The existing constructor/store proof joins TSETV 1147 and the subsequent
line-6974 region. Inspection uses the Lua caller, avoiding C-frame slots that mlua may shift
while preparing hook-error storage. Full source result packs, cache aliases and native failure
prefixes still match the prior contract.

The returned pack is **derived** from authenticated original `unpack` semantics and captured
raw length/slots. It is not an intercepted return. Synthetic cases retain nil slots among
multiple results, and cover zero/one/multiple values, repeated activations, another function
with the same source, changed bindings, bounds, protected errors and a self-removing
metamethod. Zero-result TSETM performs no entry writes or array resize; a GC barrier is not
excluded. Positive-tail start indices, capacity, resizing and warm/JIT history are unproved.

Final validation passes 128 source-program unit tests, 11 standalone copy/producer/tail
witness tests, and 14 scanner tests covering ten original-build bootstraps. The final scanner
is `runs/r2w-tail-01/scanner-02/`; the passing earlier scan is retained but lacks the final
pre-GGET attribution guard. No layout is admitted and all 30 native positive returns still
stop at the same copy traversal dependency. Complete native original coverage remains **0/5**.

The next bounded candidate is an authenticated reserved-string-key traversal certificate,
with live values and explicit invalidation for positive tails or unreserved writes. It must
also preserve deleted-key controls, key rooting, aliases, resource errors and source/JIT
histories. This is a proposed compatibility contract, not a chosen A2 model or an implemented
native feature. Keep its implementation/validation cost in the execution-model comparison.

## Original modifier observations for the comparison

A bounded source-only test now imports each of the five unchanged complete XML builds into
pinned PoB, then observes selected item modifiers and controlled query histories. It reuses
the existing complete-build bootstrap. It implements no alternative evaluator or new data
schema, and does not resolve the pending choice of prototype boundary.

Windows and Linux (WSL Ubuntu 24.04) each pass all five child cases with 17 selected
cold-resistance line occurrences
(4/4/3/4/2 by build). These are source observations, not 17 distinct effect families. Each
saved item set has 22 explicit slots; PoB expands it to 94 runtime slots. The test verifies
all saved bindings and the empty defaults instead of equating the two counts. Build 5 keeps
the same ranged ring in both equipped slots, including their distinct source-slot attribution.
Its two range replays call the original formatter and parser; the test supplies no numeric
roll result. All other observed lines are explicitly labelled retained-line component replays,
not historical parser calls from the import.

The observer captures the private `getRangedModList` function from the original
`Item.BuildModList` upvalue and verifies that binding after the probes. An exact-function
call hook records its parser inputs; hooked and unhooked helper results agree. Original
`setSource`, `ModList.Sum` and `EvalMod` process copied fragments in isolated contexts.
A resolved false/tie/true condition produces 0/0/7. Updating a live per-stat dependency through
9/10/19/20/9 produces 0/7/7/14/0 on the same modifier record. These derived probes do not edit
the originals or rerun PoB's attribute-resolution stage. Stat names, conditions and amounts
are test inputs, not production build handlers.

Cache probes retain return arity and distinguish nil/no-match from a truthy empty modifier
list with a remainder. Positive, no-match and empty-result histories cover miss, hit, return
copy independence, return edits and eviction. Invalid nil/false parser calls are also observed.
The test checks source-function identities, saved/live selections, raw cache bindings and
touched scalar contents after restoration. It does not claim restoration of physical cache
layout, arbitrary nested graphs or allocator history. Each source host is discarded afterward.

Raw slot records and isolated sums are not final actor contributions: later assembly may
scale or transform them. This test establishes neither an alternative model's parity nor
full build equivalence, callback/general interaction coverage or throughput. Native coverage
remains **0/5**. Source observations can support either a domain boundary or the public-parser
comparison; choosing a model still requires A2 experiments and the A3 discussion.

Reproduce from the repository root, using a fresh absolute report directory:

```powershell
$env:POE_DOMAIN_MODIFIER_OUTPUT = Join-Path (Get-Location) 'runs/domain-modifiers-new'
cargo test --locked -p poe-optimizer-pob --test domain_modifier_oracle -- --exact original_domain_modifiers_are_observed_on_all_five_builds --nocapture
```

`POE_DOMAIN_MODIFIER_TIMEOUT_SECONDS` optionally changes the per-build limit (default 300,
allowed 1–1800 seconds). Each child runs sequentially in a fresh test process. These processes
isolate the PoB reference host; they are not part of native candidate evaluation. Reports bind
the exact source manifest, observer and original XML hashes. Local Windows evidence is
`runs/a2-domain-oracle/attempt-03/` with `windows-summary.json` in the parent directory.
Linux evidence is `runs/a2-domain-oracle/linux-01/` and `linux-result.json`; both platforms
use the same observer and driver bytes. The bounded `additional_observation` payloads match
exactly for all five platform pairs; this excludes the full bootstrap report and build outputs.
The comparison is retained in `cross-platform-summary.json`. The two earlier attempts preserve
harness failures: a mistaken global helper lookup and an
incorrect equality between saved and runtime slot counts. Neither required production changes.

## What this tells us, and what remains

The current general source/session machinery is not wired into full native configuration
activation. The existing repeated calculation path uses restricted typed numeric plans;
it still has separate candidate admission and document verification work. Therefore the
measurements above cannot be blamed on the interpreter or used to predict a replacement's
speed. They do show that loading/preparation, retained representations and external result
work belong in the comparison alongside calculation throughput.

A1 remains open for these measurements and decisions:

1. Extend the measured dataset allocation/ownership lifecycle with per-statement allocation
   attribution and temporary JSON lifetimes. Measure source observation/lowering and private
   session costs separately using the measured lifecycle boundaries above.
2. Extend the measured restricted admission/reuse boundary to representative interaction
   histories and complete builds as supported. Scoring, search quality, arbitrary text edits
   and general incremental invalidation still lack equivalent workload measurements.
3. Extend the measured cache-hit/no-match/miss/failure lifecycle and requested allocations
   with representative invalidation/reuse histories and source-observation attribution. An unsupported
   positive parse still has no successful native throughput to report.
4. Extend the completed balance/identity-repair and unchanged-pin extraction replays to a
   real upstream or effect-family update. Record code/data changes, migration/debugging effort
   and provenance quality; synthetic edits and code size cannot substitute for that work.
5. Select representative A2 slices with the owner using the
   [all-five interaction map](execution-model-semantics-inventory.md), including mechanisms
   those five do not cover. No model or migration has been selected.

## Reproduction and evidence

From the repository root, write reports to new paths:

```powershell
python scripts/inventory_execution_model.py --revision 9c756224f7c0063b71e277e4ef2ffe33a4c91d19 --output runs/a1-code-inventory.json
cargo build --release --no-default-features --locked -p poe-optimizer-cli
.\target\release\poe-optimizer.exe prepare-build tests/fixtures/builds/breadth-20260908/build-01.xml --output runs/a1-original-new.json
.\target\release\poe-optimizer.exe benchmark-native tests/fixtures/builds/spark-action-timing.xml --mode prepared --jobs 4 --evaluations 10000 --timeout-seconds 60 --output runs/a1-profile-new.json
```

Repeat originals 01–05 and profiles/modes/workers as specified above; use 1,000 evaluations
for document mode. Timers/metric checksums are in the CLI reports. OS memory measurements
require an external observer; the exact local driver is `runs/a1-baseline/measure.py`.
Its matrix commands, timestamps, memory-query validity, binary/input hashes and report/log
hashes are preserved in `runs/a1-baseline/measured/summary.json`. The independently checked
aggregates and stronger Spark checksum validation are in `runs/a1-baseline/aggregate.json`.
The inventory is `runs/a1-code-inventory.json`. Ignored run files are local evidence, not
files shipped by the repository; this document retains their protocol and results.

Provenance anchors:

- Release executable SHA-256: `f10d35e1c1fd6e38373f6e85ca4240719a1f70b8d7ca242c1b90551bfcafbd93`.
- Game package SHA-256: `8dfadca7567d7761cf8b01a9763bec8f2662abec45c500e3e271ee58edf8f9f6`.
- PoB pin: `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`; optional reference only in this matrix.
- Supplied import SHA-256: `3e763f109adb27d48f2cf63a8a95aaea649e5336dcaf37959931725c29f6c745`.

Protected inputs, the package, lock file and executable hashes were unchanged across the
matrix. These figures establish neither a PoB speedup nor a preferred execution model.
