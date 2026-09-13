# Shared native preparation boundary

Status: R1c outer integration, R2 authored skills, configuration loading and the R2y
finite item assembly/inventory prefix are implemented. General real-build numerical producers remain
unfinished; complete native supplied builds remain **0/5**.

## Public entry points

`NativeBackend::prepare_view(build, view, options, metrics)` consumes the exact owned import
and independently resolved `SelectedView`. It checks source-owner and definition-owner
binding before lowering. It returns `PreparationOutcome::Ready(PreparedEvaluation)` or
`Incomplete(PreparationReport)`. Invalid requests and foreign bindings remain errors.
Incomplete preparation contains no calculated metrics and cannot enter the calculation loop.

`prepare_request_with_lineage(request, lineage)` imports a caller XML document, resolves its
saved view and returns that same detailed outcome. `prepare_with_lineage` maps an incomplete
outcome into the existing evaluator error contract. The existing `prepare` and ordinary
`CalculationBackend::evaluate` use this same path, so the shared model is part of actual
native evaluation, not only inspection.

The native convenience host assigns a fresh lineage using OS randomness once during import.
It never derives independent instance IDs from source hashes, labels, zero values or worker
order. This host dependency is excluded from WASM. Portable callers supply lineage through
the explicit method or use an already-owned import/view. Browser hosts must use those
methods; raw convenience preparation returns guidance when no host identity is available.
Core model, definition compilation and repeated calculation need no randomness or host I/O.

`PreparedEvaluation::source()` retains shared imported ownership and `selected_view()`
exposes its immutable report. `authored_skills()` retains the independently executed,
source/view/data-bound [authored skill stage](authored-skill-preparation.md).
`authored_configuration()` retains the [configuration loader prefix](configuration-preparation.md)
and its explicit continuation before activation. `authored_items()` retains the
[owned item assembly and inventory prefix](native-item-assembly.md): all authored item
occurrences, the executed registration prefix, duplicate-ID lookup winners and explicit
source continuation. These independent stages do not fabricate
completed root setup or effective scenario state. The compatibility request DTO still owns an XML string in
addition to the imported source; it is not copied per instance or per calculation. Removing
that duplicate DTO storage can follow a measured API migration rather than compromising
source ownership. Reports cannot manufacture a prepared plan.

## Existing numerical adapter

The current Spark/Mace adapter now resolves selected authored source ranges into one
short-lived Rust XML document and consumes those nodes for skills, passives, items and
configuration. Request bytes must match the view owner. Source problems, foreign bindings
and unsupported requested weapon states cannot silently retarget another view.

The adapter retains its prior closed-domain checks: one set per domain, canonical source
IDs, one selected group/action, primary weapon state and supported mechanics. This preserves
existing numerical/export/coverage contracts while general producers are built. Simply
removing those checks would misreport inactive effects, action indices and export selectors.
Processed gem identity, levels, quality, effects and support/provider role must agree with
what the closed numerical adapter admitted. Loaded explicit configuration scalars and migrated modifier blocks must
also agree with the raw adapter input before encounter overrides. Valid injected loader data can alter those
results; an unsupported change returns Incomplete instead of calculating stale numbers.

No third skill or build-specific profile was added. General inputs instead produce named,
source-linked prerequisites through the same public preparation entry point.

A Ready plan means ready for the existing declared diagnostic metric/capability contract.
It is not full PoB parity, build legality, complete effective graph admission or EHP coverage.
The retained selected-view report describes source resolution; its general producer
frontiers are not falsely marked complete by a closed legacy calculation.

## Structured incomplete reports

The nested item inventory report uses schema 2. Its numeric armour diagnostic map explicitly
marks incomplete projections; the owned item graph remains authoritative.

`PreparationReport` schema 5 contains the selected view, source/definition identity,
the executed authored-skill, configuration loader-prefix and ordered item inventory reports,
classified issues, requested options/metric queries and the legacy adapter's separate rejection.
Request context records intent, not calculated output. Each issue names its stage
and, where applicable, the concrete authored instance and source occurrence.

Kinds distinguish unresolved identity, ambiguous identity, source errors, unsupported
boundaries and deferred producers. Authored name matching, identity resolution and level
processing retire only the corresponding prerequisites for actually processed entries.
Partial failures retain the unprocessed suffix and the original source-linked failure.
Effective stat sets, support application, actor ownership and provider availability remain
separate dependencies even after authored loading completes.

Reports expand independently discoverable selected groups, entries, equipment/rune uses,
passive specs and jewel assignments. Item inventory records are not labeled equipped solely
because their raw numeric IDs match slot references. The new item prefix registers only
actually executed items with a base and a complete final owned assembly. It retains later
unprocessed occurrences and stops before unavailable container instructions. Registration
alone does not establish selected equipment or actor effects. Configuration effects, passive allocation/version
rules, item/provider lifecycle and root load ordering stay explicit. Dependencies hidden
behind unexecuted producers are not invented.

Diagnostic expansion has independent caps of 65,536 issues and 4 MiB of added message text.
Exceeding a cap returns a resource error with no partial report. Import, source-view and
injected definition limits apply separately. The report is evidence of reached preparation,
not a complete inventory of every future calculation dependency.

## Candidate and parallel paths

Controlled candidate preparation executes the same authored skill stage and validates its
projection into the selected-source adapter for its fixed
scenario. Fixed-scenario setup also executes configuration loading and validates explicit
scalar and migrated modifier-block agreement; a changed injected migration cannot bypass document admission through
the candidate API. `prepare_controlled_build_with_lineage` and
`prepare_controlled_mace_with_lineage` support portable host identity assignment; their
existing convenience wrappers allocate it at native setup. Prepared candidate calculations
still retain typed numerical components and private ownership bindings, not source XML or
selection reports. Source/view traversal occurs during preparation, outside repeated
calculation and Rayon dispatch. Introduced candidate supports receive the same loader
validation during setup, using temporary support-only source variations from the import
materializers. Per-axis failures affect only candidates selecting those supports; successful
repeated calculations keep their allocation-free contract.

## CLI

```powershell
cargo run --no-default-features -- prepare-build tests/fixtures/builds/breadth-20260908/build-02.xml
```

This uses the real native preparation API and reports either
`ready_for_supported_native_metrics` or `incomplete`. It performs no calculation. It accepts
one XML/share-code input, optional existing evaluation `--options`, injected `--data`, and
`--output` to a new file. Existing destinations and caller inputs are preserved. A successful
reporting command with status `incomplete` does not mean native evaluation succeeded.

Use `evaluate --backend native` for actual supported calculations. Its concise incomplete
error preserves the original legacy-adapter rejection. Detailed
reports are available through `prepare-build` and the library API without changing the
backend-neutral calculation/evaluation traits.

## Validation and next integration

The existing six numerical calibration documents retain their independent goldens through
owner/view and compatibility paths. Tests also cover caller changes, poisoned cached outputs,
exact ownership, explicit overrides, prepared lifetime and concurrent pure calculations.
All five supplied originals reach structured incomplete reports with their independent
selected alternatives and concrete source bindings. Custom injected catalogs change
identity diagnostics without source changes, demonstrating the data seam remains active.

The [implementation record](implementation.md) records terminal native/CLI regression,
Clippy and WASM checks. Authored loading compares all five original sources, including
inactive saved sets, against complete original skill methods. Next connect effective
configuration and provider/actor/action preparation for Twister and Skeletal Sniper together,
retaining the other originals as structural stress cases.
Real item-reference winners and full loader/provider lifecycle require their own paired
source evidence as those producers become executable. No source/helper test count replaces
the R3/R5 full-build numerical gates.
