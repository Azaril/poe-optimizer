# Typed native candidate evaluation

The native search path prepares immutable components once and calculates privately
validated candidate handles directly in Rust. It shares the numerical engine and injected
data with complete document evaluation. PoB remains an optional, explicitly selected
reference backend. Current admission covers the restricted Mace profile; this API does
not establish general build coverage or full native parity.

## Ownership and validation

`ControlledMaceCatalog::native_components` requires a fresh bound native baseline. It
retains the selected snapshot, exact source/configuration/catalog identities, parsed
weapon records, resolved tree choices and canonical support loadouts. Admission uses the
same strict importer and data-driven requirements as document evaluation.
`validated_native_candidate` produces an opaque `NativeMaceCandidate` only for a legal
member. Handles have no public numerical constructor or deserializer. The caller still
applies its own locks and point budgets through `CandidateDomain` before dispatch.

A private shared binding identifies the originating component set. Cloning that set
preserves compatibility; an independently created, equal-content catalog cannot supply
its handles to another prepared set. Backend and selected-data identities are checked
separately. Catalog identity and eager alternative XML hashes describe exact materialized source.
The local-weapon extension advances the catalog fingerprint for exact payload preservation;
problem schema 5/report 6 records that expanded item scope.

`NativeBackend::prepare_controlled_mace` parses and checks the source scenario once, then
prepares local weapon stats, tree, actor-resource and support axes. Actor preparation
uses the shared kernel once per tree/scenario and is bound to the selected compiled data. `PreparedMaceCandidates` retains numerical
components, selectors, identities and shared compiled data; it retains no XML and no
candidate result cache. Component storage scales with the sum of axis lengths, rather
than one prepared XML object per Cartesian candidate. Catalog construction and hashing
still visit the Cartesian alternatives, and the CLI retains admitted handles. Those
costs are outside the prepared numerical object's footprint.

Composition errors caused by injected modifiers are retained per tree or weapon choice. Evaluating
an affected handle reports the same error as complete document evaluation; an unrelated
choice does not fail preparation merely because that tree exists in the catalog.

## Calculation and verification

The calculation layers make their different costs explicit:

| API | Work |
| --- | --- |
| `PreparedMaceCandidates::calculate` | Resolve validated indices and calculate a numerical Mace output. |
| `PreparedMaceCandidates::measure` | Calculate and produce a stack `NativeMetricSnapshot`, including explicit unavailable values. |
| `NativeBackend::evaluate_controlled_mace` | Check backend/data binding and host deadline around a fresh snapshot calculation. |
| `PreparedMaceCandidates::snapshot_measurements` | Convert a snapshot into the scheduler's owned, selected metric measurements. |
| `Engine::evaluate` | Parse a complete document, calculate, construct results/diagnostics/export, and validate the backend contract. |

Successful numerical calculation, stack measurement and timed snapshot evaluation have
zero allocations in the allocation regression test. Owned measurement conversion still
allocates; candidate validation, objective assessment, archives, reporting and catalog
preparation also have their own costs. No whole-search allocation claim follows from the
numerical test. All snapshots remain diagnostic.

`search-experimental --backend native` defaults to `--native-evaluation typed`.
`--native-evaluation document` selects the full document path for differential checks.
The initial baseline and reserved finalist both use a fresh complete document evaluation,
including exact realization checks. `CandidateEvaluator::verify` is the counted finalist
hook; its default delegates to ordinary evaluation for other callers. Export materializes
the already verified candidate without an extra calculation. Empty legal domains spend
zero attempts; baseline, ordinary evaluations and verification share one ledger.

Reports add `calculation_path` and optional `native_candidate_preparation` evidence. The
latter records elapsed setup time, admitted handles, axis capacity estimates and the number of actor preparations. Zero full build
preparation calculations does not mean no numerical setup work: attributes/resources are
computed components. It is not a process-memory measurement. An explicit native
mode with `--backend pob` rejects; there is no implicit backend fallback.

## Reproducing performance measurements

The developer example rotates all legal mixed candidates and compares five API layers:
full document evaluation, prepared full-result evaluation, pure prepared calculation,
timed typed snapshots and typed snapshots converted to owned measurements.

```powershell
cargo run --release --no-default-features --locked --example benchmark_mace_candidates --target-dir target/native-only -- --evaluations 20000 --repeats 3 --jobs 1,2,4,32 --cpu-label "Describe the measured CPU" > runs/candidate-benchmark.json
```

The default `--candidate-set normal` uses weapon/support alternatives from
`examples/mace-support-search.json`. Add `--candidate-set local-weapons` to use the
supplied normal/rare alternatives and loadouts in `examples/mace-local-weapon-search.json`.
The normal/local-weapon sets use the pinned `mace-wooden.xml` calibration fixture and all 105 admitted tree
selections; the candidate-set file's template, objective, locks, neighborhood and tree
budgets are not benchmark inputs. JSON records the selected set and the two source fields
used. With bundled data, the local-weapon set has 4,410 structural and 2,949 legal candidates.

The `--candidate-set actor-resources` variant explicitly selects
`tests/fixtures/builds/mace-actor-resources.xml` and the weapon/support axes from
`examples/mace-actor-search.json`. Its authored actor modifiers affect eligibility and
resource preparation; bundled data produces 4,410 structural and 3,675 legal states.
The JSON identifies the actual fixed template. The benchmark resolves DPS by catalog ID,
so adding a metric such as Spirit cannot change its checksum input.

Choose worker counts for the machine. For longer samples of the inexpensive layers,
repeat with `--evaluations 1000000 --repeats 5 --modes pure_calculation,typed_snapshot,typed_owned_measurements`.
`--modes` selects a nonempty set of distinct known layers; checksum equality applies to
all selected modes in that invocation. `--data` selects an external package. Before timing,
the example compares every legal candidate's prepared selected hit DPS with its typed
selected hit DPS. It then rotates API order across repeats and checks an order-independent checksum
for every mode, worker count and repeat. JSON records the executable hash, backend/data/
catalog identities, hardware label, setup costs, evaluated count and raw samples.
Preparation and comparison calculations are recorded separately from timed samples.

Use an optimized build, finish unrelated compilation/test jobs before measuring, retain
raw distributions, and report setup separately. The pure numerical mode omits metric and
deadline work, while full-result modes include attachments/export construction. Component
capacity estimates exclude shared data, the import catalog and allocator metadata; they
are not peak RSS. Measure whole search separately, including admission, hashing, scoring,
verification and report/export work. These finite Mace measurements do not predict speed
for complete builds or demonstrate optimizer quality at 5–30 minute budgets.

See the [implementation checkpoint](implementation.md) for dated results, [native backend](native-backend.md)
for admitted mechanics, [controlled mutations](controlled-mutations.md) for source fidelity,
and [search contracts](search-kernel.md) for budget and verification rules.
