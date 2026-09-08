# ADR: replaceable calculation backends

- Status: accepted
- Date: 2026-09-07
- Scope: calculation core, evaluation engine, application and execution boundaries

## Context

The optimizer needs PoB's existing calculations while a native Rust replacement develops
in parallel. Search, objectives and applications must survive that replacement without
understanding Lua tables or depending on a process protocol. The native calculations must
also be usable in a browser host. The user explicitly requested these boundaries and the
parallel translation track; implementation progress belongs in [implementation.md](implementation.md).

## Decision

Use two interfaces in `poe-optimizer-core`:

- `CalculationBackend` owns calculation implementation, declared capabilities, rules/data
  identity and deadline enforcement. It consumes a typed evaluation request and returns
  typed measurements, observed context, coverage, exports and diagnostics.
- `EvaluationEngine` is the application/search-facing interface. It validates requests and
  backend responses. Execution services will add scheduling, caching, cancellation and
  shared budgets around this boundary without exposing backend internals to callers.

The CLI composes `Engine` with a selected native or PoB backend. Generic and boxed backend
implementations share these contracts. The production target is native-only packaging and
direct Rust parallel evaluation; the PoB backend remains optional for parity and updates.
Neither interface requires Lua handles, filesystem paths, process IDs, a webview or a
particular thread/async runtime. The PoB adapter privately owns those host dependencies.

`BuildDocument` carries a versioned interchange format and content. PoB XML is a supported
interchange format that either implementation can parse. It is not the canonical candidate
model: complete class/ascendancy/tree/equipment/skill/support state, stable requirements and
mutation/export fidelity need their own typed models. Native import must not call Lua at
runtime. Add formats and normalized data deliberately rather than expose private PoB state.

Native candidate adapters may prepare privately validated typed components and return
selected measurements without serializing a document or constructing diagnostics on each
search attempt. This is an additional prepared-input boundary over the same injected data
and calculation semantics. Complete document evaluation remains the interchange, baseline
and fresh-finalist contract. The generic search kernel delegates that final check through
`CandidateEvaluator::verify`; it does not require a native handle type or an IPC transport.
See [typed candidate evaluation](native-candidate-evaluation.md) for the current bounded adapter.

Metric IDs are extensible strings with actor scopes, versioned definitions and explicit
units. A requested metric has exactly one finite, classified nonfinite or unavailable
value. Unknown metrics, mismatched units, missing results, wrong backend identity and
mismatched requested options are contract failures. An empty metric filter requests the
entire declared catalog, including unavailable values for absent actors. Raw backend
snapshots are optional opaque diagnostic attachments; scoring must use typed measurements.
A catalog declaration identifies semantics, not universal mechanic validation.

`full_build_evaluation` means the backend accepts a complete build document, distinguishing
it from numerical helpers. It does not certify legality or complete mechanic coverage.
Per-result coverage and diagnostic/verification status remain separate. Do not register a
partially translated kernel collection as a complete native build evaluator. A complete
document pipeline with explicitly bounded mechanic coverage may expose this capability
while rejecting every build outside that coverage. This is not full game support, and
must never silently fall back to PoB or substitute zero for missing calculations.

Keep `poe-optimizer-engine` focused on portable calculation semantics over injected game data.
`poe-optimizer-data` owns the versioned model, portable byte loader and validated immutable
snapshots; the engine compiles supported records into shared calculation tables. The host
selects the package and injects compiled data into each native backend. Data content and
balance values remain configuration, with typed Rust operations defining their behavior.
See the [game-data boundary](game-data-boundary.md) for ownership and compatibility contracts.
The engine must not depend on `poe-optimizer-pob`, Lua, the CLI or an OS scheduler. Translation
proceeds in cohesive slices against the pinned Lua oracle, as described in
[native-engine.md](native-engine.md). Export, data extraction and packaging are separate
adapter responsibilities. `poe-optimizer-import` owns shared portable interchange; native
build preparation and typed backend adaptation belong in `poe-optimizer-native`.
A build-time Lua data extractor does not imply a Lua runtime
dependency for the native/browser evaluator.

The initial engine interface is synchronous. Backend implementations enforce their own
deadline; the generic engine cannot forcibly interrupt a borrowed Rust call. PoB uses
supervised processes. A native host needs cooperative deadline/cancellation checks or an
execution boundary it can terminate. Portable math functions allocate no thread pools.
Desktop scheduling uses the shared Rayon/resource budget; browser scheduling is supplied
by its host. Do not make every backend use IPC merely because PoB needs process isolation.

## Alternatives and tradeoffs

| Option | Benefit | Reason for this decision |
| --- | --- | --- |
| Application calls PoB worker protocol directly | Small initial wrapper | Exposes runtime and raw-field assumptions to objectives and frontends. |
| One calculation/evaluation interface owns all scheduling | Fewer interfaces | Couples numerical implementation to cache, process and application lifetimes. |
| Fully translate PoB before building an evaluator | One runtime immediately | Delays useful validation and removes the executable oracle during development. |
| Require IPC for every backend | Uniform hard termination | Imposes serialization/process requirements on native and browser hosts. |
| Separate semantic contracts and host adapters | Extra conversion/contract checks | Preserves a usable PoB path while native calculations and execution hosts evolve. |

## Consequences and validation

Maintain source, effective data-content, rules/semantic, adapter and metric identities with
every result and cache key. Identity is backend-instance-specific. Prepared inputs retain
their data and semantic identity; a backend must reject reuse with a different dataset even
when both instances expose the same backend ID. Core validation and persisted schemas must
preserve that distinction as injection is implemented.
The adapter fingerprint covers calculation shims, metric mapping, backend conversion,
core contracts and process supervision. Backend swaps require equivalent supported
semantics, not just matching trait signatures. An intentional rules change must be visible.

Objective assessment uses a separate `ScoringPolicy` contract over typed measurements;
see [objective assessment](objective-assessment.md). Saved assessment must validate recorded
structures and retain metric versions without requiring the current calculation backend.

Use fake backends to test engine invariants independently of Lua. Test PoB against separately
produced reference outputs, then differential-test translated Rust slices against actual
upstream functions and full candidate states before integrating them. Compile portable
libraries for WebAssembly, and add browser execution tests when bindings exist. Compilation
alone establishes neither browser functionality nor a speed improvement.

Observed configuration tables are evidence, not a fully resolved scenario model. Skill
indexes are scoped to the evaluated active skill set and pinned catalog, not stable user
requirement IDs. Those limitations must remain visible until canonical candidate, scenario
and coverage models have complete supported semantic integration. See [execution-and-interfaces.md](execution-and-interfaces.md)
for the target job API and [design.md](design.md) for complete optimizer acceptance criteria.
