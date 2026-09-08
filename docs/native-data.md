# Native game-data packages

The native evaluator loads a versioned JSON package into `GameDataSnapshot`, compiles it
once into `CompiledGameData`, and shares immutable data across prepared builds and Rayon
workers. The native calculation path uses injected records; it performs no configuration
parsing, I/O or data hashing. This implements the current-profile slice of the
[game-data boundary](game-data-boundary.md). Delivery evidence and remaining gates live in
[implementation.md](implementation.md).

## Included data and current limits

The reviewed package contains the existing class/root/entrance tree subset, character
resource and accuracy parameters, level-one Spark/Mace identities and values, Brutality I,
two normal weapon bases, quest rewards/defaults, defence coefficients/caps, 100-level
monster armour/evasion tables, encounter defaults, typed entrance effects and explicit
level/attribute requirements for the included skills, support and weapon bases. Support
color and per-color aggregate attribute costs also come from data. Display
stat text is source evidence; typed effect IDs and values drive the calculation.

The package is explicitly partial. It adds no new skill, support, equipment or tree coverage.
The two Mace weapon IDs are compiled capability slots whose records come from the package.
Unknown fields/operations, malformed identities, missing records and unsupported selectors
reject. Gameplay input bounds and supported operations remain enforced in Rust.

Structural tree content and class attributes remain bound to the reviewed tree artifact
and exact source compatibility guard. Custom numerical/effect records can change; editing
topology or its provenance requires the remaining source/compatibility migration. The
supplied minion build and unrestricted native evaluation remain unsupported.

## CLI loading and authoring

`evaluate`, `metrics`, `benchmark-native` and `search-experimental` accept `--data <package.json>` for the native
backend. Omission selects the embedded reviewed package through the same validated loader.
An external byte-identical copy has the same identity and reviewed status. Other packages
are custom/unreviewed unless the host supplies an expected digest from an external review.
An expected hash verifies identity; it does not establish numerical parity.

```powershell
cargo run --no-default-features --locked -- evaluate examples/native-witch-entrance.xml --data crates/poe-optimizer-data/data/game-data.json --raw
cargo run --no-default-features --locked -- benchmark-native examples/native-witch-entrance.xml --data crates/poe-optimizer-data/data/game-data.json --mode prepared --jobs 4 --evaluations 10000
```

`--data-sha256 <lowercase-sha256>` requires `--data` and rejects mismatched bytes without
falling back. Native-only packaging needs no PoB checkout. The PoB backend rejects data
selection because it uses its own reviewed source/runtime.

For local edits, copy the package to a separate file and change supported records. Refresh
the section digests with the developer authoring helper, which validates the result and
requires a new output path:

```powershell
Copy-Item crates/poe-optimizer-data/data/game-data.json runs/custom.edit.json
# Edit the copied JSON's supported values before sealing it.
cargo run -p poe-optimizer-data --example seal_package --locked -- runs/custom.edit.json runs/custom.json
cargo run --no-default-features --locked -- evaluate tests/fixtures/calibration/spark-mapping.xml --data runs/custom.json
```

`seal_package` decodes bounded regular-file input, refreshes section hashes, validates the
package, then writes it. It does not derive values from PoB, confer trusted origin,
or overwrite existing output. The native executable can read the resulting package without
recompilation. Do not edit the reviewed artifact or original numerical goldens to manufacture
parity. The optional [pinned source exporter](game-data-extraction.md) produces all current
sections from source; broader source-version updates remain unfinished.

To regenerate the reviewed package from the pinned PoB checkout, use the development CLI:

```powershell
cargo run --locked -- extract-game-data --output runs/extracted-game-data.json
```

It also writes an `.extraction.json` evidence companion. Native evaluation can load the
result through `--data`; extraction is separate from the native runtime.

Native `search-experimental` loads the selected snapshot once and shares it between the
controlled catalog and evaluator. Skill/support identities, generated XML, weapon names,
quest selectors/defaults and requirement checks consume that snapshot. PoB reference binding
still accepts only the reviewed default content.

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-search.json --data runs/custom.json --jobs 4 --max-evaluations 10
```

Search rejects candidates that fail the closed profile's level/attribute requirements
before calculation. The required value for each attribute is the maximum of individual
item/active/support requirements and the support-color aggregate. Item level is separate
from equip-level requirements. Reports retain rejected choices and their required/available
values; if every locked choice fails, `empty_legal_domain` reports zero evaluations and
writes no XML. Diagnostic `evaluate` remains distinct from search legality. See
[controlled mutations](controlled-mutations.md) for the supported scope.

## Library composition

```rust,ignore
use std::sync::Arc;
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy};
use poe_optimizer_native::{CompiledGameData, NativeBackend};

let snapshot = GameDataLoader::from_bytes(
    &bytes, &TrustPolicy::AllowCustom, &LoadLimits::default(),
)?;
let data = Arc::new(CompiledGameData::compile(Arc::new(snapshot))?);
let backend = NativeBackend::with_data(Arc::clone(&data), host_clock)?;
let prepared = backend.prepare(&request)?;
let result = backend.evaluate_prepared(&prepared, budget)?;
```

The host supplies bytes and clock. `NativeBackend::new` / `with_clock` are convenience
constructors for the compiled reviewed default. Explicit `with_data` never silently selects
another package. The default uses a cache for immutable data initialization; it does not
change the dataset of existing instances or cache calculated build results.

Prepared inputs retain shared data ownership and originating backend identity. Distinct
instances with the same verified data/semantic identity may reuse them. Different datasets
reject before calculation. `PreparedEvaluation::calculate` uses the retained data directly.
`CalculationBackend::identity` optionally advertises full instance identity; the core engine
checks it in addition to capability IDs. Native always advertises it.

Pure Spark/Mace `evaluate_with_data` functions receive `&CompiledGameData` explicitly.
Legacy `evaluate`, `evaluate_with_character`, `data()` and default-character helpers obtain
the same reviewed package. They contain no duplicate balance-value database. New hot paths
should use the injected APIs.

## Identity, reports and exports

`BackendIdentity.data` records game, release, schema, semantics version and verified package
byte digest. Data identity is separate from implementation/source fingerprints and trust.
Changing only a numeric record changes data identity while retaining the implementation
fingerprint. JSON formatting changes can change the byte digest; that conservatively prevents
cache reuse. All section digests must also match canonical typed records.

Evaluation reports use schema **3**; saved assessment accepts schemas 2 and 3, and schema 3
native reports require a valid data identity. Native results carry reviewed/custom trust in
warnings, so it remains visible without `--raw`; raw mode additionally retains structured
`game-data` and configured entrance-effect diagnostics. Controlled-search schema **2** records selected data identity/trust, requirement rejection
evidence and the existing evaluation budget ledger. Its native export companion retains
structured trust and the selected package path as a reload hint. Benchmark schema **2** reports
structured `data_trust`, identity and separate backend/data initialization time.

Native `evaluate --export <file.xml>` and controlled native search write `<file.xml>.data.json`
next to XML. The companion records backend/data identity and XML hash for reload. Output
collisions reject before evaluation. XML bytes remain source-preserving; re-evaluate custom
exports using the matching data package. A filename hint is neither content identity nor
trust. Existing source-only imports still preserve input bytes.

## Schema migration

Package schema **2** and semantics **`poe2-native-profiles-v2`** require explicit
`requirements` records on Spark, Mace, Brutality and each weapon. Weapon requirements replace
`required_strength` with level and all three attributes. Mace additionally records support
attribute costs, and Brutality records its color. Support individual attribute requirements
must remain zero; change the matching-color cost to configure their attribute demand.
Schema-1 packages fail explicitly; use
`extract-game-data` to regenerate from the pinned source, then reapply and review custom
edits. Changing only the schema number does not migrate missing records. Existing tree
schemas, source pin and numerical goldens are unchanged.

## Validation and update procedure

The default package is 141,502 bytes, SHA-256
`854dca85abcd031905761d9533b7437ce28e40b5154c23c99655084fbb719507`.
Its original tree bytes/source manifest and six independent goldens remain unchanged.

```powershell
cargo test -p poe-optimizer-data --locked
cargo test -p poe-optimizer-engine --test data_injection --locked
cargo test -p poe-optimizer-cli --no-default-features --test native_data --test native_data_cli --locked
cargo test -p poe-optimizer-pob --test game_data --locked
cargo test -p poe-optimizer-cli --test native_passive_parity --locked
```

The optional source tests verify package records against pinned Lua data, modifier parsing
and source expressions, including warmed functions and every effective entrance. The full
matrix checks complete native builds against fresh PoB evaluations. Custom-package tests
are isolation and controlled-change evidence, not PoB parity claims. Preserve source review,
strict compatibility and new differential evidence when deliberately updating a package.
