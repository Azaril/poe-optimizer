# Execution-model performance evidence inventory

This is A1's read-only inventory at `9c756224f7c0063b71e277e4ef2ffe33a4c91d19`
(2026-09-12). It identifies reusable measurement paths and gaps; it selects no replacement
execution model. The [investigation](rule-execution-model-investigation.md) defines the
decision gates and the [implementation record](implementation.md) owns the current status.

**None of the five supplied originals completes native evaluation.** Their successful PoB
runs and native preparation failures are breadth evidence, not native build throughput.
Existing native timings concern restricted Spark/Mace profiles. A small numerical kernel,
a prepared profile, a public parser call and a complete optimizer run are different workloads.

The read-only identity/measurement inventory is retained in
`runs/a1-performance-inventory.json`. No benchmark, test, compilation or extractor was run
to produce this document. Fresh A1 measurements must retain their own command, binary, input
and dataset identities rather than reuse the historical rates below.
The [fresh A1 baseline](execution-model-baseline.md) records the separately executed
preparation and fixed-profile measurements; its scope and checksum qualifications apply.

## What can be measured with the current paths

| Stage | Existing entry point | What the result establishes | Measurement gap or limit |
| --- | --- | --- | --- |
| Source acquisition | `extract-game-data`; [host publisher](../src/game_data_extract.rs) and supervised PoB extraction worker | Authenticated source extraction, canonical package/evidence and selected data identity | The report has no phase timings. An external timer combines verification, process startup, source loading, extraction, validation and publication. Separate these before comparing acquisition models. |
| Package loading and compilation | `benchmark-native` reports `initialization.backend_and_data_ms`; `evaluate` also reports initialization | Elapsed construction of the selected native backend and data | Loading, validation, indexes and compilation are aggregated. This is process-local initialization, not a cold-disk measurement. |
| General imported-build preparation | `prepare-build INPUT` | Source/view/data-owned preparation outcome and exact remaining stages; `calculation: not_run` | The command has no internal timing fields. External process time includes loading and report serialization. An incomplete result must not become a completed-build timing. |
| Restricted document evaluation | `benchmark-native INPUT --mode document` | Repeated source parsing/validation, supported calculation, complete typed result, diagnostics and XML export | Also includes Rayon scheduling, accounting and checksum work. Coverage remains the selected native profile. |
| Restricted prepared evaluation | Same command with `--mode prepared` | Same complete result work, reusing one validated input | It is not the pure calculation rate. Preparation and local pool creation are outside iteration timing. |
| Candidate preparation and hot calculations | [mixed-candidate harness](../examples/benchmark_mace_candidates.rs) | Separate dataset/catalog, handles, XML materialization/preparation and equivalence setup; document, prepared-result, pure calculation, stack snapshot and owned-measurement modes | Uses explicit diagnostic Mace axes; it is not a general-build preparer or realistic search. Short fast-mode samples need careful interpretation. |
| Fresh candidate admission versus reuse | [assembly harness](../examples/benchmark_assembly.rs) | `fresh_admission_and_measure` versus `already_admitted_measure`, setup costs, rotating checksums and worker scaling | The second mode already has prepared actors. Neither mode includes source XML, parser sessions, proposals, scoring, reports, deadlines or exports. |
| Whole restricted search | `search-build --native-evaluation typed` or `document` | Candidate generation/admission, scoring, archive/ledger, fresh finalist and resulting export for an admitted diagnostic problem | Historical 1,000-attempt runs establish bounded reproducibility, not 5–30 minute optimization quality or full-original coverage. Report timing and external process timing have different endpoints. |
| Original source/parser investigation | Full-runtime source tests and `runs/r2p-parser-factories/build-01..05.json` | Reached original behavior, native failures and their state/identity witnesses | Whole-worker elapsed time mixes bootstrap, import, probes and assertions. There is no isolated acquisition, lowering, parser-hit/miss, state-copy or witness-overhead benchmark. |

The source interpreter and direct native calculation paths are both implemented in Rust.
Comparing them requires equivalent behavior, inputs and lifecycle; their implementation
language alone does not make the old kernel numbers a prediction for the source interpreter.
See [shared source programs](shared-source-programs.md) and [parser sessions](parser-sessions.md).

## Replayable commands

These commands describe existing interfaces, not measurements performed for this inventory.
Build release executables outside a timed window, then time the executable itself. Use fresh
output paths; publication commands reject existing files. Preserve full reports, errors,
deadlines and worker counts. The documented sample counts are replay parameters, not targets.

```powershell
# Restricted full typed API; repeat with document mode and the same worker/count matrix.
cargo run --release --no-default-features --locked -- benchmark-native tests/fixtures/builds/mace-action-timing.xml --mode prepared --jobs 1 --evaluations 10000 --timeout-seconds 60 --output runs/a1-unique-prepared.json

# Independent admission and already-admitted measurement layers.
cargo run --release --no-default-features --locked --example benchmark_assembly -- --problem examples/action-timing-search.json --sample-ms 700 --repeats 3 --jobs 1,2,4,32

# Older fixed diagnostic axes, with explicit API modes.
cargo run --release --no-default-features --locked --example benchmark_mace_candidates -- --candidate-set actor-resources --evaluations 20000 --repeats 3 --jobs 1,2,4 --modes document,prepared_result,pure_calculation,typed_snapshot,typed_owned_measurements

# Whole restricted search; use a new output directory per replay if exporting artifacts.
cargo run --release --no-default-features --locked -- search-build --problem examples/action-timing-search.json --native-evaluation typed --jobs 4 --max-evaluations 1000
```

Use a single decoded original or share code with `prepare-build INPUT --output NEW_REPORT`;
the multiline `example.import.txt` is a corpus, not one build. The five source hashes in the
reviewed breadth expectations identify the inputs independently of their local file paths.
Keep PoB acquisition/reference commands in a build with the optional `pob` feature; native
candidate evaluation uses `--no-default-features` and requires no PoB runtime.

For `benchmark-native`, initialization is a subset of total preparation; do not add the two.
Iteration timers end after workers join and exclude final report aggregation/serialization.
The checksum is absent when any completed result contains unavailable or nonfinite requested
metrics. The Mace default catalog includes an unavailable average-hit metric. A null checksum
does not mean failed calculation, but it cannot support a per-call finite-bit identity claim.
Comparing a reported sample across runs is weaker than observing every result.

## Historical measurements and whether their identities still apply

The recorded A1 checkpoint package is schema **29**, **26,286,752 bytes**, SHA-256
`8dfadca7567d7761cf8b01a9763bec8f2662abec45c500e3e271ee58edf8f9f6`.
The PoB revision remains `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.
An unchanged upstream revision does not imply an unchanged extracted model, compiler,
native backend or benchmark executable.

R2z advances the package to schema **30**, **26,294,538 bytes**, SHA-256
`0c36d1c8dc7b42f829715a36724452fde5b29622ce3e75e165eff36c0957168f`.
This changes item assembly definitions; it does not rebase any measurement below onto the
new data or executable identity.

| Existing evidence | Actual recorded measurement | Applicability at this checkpoint |
| --- | --- | --- |
| `runs/action-speed-benchmark-{release,summary}.json` | Windows x64, 1,806 admitted diagnostic selections, three samples per 1/2/4/32 workers. One-worker medians: **38,868 fresh admissions + measurements/s**, **4.81 million already-admitted measurements/s**. Samples were about 661–752 ms. | Historical schema **11**, package `b7943c63…`, not the recorded schema-29 checkpoint. The benchmark source, problem and Mace template hashes still match; that does not preserve the old backend/data identity. |
| Same action-timing evidence | Dataset/backend setup **289.81 ms**; candidate admission **60.94 ms for 1,806 attempts**; prepared-component setup **1.75 ms**. Setup also records input loading, proposals, catalog and document-equivalence checks separately. | A useful example of separating costs. These are measurements of that older package/process, not estimates for current preparation. No peak-memory measurement accompanies them. |
| `runs/action-speed-release-search/summary.json`, summarized in `runs/action-speed-benchmark-summary.json` | Three repeats of typed/document search at 1/2/4/32 workers; **1,000 total attempts** each. One-worker report medians: **376.51 ms typed**, **3,260.87 ms document**; corresponding external process medians **408.59 / 3,277.62 ms**. | Same old schema-11 diagnostic domain. Current search source differs from its recorded implementation hash. This does not establish full-original search speed or useful build quality. |
| `runs/actor-benchmark-isolated-summary.json` | Schema-6, 3,675 legal Mace candidates; one-worker medians **4,485 document calls/s** and **20,221 prepared-result calls/s**, three 20,000-call samples. | Earlier profile/data/backend. The contrast includes result/document work and must not be compared directly with a pure kernel rate. |
| `runs/actor-benchmark-fast-isolated-summary.json` | Same historical corpus; five 1,000,000-call samples. One-worker medians **15.16 million pure calculations/s**, **5.63 million calculations with stack snapshots/s**, **4.32 million calculations with owned measurements/s**. | These latter modes include fresh calculation, not isolated conversion. Useful layer separation, not a current throughput claim. At 32 workers the fastest samples are only a few milliseconds; rates are especially sensitive to scheduler/timer effects. |
| `runs/assembly-benchmark-release.json` | An earlier schema-7 assembly/admission corpus with setup, distributions and checksums. | Both dataset and benchmark-source hashes differ from current files. Rebuild and recapture to compare; do not combine its rates with later runs as one experiment. |
| `runs/native-class-throughput/` and `runs/native-pipelines-throughput/` | Early fixed-input prepared/document API distributions and machine reports. | Predate current explicit data identity and broad preparation changes. Preserved historical evidence only. |

The action-timing benchmark source SHA is
`9a411a62e81728682ac7dd67f0838c0958685d247da811ef906f73a44b6c510d`;
its historical binary SHA is `b592482ecb060cd1154361f0f87224374b5e3a98223d1581bf3654b5b8ef1c36`.
The inventory records full input, executable, source and data hashes, including current
comparison outcomes. None of these historical executables is asserted to be the new A1 binary.

## State, workers, memory and diagnostic overhead

R2t's five closure-aware source-worker reports contain elapsed observations of
**12,847–13,850 ms per worker**. Each includes original-runtime loading and a large differential
test workload. They are not parser-call rates, source snapshot rates, native candidate rates
or controlled worker-scaling samples. Repeating correctness suites under a debug profile
does not turn their elapsed fields into an evaluator benchmark.

No isolated timing distribution was located for source graph acquisition, closure/dump
observation, lowering, catalog compilation, immutable-owner sharing, session import,
snapshot export, candidate reset/clone or opt-in traversal-witness collection. There are
correctness tests for aliases, cycles, ownership, cumulative bounds and parallel session
isolation; those establish semantics, not costs. The diagnostic-off path's lack of trace
records is not a measured zero-overhead claim.

Historical footprint fields are explicitly partial capacity accounting. For example, the
action-timing run reports **1,933 bytes of prepared components**, **221,152 bytes of compiled
actor heap**, and **1,358,112 inline bytes of benchmark-owned admitted handles**. Those are
different owners and are not a process RSS total. Shared definitions, strings/collections,
allocator metadata, thread stacks and other allocations are excluded. The closure-aware
test's **512 MiB allowance** is a resource limit, not observed consumption.

Zero-allocation regressions cover successful prepared numeric and stack-snapshot paths.
Owned measurements, candidate admission, source/session structures and the whole search
have additional allocations. Measure peak/retained RSS and allocation traffic separately,
including the effect of sharing one compiled owner across 1/N workers. No browser runtime
or WASM memory/throughput measurement was located; portable compilation proves neither.

## Acquisition, updates and debugging costs

The repository has exact source/package identities, extraction roundtrip checks and preserved
parity artifacts. Historical release extraction reports establish reproducibility, not
stage-separated acquisition time. The current CLI timeout spans source verification,
subprocess startup, source execution and validation; an increased test timeout is not a
measurement of any one phase.

All examined measurements still use the same pinned upstream revision. Consequently there
is **no measured end-to-end upstream-update effort** here: no timed revision-to-reconciled-data
migration, engineering/review hours, changed-rule acceptance rate or subsequent regression
triage cost. Injected-data tests prove supported values can change without recompiling an
algorithm; they do not measure the effort of introducing an unsupported effect family.

There is concrete evidence of diagnostic complexity that A1 should preserve in its comparison:

- R2t required paired source/native identity witnesses to identify the first modifier record
  at `cache[line][1][1]`; a line number or equal table contents could not establish its origin.
  The public parser still cannot return that successful miss natively because traversal
  evidence is unavailable. No proposed alternative has yet preserved this whole behavior.
- Linux constructor tests exposed a source compiler spill-slot limit, reproduced with fresh
  and reused processes. The repaired oracle separates a source warm attempt from proof that
  the exact function compiled. This was test/provenance work, not a native arithmetic defect.
- Recent CI includes long test runs, a hosted runner shutdown and the repaired schema assertion.
  These logs identify validation/triage burden; they do not isolate engineering time or prove
  an execution architecture's runtime cost.

Each prototype should account for acquisition adapters, generated artifacts, schema migrations,
runtime mechanisms, tests and manual failure investigation. Moving source interpretation into
an offline generator can move those costs rather than remove them. A fair comparison must
also retain useful source/definition/instance provenance and explicit unsupported results.

## Remaining measurements for A1/A2

1. Split cold package decoding/validation, native compilation, source observation/lowering
   and per-build preparation; record peak and retained memory for each owner/lifecycle.
2. Use the same admitted inputs and output contract for fresh versus reused preparation,
   then vary candidates and workers. Separate allocation-free numeric work from result
   construction, objective/scoring, cancellation and reporting.
3. Measure source-session cache-hit, no-match, successful miss, failure-prefix and repeated
   history cases only where the entire operation is supported. Record unsupported cases;
   do not divide partial executions into a successful-parse rate.
4. Measure reset/clone/import/export and diagnostics on/off using equivalent state, preserving
   aliases and behavior. Include private-state growth and shared-definition memory.
5. Keep the all-five original preparation failures as the breadth baseline. Add complete
   native throughput only as those builds become supported, with explicit coverage/exclusions.
6. Record update/migration/debugging effort during a real revision or effect-family change.
   Use those observations alongside runtime results before selecting A2 prototypes or targets.

Fresh runs should record the exact binary, source commit and dirty state, data/fixture hashes,
machine/runtime/profile, sample lengths, seeds and checksums, admitted/excluded counts,
warmup/calibration work, worker counts, failures/late results and timing endpoints. Keep all
resource limits visible. Do not claim an architectural speedup from unequal workloads or
an unmeasured baseline.
