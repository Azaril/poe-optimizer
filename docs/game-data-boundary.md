# ADR: injectable game data for native evaluation

- Status: accepted direction; the interfaces below are planned, not implemented APIs
- Date: 2026-09-07
- Decider: user direction to load game data from configuration and inject it into evaluation
- Scope: portable game-data model, loading, native calculation inputs and dataset identity

## Context

Game patches change skill values, item bases, monster tables, passives, rewards and balance
parameters more often than they change the calculation operations themselves. Most game
content must therefore be configuration data with its own model and lifecycle. Translating
Lua tables into Rust constants would keep the runtime native but leave updates coupled to
compiling and releasing the evaluator.

The existing `poe-optimizer-data` crate is the starting point, not a completed injection
boundary. It currently supplies a compiled class/tree subset through a global loader;
Spark/Mace values, weapon definitions, monster tables and exact passive-effect mappings
still live in Rust. Current implementation and migration evidence belong in the
[living implementation plan](implementation.md), not in this end-state decision.

## Decision

Extend `poe-optimizer-data` into the independent, versioned model and loader for game-data
packages. The host selects and supplies bytes; the portable loader validates those bytes;
the engine compiles supported records into immutable calculation data. Inject that data
into each native backend at construction. No evaluator or numerical kernel selects a
process-global dataset, reads a file, fetches a URL or invokes PoB to resolve missing data.

Ship a reviewed default data package for convenience. Embedded default bytes are a
packaging option and must use the same validation and injection path as an external
package. Compatible data-only updates must work without rebuilding the Rust evaluator.
Changes to mechanic semantics or the supported operation set can still require Rust changes.

### What is data and what is code

| Concern | Owner and representation |
| --- | --- |
| Game identity, patch, source lineage, schema, required mechanic versions and content digests | Data-package manifest and typed identity |
| Classes, ascendancies, base attributes, passive graph, overrides, point categories and choices | Game-data records with stable IDs and explicit coverage |
| Skill/support identities, tags, stat sets, level/quality values, compatibility and requirements | Typed skill/gem records and tables |
| Item bases, affix/unique definitions, roll bounds, requirements, local/global modifiers and granted effects | Typed item and modifier records; actual owned items stay in the run's inventory |
| Resource/attribute coefficients, resistance caps, quest rewards, encounter defaults and monster level tables | Versioned rules parameters and lookup tables; selected rewards and encounter choices remain run inputs |
| Passive/item/gem effects, conditions and modifier dependencies | Declarative typed effect records targeting versioned operations; preserve source ordering, actor scope and provenance |
| Arithmetic, rounding order, aggregation, conversions, actor resolution and special mechanic algorithms | Rust calculation semantics consuming explicit data parameters |
| Structural validation, known operation implementations, coverage enforcement and resource limits | Rust code; a package cannot declare an unimplemented mechanic supported |
| Objectives, thresholds, required skill/item sets, inventory, scenario choices and search budgets | Separate per-run problem configuration, resolved against a specific data snapshot |

Distinguish balance parameters from mathematical constants: a game resistance cap belongs
in data; converting percentage points to fractions is part of unit semantics. When a
coefficient embedded in a formula is patch-dependent, expose it as a versioned rules
parameter with a parity test. A new rounding algorithm requires a reviewed semantic version,
not an untyped configuration expression.

The data model must express effects structurally, rather than using English display text
as the runtime definition. Extraction/import can preserve and interpret source text, but
the hot path consumes typed stat IDs, values, operation kinds, flags, conditions and actor
targets. A new passive using existing supported operations should need data changes only.
Unsupported operations remain explicit errors; arbitrary Lua, JavaScript or native plugins
inside packages are outside this boundary. This does not require designing a general-purpose
formula language before the existing numeric profiles can accept injected data.

### Package and runtime models

Keep these representations separate:

1. **`GameDataPackage`**: serialized manifest and source-derived records. Begin with versioned
   JSON for reviewable changes; reserve a compiled binary representation for measured need.
   It includes per-section coverage and omitted/unsupported source evidence. Missing tables
   are explicit, never populated from hidden Rust defaults.
2. **`GameDataSnapshot`**: owned, validated, immutable portable model. Private validated state
   and read-only accessors prevent callers from changing records after identity calculation.
   It preserves game/source IDs and shared physical-node versus effective-node distinctions.
3. **`CompiledGameData`**: engine-owned immutable indexes, dense tables, resolved effect
   programs and compatibility evidence derived from one snapshot. Compilation checks the
   implementation's operation/capability registry. It contains no calculated build results.
4. **Prepared build and task state**: normalized candidate inputs bind to that compiled
   snapshot. Mutable scratch buffers, actor state and candidate-dependent caches belong to
   one evaluation/task and never mutate the shared data.

The manifest needs game namespace, data release/patch, schema version, required semantic
versions, section/file digests, coverage and source/extractor provenance. `DataIdentity`
uses verified content digests, not a mutable filename or display label. Source lineage may
include a PoB revision during migration; a production dataset must not require a PoB
checkout, runtime or even a PoB origin. PoE1 uses its own namespace and rules compatibility;
loading PoE1 data does not make PoE2 calculation semantics valid for that game.

### Ownership and injection seam

| Package | Owns / depends on |
| --- | --- |
| `poe-optimizer-core` | Lightweight data identity in evaluator/run/result contracts; no concrete game database |
| `poe-optimizer-data` | Package schemas, portable byte decoding, validation and immutable snapshots; may use core identity types, never depends on engine/native/PoB |
| `poe-optimizer-engine` | Operation semantics, compatibility compilation and calculation over borrowed resolved data; depends on portable data models |
| `poe-optimizer-native` | Backend instance containing shared compiled data, build preparation and typed results; no acquisition I/O |
| CLI / future GUI / browser host | File/network acquisition, selected package/trust policy, resource accounting, loading and backend composition |
| Optional PoB adapter / extraction tooling | Source extraction and parity production; emits the same package contract without becoming a native runtime dependency |

The intended seam is a concrete immutable model, not a virtual provider lookup for every
stat. Alternative package sources converge at byte loading and validated snapshot
construction. Introduce a provider trait only if acquisition adapters need one; it must not
perform I/O during preparation or calculation.

Illustrative future API, not code available today:

```rust,ignore
let snapshot = GameDataLoader::from_bytes(&package_bytes, &trust_policy, &load_limits)?;
let data = Arc::new(CompiledGameData::compile(Arc::new(snapshot))?);
let backend = NativeBackend::with_data(Arc::clone(&data), host_clock)?;
let prepared = backend.prepare(&request)?;
let result = backend.evaluate_prepared(&prepared, budget)?;
```

`NativeBackend` owns an `Arc<CompiledGameData>` and instance-specific backend identity.
`PreparedEvaluation` retains shared ownership of its originating data and the relevant
semantic identity, even when its numerical inputs have already been copied/resolved.
Use stable IDs or validated indices instead of self-referential or `'static` record borrows.
Pure kernels accept small borrowed views or copied parameters, with no Arc cloning in
inner arithmetic loops.

`evaluate_prepared` checks the prepared data/semantic identity against the receiving backend
before calculation. Another instance with the same verified identity may reuse it;
logical equality must not depend on pointer address. A different dataset must reject the
prepared input, rather than calculate with one dataset and label the result with another.
The standalone `PreparedEvaluation::calculate` path uses its retained data exclusively.

One host may construct A and B backends simultaneously. Neither construction changes the
other instance. A running job keeps one immutable snapshot; loading an update creates a
new snapshot for a new job. Old prepared builds remain valid for their original data while
it is retained. A bundle convenience constructor belongs at composition and delegates to
the same path; it cannot be a fallback when an explicitly selected package fails to load.

### Validation, compatibility and provenance

Loading is bounded by bytes, record counts, string sizes, effect complexity and nesting.
Validate schema, duplicate keys/IDs, finite numeric values and field ranges, units, table
coverage, referential integrity, ownership, unsupported effect-dependency cycles and unambiguous selectors
before publishing a snapshot. Preserve ordered modifier sequences where order affects
rounding; canonical hashing must not reorder semantically significant lists.

Compute identity from the manifest and actual section contents. An expected digest or
trusted manifest comes from the host's explicit trust policy, outside the package's own
claims. A matching self-asserted hash proves neither provenance nor correctness. Custom
local packages can be explicitly selected without pretending to be reviewed official data;
record that status and their derived content identity in runs. Start with complete packages.
If overlays are added later, preserve base/overlay lineage, define deterministic precedence,
validate the complete merged model, and identify the resolved content separately.

Treat these as separate questions:

- Is the artifact structurally valid and consistent with its expected content identity?
- Are the game and required operation/schema versions compatible with this engine?
- Are the selected build's records and effects within implemented coverage?
- Has this exact dataset/engine combination passed parity against a matching reference?

None implies another. Known unsupported source records can remain explicitly uncompiled
with coverage evidence; they must fail if selected. Invalid structure or ambiguous meaning
fails loading. Declared coverage cannot extend the engine's capability registry or replace
point budgets, ownership rules or equipment/gem legality checks.

Keep the current exact source-pin guard during migration. Replace it only with an explicit
reviewed compatibility contract covering schema, operation semantics and dataset identity.
Do not accept arbitrary data by removing the guard. Data-only balance updates using known
semantics should not need a newly hard-coded Rust allowlist of content hashes. New source
releases still need source review and matching parity before being advertised as validated.

Backend identity, prepared state, candidate catalogs, evaluation caches, checkpoints,
reports, diagnostics and exports/run manifests all retain the effective data identity.
Keep data content identity separate from engine/adapter implementation fingerprints and
metric schema versions. Existing combined fingerprints must be migrated together with the
contract checks and a report/cache schema version, rather than silently changing their
meaning. Calculation caches cannot cross data versions even when a patch label is reused.
Exports that cannot embed metadata must carry a companion manifest for reproducible reload.

Changing only objective thresholds may reuse measurements from the same data/engine;
changing game data requires fresh evaluation. Search materialization and the evaluator
must use the same snapshot, including tree, skill and item catalogs. File locations and
process memory addresses never form part of semantic identity.

### Parallel execution and portability

Load, validate and compile once at run setup, then share immutable data across Rayon tasks.
The steady-state evaluation path performs no JSON parsing, file/network access, global
mutable lookup, data deep-copy or per-stat locking. Account for snapshot/compiled memory
once per shared allocation and for retained older snapshots; initialization time and memory
remain visible in run/benchmark records. Candidate-dependent work still consumes the shared
execution budget.

Browser hosts supply bytes and scheduling through the same API. Data/model/compiler code
must compile for WebAssembly without Lua, filesystem access or OS clocks. Successful
compilation is a portability check; browser execution, parallelism and performance require
separate tests. Benchmark load/compile time, prepared calculation and typed-result costs
separately before selecting a binary layout or adding another cache.

## Alternatives and tradeoffs

| Option | Benefit | Cost / decision |
| --- | --- | --- |
| Rust constants or generated Rust tables | Simple static access | Requires rebuilding for content changes and encourages duplicate data; reserve for mathematical/implementation constants |
| Injected raw maps or a dynamic data provider on every lookup | Flexible source access | Repeated validation, string lookup and possible I/O/locking in the hot path; resolve once instead |
| Arbitrary scripting for all mechanics | Broad configurability | Adds execution and semantic complexity before parity exists; use typed data plus versioned Rust operations |
| Validated packages compiled once and injected as immutable data | Independent content updates, deterministic identity, shared memory and native performance | Requires schema/compatibility validation and explicit lifetime ownership; selected approach |

## Acceptance criteria

- The same native executable loads the bundled and an external representation of the same
  reviewed dataset through one path and produces equal metrics, coverage and content identity.
- A fixture package with a controlled numeric change alters the expected result and data
  identity without editing/rebuilding Rust. It is labeled synthetic/custom, not PoB parity.
- Concurrent A/B/A evaluations and one/many-worker runs demonstrate isolation. Cross-data
  prepared-input reuse and cache/catalog mismatches reject; identical-data instances work.
- Corrupt, oversized, duplicate, incomplete, unknown-schema, wrong-game, incompatible-operation
  and invalid-reference packages fail with actionable errors; no fallback/default fill occurs.
- Independent existing goldens and fresh source/full-build parity still pass for the reviewed
  default dataset. Expected results are never generated from the data under test.
- Source extraction reproduces the package deterministically. PoB update tests load the
  matching optional reference and compare exact dataset/engine pairs without requiring PoB
  in native production packaging.
- Native-only dependency checks, portable WASM compilation and allocation/throughput
  measurements verify the intended boundary. No universal speedup is assumed from this design.

Migration tasks, their order and the audit of current hard-coded data are maintained in
[implementation.md](implementation.md#injectable-game-data-design--2026-09-07). This decision
refines the [calculation boundary](calculation-boundary.md) without changing the agreed joint
search scope or the optional role of PoB.
