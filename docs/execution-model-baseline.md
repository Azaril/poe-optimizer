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

That checkpoint left an authenticated reserved-string-key traversal certificate as the next
bounded candidate. The following checkpoint implements its closed domain, while retaining
positive tails and arbitrary mutation/layout histories as explicit unsupported boundaries.
Keep its implementation/validation cost in the execution-model comparison.

## Reserved-template native traversal checkpoint

An explicit source-acquisition API now admits the closed reserved-string-key constructor
family. It joins a live original Function/template witness to the exact lowered catalog,
callback, expression, bytecode instruction and supported source profile. The entire TDUP
template must contain distinct byte-string keys whose values are exact self markers, and
static constructor fields must match that set. Imported snapshots and serialized diagnostic
reports cannot supply this proof. Existing constructor and closure-creation facets survive
attachment to the new catalog; other uncovered sites remain explicit.

The DATA facet carries ordered byte keys, bounded and validated before retention. The native
ENGINE compiles immutable shared keys once and gives each certified private table a shared
reference in a sparse side map; ordinary imported tables get no such certificate. Traversal
reads live raw values in the reserved order and skips nils. Reserved deletion/reinsertion and
deleted reserved controls remain valid. A successful unreserved write, even an absent-key nil
write, or behavior installation invalidates the certificate. Failed mutations preserve prior
state. Work and metadata storage are charged; no capacity or raw-length rule is inferred.

All RHS packs and effects execute before checking the final constructor tail. Zero results
are admitted; any positive count, including all-nil packs, returns `UnsupportedCapability`
before guessing array writes or returning a table. Source errors retain their own error class
and already-completed effects. Source oracle runs retain the original Function/template roots
through GC; native libraries retain only key bytes. The contract does not claim source
instances whose template roots have died, arbitrary hash histories or positive-tail layouts.
See the [session contract](parser-sessions.md#reserved-template-keys) for the exact domain.

The final five-original scanner creates a second, explicitly admitted native session from each
original coherent input graph. One actual source constructor witness attaches the facet to its
exact original extraction; each later witness must retain the same Function/template, site,
bytecode and ordered rows. All six canonical positive families complete both misses and hits
on each graph: **30 complete misses and 30 complete hits**. Joint comparisons cover entire
return packs, fresh actual source/native cache lookups, miss/hit row aliases and independent
returned copies. The existing unadmitted failure oracle and its 35 empty-tail records remain
separate controls; the legacy duplicate `success_miss` report is not counted again. No source
results are injected into the native session. The fixed strings belong to oracle fixtures,
not the production build path.

Four focused source/native tests cover witness/catalog tampering, binary/static keys, live
aliases, GC, deletion/reinsertion, invalidation, tail cardinality and source-error effects.
Warm tests execute two standalone and four seeded vectors, each using 128 calls per phase:
**1,280 source calls** (768 final-vector calls plus 512 seed calls). Six final complete packs
are compared, with the exact original target required in a completed live trace. This does
not assert that every intermediate result is compared or every changed branch stays compiled;
changed vectors may exit or materialize source state.

Validation passes 28 DATA constructor tests, 75 ENGINE unit tests, 162 ENGINE integration
tests, 129 PoB source-program units, four focused source/native tests, 11 copy-witness tests,
four existing native-constructor tests and 14 scanner tests covering ten original bootstraps.
A final work-charge refinement additionally reruns its seven affected ENGINE tests. Strict
workspace/native-only Clippy, formatting, whitespace and five portable WASM library checks
pass. These are scoped commands, not a claim that the entire workspace test suite ran.

Final evidence is `runs/r2w-reserved-01/`: `source-tests-01/`, `pob-build-03/`,
`final-validation-01/` and `scanner-01/run.json`. The final scanner takes 132.738 seconds and
binds 574 unchanged source/input files to executable SHA-256
`55c1e969881ea21821dcb1ba6853c2e397148cc09104021ed5ecb4bce351532c`.
That elapsed time includes source bootstraps, diagnostics and validation; it is not a candidate
throughput, allocation or speedup measurement. Supplied inputs, schema 29 bundled data,
dependencies and pinned PoB are unchanged.

Complete native original-build coverage remains **0/5**. This closes one parser dependency
within the current model; it does not complete the public parser, actor/action preparation,
full build parity or A1/A2. The source authentication, metadata, native state and validation
needed for this family are concrete compatibility costs to compare with a domain DSL, native
algorithms, retained interpreter and hybrid alternatives. Broader tagged/conditional cases
with positive tails are a next dependency, not a reason to select a model without discussion.

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

## Actual-import parser corpus checkpoint

R2x replaces selected-family-only input selection for this component with actual parser calls
from all five complete original imports. The [protocol and results](parser-input-corpus.md)
record 1,375 attempts, all 116 saved items, and 760 build-local distinct parameter pairs:
558 match complete supported miss/hit graphs and 202 retain explicit unsupported first stops.
All observed source imports preserve control scalar outputs; no original build becomes fully
native. The reference host remains optional tooling, and no production evaluator/data/schema
changes are made in this checkpoint.

Count the observer, coherent-state guard and bounded graph/replay machinery as compatibility
cost in A1: six new test/support files contain 2,477 physical lines, with 51 net shared-hook lines.
This count includes fixtures/comments and does not estimate maintenance effort. The complete
single Windows release validation took 336.8 seconds, including reference import/capture,
per-probe state checks, fresh native session preparation and report serialization; it is not a
hot-path throughput benchmark. Separate these costs from candidate evaluation when comparing A2.

The main reached parser categories are unavailable traversal order, unavailable source entries,
and positive constructor tails. Their downstream observable requirements must be examined in
complete item assembly and registration before choosing broader interpreter work or another
representation. The next production seam is the existing ItemLoadProvider/prepare_view integration,
with complete owned item outputs, rather than treating diagnostic assembly evidence as a ready
item or adding another skill profile. A1/A2 remain open and complete native original coverage is
still **0/5**.

## Native accessory assembly checkpoint

R2y adds a finite native algorithm behind the existing injected assembly policy, an owned
assembly graph and source-ordered inventory integration. It is bounded work within the current
seams, not an A2 comparative prototype or an approved migration. The
[assembly checkpoint](native-item-assembly.md#r2y-implementation-checkpoint) records 32 accessory
matches using the original parser and 31 using the native parser, from all 116 original items;
full native build coverage remains 0/5.

New files contain 3,181 physical lines in the assembly module (including its unit tests), 455
in native inventory preparation and 2,466 in eight test/support files. Changed shared provider,
loader, preparation and observation code is additional cost. Counts include comments/fixtures
and do not estimate maintenance effort. The final code inventory and command/source hashes are
in `runs/r2y-item-assembly-01/code-footprint-final.json` and its validation receipts.

The eight-test source target took 41.36 seconds on one Windows release run, including ten fresh
reference hosts, original/control graph checks, native replay and directed histories. It is not
a throughput benchmark or evidence of a native speedup. The control imports agree on their
finite observable contract; callback-bearing graphs, actual dependency arity/order, specialized
item locals and wider state histories remain separate gates. Count reference harness complexity
as well as shipping representation costs when comparing models, and distinguish both from hot
candidate evaluation. The A1-to-A2 readiness review need not wait for every missing item family.

## Native local-item assembly checkpoint

R2z extends the existing injected policy/native algorithm seam to armour, flasks and charms.
It is not an A2 model comparison. Its schema-30 package preserves all 28 other sections and
the previous assembly policy; only the three new policy families and version/hash metadata
change. The data identity is recorded in the [performance inventory](execution-model-performance-inventory.md).

The five new modules add 780 physical lines of policy/extraction, 357 lines of native
algorithms and 1,102 lines of focused/reference tests. Existing adapter, loader and inventory
integration edits are additional. Counts include comments and fixtures and are not estimates
of runtime or maintenance cost. `runs/r2z-item-local-01/new-module-costs.json` records this
scope. The [implementation record](implementation.md) retains validation and remaining gates.

This extension also exposed an adapter requirement: a numeric diagnostic projection cannot
replace an owned graph containing nested override values, and pending header writes must
survive a no-base reparse. Those are observable consumer requirements for any A2 alternative.
Their current graph/patch implementation is one representation to compare, not a requirement
to reproduce its internal machinery. Source-host and differential-fixture complexity belong
in the comparison alongside production runtime code. A1-to-A2 readiness does not require
finishing weapon/jewel locals or the complete native evaluator.

The eight-test R2z source target passes in 43.86 seconds on one Windows release run,
including ten reference hosts and the extended directed histories. Of 91 eligible original
items, 87 match with an explicit original-parser dependency and 85 with the native parser;
25 weapon/jewel records remain excluded in this lane. See the [scope and frontiers](native-item-assembly.md#r2z-local-item-checkpoint).
These component results and validation time establish neither full-build parity nor candidate
throughput, and select no execution model.

## Native weapon-local assembly checkpoint

R2aa extends the same injected item policy and native assembly producer to weapons. It is
component progress within the existing model, not an A2 comparison or a migration decision.
The schema-31 / item-assembly-schema-3 package is 26,298,910 bytes, SHA-256
`01f484bc3f30ca595735c7c4b2fc29b4c2e7682fee2863f4ab55c4c0059e365f`.
All 28 other sections and the previous assembly policy are unchanged. Historical measurements
above keep their own earlier package and executable identities.

The [weapon checkpoint](native-item-assembly.md#r2aa-weapon-local-checkpoint) retains all
116 saved items: 98 are eligible for the implemented families, with 93 original-parser plus
native-assembly matches and 91 built-in-native-parser plus assembly matches. Weapons contribute
six of seven matches in both lanes. Build 03 Item 17 stops at ambiguous rune reconstruction;
18 jewels remain excluded from this family lane, not counted as executed assembly failures.
The other stops remain rune reconstruction, advanced-copy affix ordering and native parser
callbacks. These component counts leave full native original evaluation at **0/5**.

The consumer contract now includes whole per-slot `weaponData`, its publication before later
queries can fail, and aliases retained through generic late overrides. Overrides precede
residual hand-condition tag changes and the final ordered DPS reset/sum. A substitute model
must preserve those observable outputs and reached failure prefixes; matching a final damage
number alone would not establish equivalent item preparation. It need not reproduce the
current arena implementation. Dependency-call arity/order and arbitrary callback behavior
remain separate gates.

| New module role | Physical lines |
| --- | ---: |
| Typed weapon policy and validation | 271 |
| Source extraction/authentication | 530 |
| Native weapon algorithm | 287 |
| Focused native tests | 524 |
| Original-source fixture/history support | 655 |

Existing module, provider, inventory and test wiring edits are additional. These counts include
comments and fixtures; they are neither runtime costs nor maintenance-effort estimates.
The eight-test source target passed in 45.02 seconds on one Windows release run with ten fresh
reference hosts, excluding 37.66 seconds of compilation. This includes import, control/graph
comparisons and directed histories; it is validation cost, not candidate throughput or a
speedup comparison with earlier targets. Broader checkpoint validation was still in progress
when this component result was recorded.

Retained evidence is `runs/r2aa-weapon-local-01/source-parity-counts.json`,
`remaining-item-stops.json`, `new-module-costs-final.json`, `package-reconciliation.json` and
`source-01/`. The implementation record owns subsequent validation and production status.


## Native jewel and radius checkpoint

R2ab continues the existing injected-policy/native-algorithm seam. It adds finite local jewel
production, an immutable radius context, ordered header/post-ParseRaw behavior and startup-context
integration. The implementation record owns validation status; no alternative execution model is
selected and complete original native builds remain 0/5.

The additional standalone modules make the compatibility cost concrete. The measured files contain 189 lines of data policy, 793 lines of extraction, 789 lines in native
producer/context/continuation modules, 1,228 lines in separate native test files and 1,773 lines in
source-history/lifecycle test files. These are physical lines, including embedded tests/comments;
they are not disjoint production-versus-test totals and exclude edits to existing integration
files. Exact paths/counts are retained in `runs/r2ab-jewel-local-01/module-line-counts-final.json`.
The generated package grows by 3,894 bytes. The original Item data, prior assembly policies and
26 other sections remain unchanged; unique-requirement evidence updates only its item-loading
dependency digest.

The consumer requirements exposed here are lifetime and order: import uses the startup radius
context before saved tree selection; a later tree switch does not replay item headers; an unknown
base skips assembly but still reaches the radius tail; and arbitrary finite override values can
retain aliases across reparses. No candidate model must use the present table arena or loader
classes, but an alternative must preserve behavior that these later consumers observe.
Shared ParseRaw hydration is reused for the NoBase continuation instead of duplicating it.

Include local producer work, context preparation, graph adaptation, source authentication,
observer/control tooling and failure diagnosis in A1/A2 comparisons. Test/extraction elapsed
times are validation costs, not native candidate throughput. Spatial application, opaque callbacks,
cluster topology and actor integration remain separate gaps; this checkpoint does not establish
whole-build performance or a reason to retain the interpreter.


The same checkpoint exposed a Windows debug startup cost. The current CLI's `--help` succeeded,
but `metrics --backend native` overflowed its 1 MiB main-stack reserve before backend creation
completed. A Windows/MSVC executable reserve of 8 MiB admits both commands. PE headers and binary
hashes are retained in `windows-stack-before.json` and `windows-stack-after.json` under the R2ab
evidence directory. Static frame inspection shows large generated Clap and typed-deserializer
frames; it does not identify the exact crashing function. The stack-reserve fix is a host
configuration change, not an execution-model selection or proof that all resource bounds are
optimal. Include peak stack as well as heap in A2 preparation comparisons and eventual actual
WASM runtime tests; portable compilation alone does not establish those runtime limits.


## Native slot-validity checkpoint

R2ac adds a complete equipment-validity predicate at the existing injected-data/native
algorithm seam. It is a comparison input for A1, not an A2 alternative-model prototype.
The [equipment integration design](native-equipment-integration.md) distinguishes prepared
inventory, saved assignments, live slot state and actor-effective equipment. Completing the
predicate does not complete ordered activation, actor preparation or a whole native build.

The generated package grows by **2,146 bytes** to schema 33 / item-assembly 5; item-loading 5
and all other data sections are unchanged. Acquisition authenticates the complete source
method before extracting its operands. Even a literal-only upstream edit requires review of
that body pin; separately injected custom definitions do not. Count this review, extraction,
regeneration and differential validation in the update-cost comparison. A small package
change alone does not show that a model is inexpensive to maintain.
The policy parameterizes this fixed algorithm family; a new branch or interaction family can
require both schema and Rust changes. Compare that separately from editing existing operands.

The immutable native program validates its policy and compiles seven patterns once. Queries
borrow their input owners; cloned programs share compiled storage and keep query budgets and
context private. The prepared-inventory adapter supplies its own registered lookup winners,
while tree, active-set and actor-flag dependencies stay explicit. Unknown context cannot become
an ordinary invalid-slot result. These are current implementation choices, not required
representations for an alternative model. No throughput or allocation-free claim follows
from this reuse design or a passing concurrency test.

Required consumer behavior includes lazy context lookup, main-hand/off-hand relationships,
node inheritance, conditional flags and source errors. Raw return arity and table ownership
are checked at the component boundary; actual equipment consumers use truthiness. A2 may
propose a simpler boundary if it identifies the consumers and proves their behavior. In
particular, separate source-harness projection/authentication machinery from production
requirements instead of requiring every alternative to reproduce the same tooling.

Evidence belongs under `runs/r2ac-slot-validity-01/`. The complexity inventory separates
production policy/acquisition/runtime, prepared adapters, test/reference tooling and existing
wiring changes. Physical line and byte counts are reproducible footprint measures, not a
measurement of semantic complexity. Final validation and the current resume point belong in
[the implementation record](implementation.md). Full native original evaluations remain **0/5**.

The final new-module inventory contains **1,346 production lines / 50,921 bytes** and
**2,643 test/reference lines / 91,674 bytes**, with blank lines and comments included.
The existing CLI fixture repair and shared-file wiring are separate deltas. The final
source comparison runs 48,138 corpus queries plus 860 directed cases in 28.03 seconds,
excluding compilation; this is validation cost, not native evaluation throughput.
`complexity-inventory-final.json` records the final module footprint, including the
two-line test-only lint annotation. No
production code or test behavior changed after the passing source comparison.
## Original item-set loading lifecycle checkpoint

R2ad adds a reference-only observation of the complete original `ItemsTab:Load` boundary,
including activation, slot population, loadout synchronization, export-selection refresh
and the final undo reset. It extends the A1 consumer evidence without adding production
interpreter features, selecting an A2 model or completing native equipment activation.
The [consumer audit](native-equipment-integration.md#consumer-boundary-for-the-execution-model-investigation)
separates calculation/export effects from GUI history and observer-only state.

All five unchanged originals run in three fresh hosts each: one unhooked control and two
hosts with bounded call/return observation and JIT disabled. The original methods and
iterators remain unchanged; the harness verifies retained bindings and source provenance.
Ten complete original Load returns contain 40 before/after Load/activation snapshots and
26,350 reached validity calls. Every observed validity call has a present `calcsTab` and
absent `mainEnv`. Final post-import flags cannot replace that startup context. No loadout
activation callback is reached inside these ten Load calls; the source audit and mechanics
fixture establish why prior state and later histories must still account for re-entry.

| Measurement | Result and limit |
| --- | --- |
| Exact declared finite post-import graph against control | 5/10 equal. Raw arrays and aliases are retained even when the comparison differs; the projection is not the entire original object graph. |
| Separate selected-field comparison | 10/10 equal. It compares fixed state, selected slot fields, item ID/label choices, child names/inactivity and rune selected names/name-occurrence counts. It excludes slot/rune aliases, dropdown order/indices and rune effect data. |
| Observed slot traversal order between paired hosts | 0/5 pairs equal. Matching final selected fields does not prove that arbitrary traversal orders commute for later builds or failure/history cases. |
| Focused test target | 9 tests pass in 54.78 seconds, excluding 1.74 seconds compilation. This is reference-validation cost, not candidate throughput. |
| New reference code | Three files, 1,061 physical lines / 51,071 bytes, including comments and blank lines. Existing shared graph/bootstrap helpers are reused and excluded from this incremental footprint. No production code or package data changes. |

The first run stopped on a rune-list permutation. Its retained evidence identifies two
different-effect runes with tied sort keys; the next run keeps exact mismatch results and
adds the explicitly narrower name/count comparison. A mechanics test checks that this
comparison rejects changed selections and duplicate counts. This is a correction to the
observation contract, not proof that ordering or rune effects are irrelevant. The source
audits, failed attempt, final receipts and code inventory are retained under
`runs/r2ad-item-set-lifecycle-01/`.

For A2, compare a domain transition representation against the behavior actually required
by calculation, save and export consumers. Do not turn source observations into runtime
services required by the native evaluator. Legacy/repeated loads, duplicate IDs, unknown
rune names, loadout re-entry, order-sensitive errors and native-produced state remain
integration work. Full native original evaluations remain **0/5**.

## Native item-set materialization checkpoint

R2ae adds a fixed constructor/ordered-loading state machine at the existing injected-data
and native-item seams. It does not expand the generic source interpreter. The
[equipment integration contract](native-equipment-integration.md) separates saved rows,
previous active references, live selections and the still-unimplemented activation consumers.
This is another current-model comparison input; it does not choose an A2 representation.

The package grows by **9,875 bytes** to **26,314,825 bytes** (schema 34 / item-assembly 6).
Only the manifest and item-assembly section change. Slot relationships, defaults, 19 complete
passive socket IDs and 87 ordered trade-stat rows come from authenticated source. Full-tree
version/digest binding prevents using the projection with a different package tree; startup
version binding also keeps constructor slots consistent with radius preparation. Resolved
radius fallback data may still come from an older supported version.

The new-file inventory records physical lines, including comments and blanks; existing-file
integration changes and reused code are excluded. These are footprint measurements, not
maintainability scores or a complete tally of compatibility costs.

| New-file role | Files | Physical lines | Bytes |
| --- | ---: | ---: | ---: |
| Production module locations | 5 | 1,562 | 58,414 |
| Acquisition module (includes colocated tests) | 1 | 584 | 21,610 |
| Dedicated native/data unit tests | 2 | 712 | 23,424 |
| Reference-test files | 2 | 1,040 | 42,629 |

Sixteen fresh reference hosts cover all five originals plus eleven derived cases. All sixteen
fresh constructor comparisons and declared materialization comparisons match; six cases retain
expected original Source failures. The successful comparison ends at the first original
SetActiveItemSet entry and checks its parent Load and exact requested-set value. The graph
projection retains set/child aliases and duplicate order but compares rune selected names only;
raw graphs remain in the receipts. Error snapshots omit the unavailable prior argument, and
constructor snapshots omit uninitialized source trade storage. Exact trade-function identity,
activation, slot population, loadout callbacks and complete inventories are outside this component
comparison. The production coordinator separately preserves earlier native item failures.

These source results are correctness evidence, not candidate throughput measurements. General
candidate invalidation and effective equipment/actor participation remain required; complete
native original builds remain **0/5**. Compare alternatives against the actual downstream
consumer effects, rather than requiring them to retain this particular graph representation.

Receipts: `runs/r2ae-item-set-loading-01/package-review.json`, `new-code-inventory-final.json`,
`source-reconciliation.json`, and the raw `source-01/` observations. Broader validation and
publication state belong in the [implementation record](implementation.md).

## Strict rune-order checkpoint

R2af removes an over-broad rejection at the existing native algorithm seam. Distinct finite
vectors force the original comparator's candidate order even when multiple minimum-count
combinations exist. The native engine retains the original first DFS result and its ambiguity
bit; exact padding comparisons and search tolerance remain separate. A pre-add integer bound
also closes a grouping-order flaw that a rounded post-add comparison could miss. Definitions
and the schema-34 package remain unchanged; no generic interpreter capability is added.

The existing whole-item comparison gains four original items: built-in native assembly rises
from 107 to 111 of 116, and the lane with an explicit original-parser dependency rises from
109 to 113. All prior completed graph hashes and remaining frontiers are unchanged. The
ordered public preparation path advances from 68 to 92 registrations because later records
become reachable. Complete native build evaluations remain **0/5**.

The new scoped witness observes retained original ParseRaw/UpdateRunes functions across eight
cases, including explicit and header-free real item texts and custom insertion orders. It
checks the actual first DFS result and saved rune names, declared loading projections and
parser parameter order. The source's numeric candidate array also retains named aliases;
the witness validates those aliases separately. This is bounded component evidence, not
actual parser-call arity, universal observer noninterference or full object-graph parity.

Validation exposed a useful model boundary: ItemState retains parsed socket groups alongside
assembly projections, whereas the owned arena has the final groups rewritten by assembly.
The test now compares each against its actual source stage, retaining raw snapshots and
checking owned socket contents and row aliases independently. The [assembly contract](native-item-assembly.md)
records why that transport cannot stand in for a complete post-assembly object. Include the
cost of clarifying or separating these diagnostic stages in A2 alternatives.

| Changed-file role | Files | Net physical lines | Net bytes |
| --- | ---: | ---: | ---: |
| Production | 2 | 57 | 2,466 |
| Native regression tests | 3 | 371 | 12,949 |
| Reference tests and observation | 2 | 1,035 | 38,760 |

These are net changes relative to R2ae, including comments and blanks with normalized LF
endings, rather than complete layer sizes or maintainability scores. The observer reuses
existing source bootstrap and state comparison code. No candidate throughput claim follows
from test timings. Receipts are under `runs/r2af-rune-order-01/`: `code-footprint.json`,
`assembly-reconciliation.json`, `public-reconciliation.json` and the raw source observations.
