# Execution-model baseline

A1 is **in progress**. This checkpoint records the implementation footprint, consumer
boundaries and fresh measurements at `9c756224f7c0063b71e277e4ef2ffe33a4c91d19`
(2026-09-12). It does not choose a replacement or establish full native build parity:
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
These aggregate costs do not identify which loader, validation or compilation step dominates.

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

## What this tells us, and what remains

The current general source/session machinery is not wired into full native configuration
activation. The existing repeated calculation path uses restricted typed numeric plans;
it still has separate candidate admission and document verification work. Therefore the
measurements above cannot be blamed on the interpreter or used to predict a replacement's
speed. They do show that loading/preparation, retained representations and external result
work belong in the comparison alongside calculation throughput.

A1 remains open for these measurements and decisions:

1. Attribute load/validation/compilation time, allocation traffic and peak/retained memory
   to actual owners. Separately measure source observation/lowering and session costs.
2. Extend the measured restricted admission/reuse boundary to representative interaction
   histories and complete builds as supported. Scoring, search quality, arbitrary text edits
   and general incremental invalidation still lack equivalent workload measurements.
3. Measure cache-hit/no-match/miss/failure histories, reset/import/export and diagnostics
   at an equivalent supported boundary. An unsupported successful parse has no successful
   native throughput to report.
4. Record an upstream or effect-family update exercise, data-only versus code changes,
   migration/debugging effort and provenance quality. Code size cannot substitute for it.
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
