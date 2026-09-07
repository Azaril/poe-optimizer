# Implementation log and resume point

Last updated: 2026-09-07

Current phase: injectable game-data architecture is recorded; runtime injection is the
next implementation gate, before further class/passive search expansion. The
[data-boundary decision](game-data-boundary.md) separates versioned configuration packages,
immutable data models and compiled calculation data from Rust mechanic semantics. This
checkpoint changes documentation only; no injection API or external-data CLI option exists yet.

The validated code baseline remains `36fc552`, documented through `2675ffb` on `main`:
restricted native Spark/Mace evaluation across eight classes, 23 ascendancy identities and
zero or one ordinary entrance; a portable authenticated tree subset; 277 workspace tests,
16 native-only CLI tests, five portable WASM libraries and hosted Windows/Linux CI passing.
One hundred fresh PoB evaluations show zero observed numeric difference. The prior 4.5-million
evaluation benchmark remains specific to `fa5136b`. Numerical constants and global data access
still need migration. Full native game coverage, the supplied minion build and the first
usable all-six-dimension optimizer remain unfinished.

This is the living record of delivery order, implemented behavior, validation, unresolved
work, and the next session's starting point. [Design](design.md) defines the intended system
and acceptance criteria; [execution and interfaces](execution-and-interfaces.md) and the
[calculation boundary decision](calculation-boundary.md) define
runtime and application contracts. The [PoB investigation](pob-integration.md) and
[prior-art review](prior-art-and-product-review.md) retain dated evidence and rationale.
Update this document at progress checkpoints, rather than adding implementation status to
the design documents.

## Resume here

1. Inspect `git status --short --branch` and the current validation/publication table below.
   Read the [data-boundary decision](game-data-boundary.md) and complete D1–D5 below before
   broadening class/passive search. The previous native class/entrance implementation is
   validated and published, but still obtains data through globals and Rust constants.
   Also read [native backend](native-backend.md),
   [native calculations](native-engine.md), [live passive coverage](passive-coverage.md)
   and [tree projection](tree-projection.md). The production target is a fully native
   parallel evaluator; PoB is an optional explicit reference, never a hidden fallback.
2. Keep PoB at `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` and unmodified. Preserve supplied
   original exports and independent Spark/Mace goldens. Maintain actual-source interpreted
   and warmed parity, full-build differential cases and strict unsupported-mechanic errors.
3. Implement the injectable-data migration in the checkpoint below. Start with typed package
   schema/validation and the instance-owned data seam, then move existing Spark/Mace values,
   tables and effects into configuration. Keep prepared-data ownership, instance identity,
   search/catalog/cache consumers and compatibility guards coherent at each step. Preserve
   current unsupported-mechanic admission and reference parity while replacing globals.
4. Connect source-preserving class/ascendancy/passive materialization to a finite search
   catalog using the authenticated portable data. Rebind composed catalog identities to
   tree, skill and item payloads; validate requested versus realized physical allocations,
   effective entrance stats and source exports. Start with the numerically validated
   31 identities and 16 ordinary entrance cases, explicit caller 0/1 ordinary-point and
   zero ascendancy-point budgets, shared-root ownership and exact locks. The native
   evaluator's observed allocation count must never establish its available point budget.
   Equipment/gem requirements also need explicit finite rules before cross-class item
   recommendations: diagnostic calculation alone does not prove they can be equipped.
   The current controlled Mace search remains Warrior/no-paid-passives/none-ascendancy.
5. Broaden source-derived passive/modifier extraction beyond the twelve exact admitted
   stat strings and nine numeric modifier fields, with actual-source and full-build parity
   before relaxing admission. Preserve the data bundle's distinction between retained and
   excluded records; schema-1 snapshots must be regenerated for schema 2. Connect broader
   equipment, support and supporting-skill catalogs together, keeping multiple required
   skills/items, cross-class search, exact locks and ownership/resource/point accounting.
   No selected special node, grant, socket or unknown mechanic may disappear during projection.
   Global extraction uncertainty remains separate from unsupported selected mechanics.
6. Replace closed profiles with reusable complete native pipelines as coverage permits:
   item/gem/passive modifiers, resolved actors/conditions and resources, offence/defence,
   conversions, ailments, triggers and minions. PerStat/StatThreshold programs now support
   ordered numeric dependencies; actor-target multipliers, recursive producers and special
   GetStat names remain explicit gaps. No profile count or shared trait establishes full
   game replacement. Source/data upgrades require source identity and parity review.
7. Specialize coordinated proposals with game-aware repair and diverse archives. Preserve
   exhaustive tiny references, deterministic one/many-worker comparisons, locks and empty
   sample retry bounds. Benchmark random, greedy and alternating baselines at 5, 15 and 30
   minutes in mapping/bossing before making optimizer-quality claims. Current eight-state
   Mace search is a diagnostic integration test, not that benchmark or the first product.
8. Profile preparation/calculation/result costs on realistic supported native builds before
   tuning. Reuse immutable prepared inputs, compiled modifiers and task-local scratch state;
   avoid per-candidate XML/diagnostic/export construction in a future native search hot path.
   Keep versioned result/provenance contracts and fresh native finalist recalculation.
9. Add shared memory/CPU admission, CLI signal cancellation, progress events, checkpoints,
   persisted cache and throughput scheduling. Optional PoB workers retain separate supervision;
   their fresh-process/reset requirements do not apply to native execution. Every dispatched
   calculation, preparation evaluation and finalist check remains in the shared budget ledger.
10. Add offline comparison/HTML reports, then GUI and browser bindings. Portable compilation
   does not prove browser execution, browser parallelism or performance. The browser host
   supplies clocks/scheduling and uses the same calculation contracts.
11. Update this resume point and checkpoint evidence before the next handoff.

For targeted data questions, the user supplied PoE2DB/PoEDB; see the
[lookup constraint](design.md#external-lookup-references). Do not scrape or bulk-extract either site.

No product-scope answer blocks these tasks. Exact objective metrics, thresholds, scales,
required skill/item subsets and encounter assumptions remain explicit per-run inputs.
Assessment reports constraint evidence and primary availability; it does not certify build
legality or turn diagnostic calculation output into a recommendation. All calibration cases
compare independent hosts/extractors using shared PoB calculations, not independent game models.

## Injectable game-data design — 2026-09-07

The user clarified that most game data must load from configuration through its own model
and injection seam. [The accepted direction](game-data-boundary.md) extends the existing
portable data crate, makes backend/prepared identity instance-specific, and shares immutable
compiled data across workers. Game values and content become data; Rust retains arithmetic,
rounding and operation semantics. Embedded defaults use the same loader as external packages.
Data-only changes using supported semantics must not require rebuilding the evaluator.

This is a design/documentation checkpoint on the clean `2675ffb` baseline. No production
code, fixtures, game-data artifacts or submodule pin change here. The runtime APIs and
migration tasks below are **not implemented**. The previous code validation remains evidence
for that baseline only; Rust tests are not repeated for these document-only edits.

### Audit of the current data coupling

| Area | Current implementation / migration work |
| --- | --- |
| Backend and prepared ownership | [Native backend](../crates/poe-optimizer-native/src/lib.rs) stores only a clock; prepared profiles have no dataset owner/identity. Introduce shared data ownership and reject incompatible prepared reuse. |
| Tree access and provenance | [Native tree](../crates/poe-optimizer-native/src/tree.rs) obtains global bundled data and keeps static record borrows. Replace with instance-resolved IDs/indices and retained immutable ownership; diagnostics use the selected data identity. |
| Package loading | [Bundle loader](../crates/poe-optimizer-data/src/bundled.rs) uses a compiled expected digest and global `OnceLock`; [source model](../crates/poe-optimizer-data/src/tree_data.rs) fixes source/tree selection. Add portable byte loading with explicit trust/compatibility inputs while preserving source evidence. |
| Numerical content | [Spark](../crates/poe-optimizer-engine/src/spark.rs) embeds skill/resource/quest/resistance values. [Mace](../crates/poe-optimizer-engine/src/mace.rs) embeds weapon definitions, accuracy/support values and monster tables. Move records and patch-dependent parameters into the package; retain pure calculation semantics. |
| Further parameters and effects | Audit base evasion and other balance coefficients in [character](../crates/poe-optimizer-engine/src/character.rs) and [defence](../crates/poe-optimizer-engine/src/defence.rs). Replace native tree's twelve English-line effect mappings with typed effect records and versioned operation validation. |
| Admission and defaults | [Native profile](../crates/poe-optimizer-native/src/profile.rs) and native reporting embed skill/item identities and quest/encounter defaults. Resolve them from data; keep operation support and structural restrictions in code. Configured content must not expand unsupported mechanics by declaration. |
| Identity consumers | Global `backend_identity()` feeds results, [benchmarking](../src/native_benchmark.rs), [native search binding](../src/mutation_search.rs) and [catalog checks](../src/catalog_search.rs). Migrate these together. Core's current result/backend ID check alone cannot distinguish two datasets under `native-poe2`. |

### Migration sequence and acceptance gates

- [ ] **D1 — data model and loading.** Define manifest, lightweight core `DataIdentity`, typed
  records and bounded byte loader in `poe-optimizer-data`. Separate owned validated snapshot
  from serialized DTOs. Cover the existing tree, character, skill/support, item, quest,
  encounter and level-table data first. Preserve raw-source provenance, partial coverage,
  physical/effective IDs and order-sensitive modifier semantics. Validate duplicate IDs/keys,
  ranges/units, references, missing tables and schema/game identity. Supply trust through the
  host; package claims do not authenticate themselves. Keep dependencies one-way.
- [ ] **D2 — instance injection and identity.** Add compiled data ownership to native backend
  construction and prepared evaluation, replacing global tree reads and static record
  lifetimes. Kernels consume borrowed resolved data; compilation checks executable mechanic
  capabilities. Migrate backend identities, result construction, diagnostics, benchmark/search
  factories and catalog checks together. Version affected persisted contracts/cache identities.
  Prove A/B/A isolation, same-data cross-instance reuse and different-data prepared rejection.
  Keep the existing exact source guard until an explicit equally strict compatibility contract
  is ready; do not weaken admission simply to make edited packages load.
- [ ] **D3 — externalize the current native data.** Move the audited Spark/Mace constants,
  weapons, monster tables, rewards/defaults and patch-dependent formula parameters out of Rust
  and into the reviewed package. Replace English stat matching and numerical magnitudes with
  typed effect records whose operations are implemented in Rust. Separate generic operations
  from supported-profile admission. Default kernels and native documents must use the injected
  values; compatibility wrappers may not become a second hard-coded source of truth. A
  controlled synthetic package must change a supported result without Rust changes and must
  not inherit the reviewed dataset's identity or PoB parity status.
- [ ] **D4 — host loading and source updates.** Compose CLI default/external byte loading through
  one path, with explicit selection failures and no fallback. Carry effective data identity into
  run/checkpoint manifests, exports and search configuration. Build deterministic optional PoB
  extraction for the expanded package and retain exact source/parity provenance. Embedded and
  external copies of identical data must yield identical semantic identity/results. New source
  releases using supported operations update data/compatibility evidence without recompiling
  a Rust allowlist. Keep arbitrary custom packages clearly distinct from reviewed parity data.
- [ ] **D5 — parity, portability and performance checkpoint.** Preserve the six independent
  goldens and 100-case fresh PoB matrix for the reviewed default data. Exercise invalid-package
  and cross-data cache/catalog cases, one/many-worker isolation, native-only dependencies and
  five portable WASM libraries. Measure initialization separately and confirm repeated prepared
  calculation neither reloads nor hashes configuration. Record memory ownership and supported
  fixture/machine identities before judging throughput. Update this resume point, then continue
  class/passive materialization with explicit point and equipment/gem legality rules.

After D1 establishes concrete interfaces, numeric record extraction and independent
contract/parity fixtures can proceed in parallel in separate files. Keep backend identity
integration under one owner. A tree-only injection checkpoint does not complete D3 or the
requirement to externalize most game data.

Design verification: reviewed against the current code and independently audited for
ownership, provenance, compatibility and concurrency gaps. All 221 local file links and
20 heading links across 28 Markdown documents pass, as does Git whitespace validation.
One pre-existing stale M1 heading link was repaired. This checkpoint changes documentation
only; code checks were not rerun. No product-scope answer blocks D1; use versioned JSON and
complete package replacement initially, and preserve the native/optional-reference architecture.

## Native class and ordinary-passive evaluation — 2026-09-07

Starting point: `5dd77eb` on `main`, clean checkout. This implements the planned portable
data consumer and native class/ordinary-entrance calculation phases. No end-state scope
was removed. The supplied minion build and unrestricted native evaluation remain unsupported.

- `poe-optimizer-data` owns portable snapshot types, source identity/content authentication,
  automatic source views and finite graph projections. PoB compatibility paths remain;
  extraction and process supervision stay in the optional adapter. Snapshot schema 2
  preserves raw class-array positions separately from canonical integer IDs. Old schema-1
  snapshots reject explicitly and must be regenerated.
- The authenticated 131,681-byte bundle retains 40 source nodes: 28 physical implicit roots
  and 12 physical ordinary entrances, yielding 16 class-specific entrance views. Its coverage
  explicitly excludes 4,874 of 4,914 source nodes. Generation proves all 31 class/ascendancy
  selections have no implicit-root effects and that ascendancy selection leaves the admitted
  ordinary entrance effects unchanged. Witch/Sorceress and Ranger/Huntress shared roots,
  Witch node 4739's source 17306, Huntress node 56651's source 39263, and shared Lich/Abyssal
  Lich roots remain distinct evidence. The bundle is source data, never cached numerical output.
- `engine::character::CharacterInput` carries class attributes and nine resolved numeric
  modifier fields into `spark::evaluate_with_character` and `mace::evaluate_with_character`.
  Existing `evaluate` wrappers retain their six unchanged goldens. Both pipelines now compute
  armour/evasion as well as prior resources/resistances/hit outputs. Skill speed and flagged
  damage increases follow source rounding and modifier semantics; minion damage has no actor
  in these supported zero-minion profiles. Public metric availability is unchanged.
- Native document selection follows the active skill independently of class. Admission
  verifies class/ascendancy identity fields and admits roots plus zero or one class-connected
  ordinary entrance. Duplicates, foreign roots, unsupported paid ascendancy allocations,
  other ordinary nodes, choices, overrides, weapon sets and unknown stat forms reject.
  Immutable prepared inputs still run entirely in Rust without Lua/process calls.
- A `native-tree` diagnostic records native source resolution, physical allocations,
  effective paid-node stats/provenance and bundle identity. It explicitly sets
  `point_budget_verified: false`. The separate PoB live-pointer passive observation field
  stays absent on native results. Exports preserve admitted input bytes apart from the
  established explicit-options/cache cleanup. [The Witch example](../examples/native-witch-entrance.xml)
  demonstrates an ascendancy identity plus a class-switched ordinary entrance.

| Validation | Current evidence |
| --- | --- |
| Portable data and extraction | **21 targeted tests pass:** six Lua-free bundle/authentication tests, one exact fresh-source bundle reproduction, seven extraction tests and seven projection tests. Targeted Clippy and data WASM compilation pass. |
| Numerical source parity | **31 modifier/source tests pass**, including interpreted/warmed full ModParser semantics for all 16 effective entrances and both pipelines across levels/mitigation/rounding boundaries. Both independent golden tests covering six untouched fixtures pass; engine Clippy and WASM compilation pass. |
| Native admission/exports | Four new contracts pass across 62 class/ascendancy documents, 32 entrance documents, coupled shared-root cases, legacy/canonical class IDs, byte-preserving exports and unsupported input cases. |
| Fresh full-build parity | **100 fresh PoB evaluations pass** in a targeted 128.11-second run with two workers: 98 primary class/entrance/coupled-selection cases plus two native-export reimports. Maximum observed absolute and scaled numeric delta: **0** across public metrics and extra raw-stat comparisons. Mace cases have positive enemy evasion and uncapped hit chance. Native-only builds compile the oracle target with zero tests. |
| Full integrated validation | **277 workspace tests pass, zero failures**, eight ignored child helpers exercised by parent tests; **16 native-only CLI tests pass**. Workspace/native-only Clippy with warnings denied, formatting, native dependency audit and five portable library WASM targets pass. Logs: `runs/native-class-final-workspace-tests.log`, `runs/native-class-final-native-only-tests.log`, `runs/native-class-final-clippy.log`, `runs/native-class-final-native-only-clippy.log`, `runs/native-class-final-wasm.log`, `runs/native-class-final-dependencies.txt`. |
| Data/calculation compatibility | Native preparation now explicitly requires the bundle and both numerical pipelines to agree on rules revision, tree version and tree source hash. Twenty native tests pass, including mismatched/missing source identity cases; native Clippy and WASM checks pass. The guard changes admission only, with no formula/result-shape changes. Logs: `runs/native-class-source-guard-tests.log`, `runs/native-class-source-guard-clippy.log`, `runs/native-class-source-guard-wasm.log`. The complete integrated validation above includes this guard. |
| Initial integrated correction | The first full run caught one obsolete rejection assertion for newly supported node 4739; it now uses an unsupported node. No calculator change was required. The complete rerun passes. Initial failure log: `runs/native-class-workspace-tests-initial.log`. |
| Native-only release example | `evaluate examples/native-witch-entrance.xml --backend native --raw --timeout-seconds 30`: Witch/Abyssal Lich, allocated physical IDs `[4739,23710,54447]`, effective entrance source 17306. Life **809**, mana **315**, selected average hit **6.54**, selected hit DPS **9.342857142857143**. XML export is byte-identical; source/export SHA-256 `fcb6ad36f0991733fe9ed9ec9fada97ee4cebab4d68247c1a53a50aab54c63ed`. The final `36fc552` native-only release reproduces these metrics and bytes. Artifacts: `runs/native-class-final-example.json` and `.xml`; release build log: `runs/native-class-final-release.log`. This example is source-derived demonstration data, not an independent numerical golden. |
| Documentation and source preservation | 195 local file links across tracked Markdown documents and Git whitespace checks pass. Supplied original exports, independent fixtures and the pinned submodule are unchanged. |
| Publication and hosted CI | Published to `origin/main` through `36fc552`. Initial code `fa5136b` passes both [Windows/Linux jobs in run 34169436926](https://github.com/Azaril/poe-optimizer/actions/runs/34169436926). Final source-guard code `36fc552` also passes both Windows/Linux jobs in [run 34170105192](https://github.com/Azaril/poe-optimizer/actions/runs/34170105192). |

Bundle SHA-256: `272c40b13109c999e4e28693a5c64dfe9625ed24c106c83e2a132794ac955411`.
Full schema-2 source snapshot SHA-256:
`68445629df3af8bb934aad91f5e6b457f8fe7e478b67e306493ee9a017d171f7`.
Regeneration/authentication procedures are in [tree data](tree-data.md); the byte digest
comes from the checked-in trusted artifact manifest, not the input's self-asserted labels.

At this code checkpoint the next work was class/passive materialization. The new data
injection migration now precedes that search gate in the resume steps above. Native point-budget/gear legality, broader modifier coverage and all-six-dimension
optimization remain unfinished. Previous throughput measurements below apply to their
explicitly named prior code and fixed Spark profile, not automatically to this expanded result path.

### Release throughput for the class/entrance example

Measured code: `fa5136b`; input: `examples/native-witch-entrance.xml` (2,831 bytes;
source hash above). AMD Ryzen 9 9950X3D, 16 physical cores / 32 logical processors,
Windows 11 Pro 10.0.26200 x64, Rust 1.93.0. Native-only default release build/target
settings; no concurrent local Cargo or parity workload. OS/background load was not controlled.

Three independent 250,000-evaluation runs per mode/thread count, cycling repetitions,
thread counts and modes, with a 60-second deadline per run. All **4,500,000** evaluations
completed, with zero failures/late discards, stable backend identity and identical finite
metric checksums. No calculation warmups or result cache. Preparation is timed separately.

| Threads | Prepared typed results/sec, median (min–max) | Document typed results/sec, median (min–max) |
| --- | --- | --- |
| 1 | 89,798 (88,465–90,069) | 27,116 (26,942–27,463) |
| 4 | 259,882 (252,937–260,103) | 83,496 (82,347–85,391) |
| 32 | 466,612 (422,521–488,600) | 189,033 (179,382–193,643) |

Both modes recompute the native calculation and construct/validate complete typed results,
including the new tree/source diagnostics, XML export, Rayon scheduling, accounting and
metric checksums. Prepared mode reuses validated source inputs; document mode reparses and
resolves them each time. These are tiny fixed-profile API observations, not pure-kernel
speed, arbitrary-build throughput, search quality, browser performance or a PoB speedup.
Some prepared runs remain below one second and show substantial variation. The prior
checkpoint used another profile/result shape; the two tables are not a controlled regression
comparison. The preparation/result cost reinforces the separate compiled-input/minimal-output
search work in the resume plan.

Reproduce with `benchmark-native examples/native-witch-entrance.xml --mode prepared|document
--jobs 1|4|32 --evaluations 250000 --timeout-seconds 60 --output <new-path>`, three runs each.
Ignored artifacts: `runs/native-class-throughput/{prepared,document}-jobs*-r*.json`,
`machine.json`, `summary.json`; driver: `runs/benchmark-native-class-checkpoint.ps1`.
Per-result finite-metric SHA-256:
`3dbf33c2ddbdc3595304372a141f9cfc98df9368736a196b7087ff99982c14d1`.
Measured `fa5136b` native-only release executable SHA-256:
`48b12d9a52994ef41c12df31f5bfd3841d061ad8be7be26d958fea2f188a3587`.

## Native build pipelines, worker-free search and passive parity — 2026-09-07

The preceding native agent work stopped at 23 numerical/modifier tests; it was not a build
evaluator. This checkpoint adds complete admitted native document-to-result paths and uses
them in the controlled optimizer. The end-state design now explicitly requires a full Rust
replacement and native-only production packaging; PoB remains loadable for reference tests.

- Native Spark covers the unallocated Sorceress, level-one Spark, supported quest/resistance
  inputs and nine finite resource/resistance/hit metrics. Native Mace covers the unallocated
  Warrior, Mace Strike, two normal weapon bases, quality 0–20, item/character level 1–100,
  optional Brutality I and a normal enemy with explicit armour and optional evasion. Mace
  computes eight finite public metrics; selected average hit stays unavailable because PoB
  stores it per hand. Neither profile claims EHP, maximum hits, ailments, minions or Full DPS.
- `NativeBackend` validates XML into immutable `PreparedEvaluation`, runs actual translated
  math and returns the common contract with input/metric/source identities and supported
  exports. The pure calculation path allocates no state and calls no OS runtime. Typed
  evaluation has cooperative clock checks; browser hosts inject a clock. Cached source
  outputs are ignored and stripped from exports, and explicit options are baked into XML.
- Bounded import, structural preflight and controlled materialization now live in portable
  `poe-optimizer-import`. PoB module paths re-export them for compatibility. Native evaluator
  and search do not depend on the PoB adapter. The developer CLI retains its default PoB
  feature; `--no-default-features` packages native evaluation/search without Lua or reference
  commands. `--backend native|pob` switches the implemented evaluation/search backends.
- Controlled native search uses the existing Rayon CPU execution policy and shared budgets,
  exact candidate locks and fresh finalist checks. Native realization verifies materialized
  source, class/skill/item state and external context; candidate-derived conditions can vary.
  Four original independent attack goldens still validate the quality-zero interactions.
  Eight-state exhaustive/guided runs agree on `smithing-q20/none` (20.1596255 selected hit DPS).
- `benchmark-native` measures repeated prepared typed evaluation or complete document parsing
  and evaluation using an explicit local Rayon pool. It counts attempts, failures, late
  results, preparation and checksum observations; there is no cached-result or calculation
  warmup. This is fixed-input API throughput, not raw math speed or search quality.
- Native modifier programs now handle ordered PerStat/StatThreshold scaling with explicit
  unsupported special GetStat/actor/recursive forms. The engine has 34 calculation tests plus nine shared XML-parser tests, including
  both unchanged Spark goldens, four unchanged Mace goldens and source-executed numerical
  grids in interpreted and warmed LuaJIT modes. Class table index versus internal ID, enemy
  level clamping at 85 and monster physical-reduction cap 75 are modeled distinctly.
- Live passive coverage records actual allocated physical nodes, count buckets, roots,
  switch-source identities and displayed stats. The 49-build matrix covers all 31 class/
  ascendancy identities, 16 entrances, the supplied build and a weapon-set allocation, with
  at most two fresh PoB workers. Core coverage validates those observations structurally.
  An authenticated tree projection admits selected ordinary Normal nodes and preserves
  shared ownership/graph edges; selected unsupported mechanics reject explicitly.

- Review fixed quadratic export cleanup with one ascending source-span copy, including a
  20,000-cached-row regression preserving a large UTF-8/CRLF Notes payload. A shared lexical
  gate now rejects XML forms the pinned parser silently reinterprets: numeric references,
  spaced attribute equals, quoted delimiters, split CDATA and a leading UTF-8 BOM. Native adds stricter attribute
  normalization checks; reference preflight preserves the supplied build's valid multiline
  quest strings. Lossless import remains unchanged. Nine XML compatibility tests, including actual-source probes, cover those
  parser differences, and both backend fingerprints include the new gate implementation.

These native profiles do not yet support the supplied complex minion build.

| Validation | Evidence |
| --- | --- |
| Full Windows workspace | `cargo test --workspace --all-targets --locked`: **260 passed, zero failures**, seven ignored child helpers exercised by parent tests. Log: `runs/native-checkpoint-workspace-tests.log`. |
| Native-only CLI | `cargo test -p poe-optimizer-cli --no-default-features --all-targets --locked`: **16 passed**, including default-native selection, absent PoB commands, both benchmark modes, native search and exports. Optional reference tests are feature-gated. |
| Formatting and lint | Workspace formatting, workspace all-target Clippy and native-only CLI all-target Clippy pass with `-D warnings`. |
| Portable compilation | Core, engine, import and native library targets compile for `wasm32-unknown-unknown`; this does not establish browser execution. |
| Runtime dependencies | Native-only normal dependency graph contains no PoB adapter, mlua, LuaJIT or UTF-8 native module. CI now enforces this graph check. Log: `runs/native-checkpoint-dependencies.txt`. |
| Reference parity | Two unchanged Spark and four unchanged Mace goldens, actual-source numerical grids, 16 fresh Spark/Mace PoB evaluations including native-export reimports, plus the 49-build passive matrix all pass. |
| Release search | Native-only release CLI, `search-experimental --backend native --problem examples/mace-search.json --jobs 4 --max-evaluations 10`: template + eight candidates + fresh finalist, winner `smithing-q20/none`, selected hit DPS **20.1596255**, XML export written. Observed search/calculation duration **2.0101 ms**, excluding final persistence; a tiny diagnostic domain, not an optimizer-quality benchmark. Artifacts: `runs/native-checkpoint-search.json`, `runs/native-checkpoint-best.xml`. |
| Publication and hosted CI | Published to `origin/main` through `115a902` with explicit user authorization. [Windows/Linux run 34166302194](https://github.com/Azaril/poe-optimizer/actions/runs/34166302194) passes both jobs: formatting, workspace/native-only lint and tests, native-only dependency checks and portable WASM compilation. Initial native code `a09c406` first ran in `34165834314`, where Rust 1.98 found `chunks_exact_to_as_chunks` in benchmark accounting; `115a902` uses `as_chunks::<8>()`, retaining the arithmetic and passing both six-test benchmark suites and workspace Clippy on Rust 1.93. The following resume update changes documentation only; 179 local file links and Git whitespace checks pass. |

### Release fixed-input native throughput

Measured code: `a09c406` (the subsequent checksum-iteration lint fix retains its arithmetic).
Machine: AMD Ryzen 9 9950X3D, 16 physical cores / 32 logical processors, Windows 11 Pro
10.0.26200 x64, Rust 1.93.0. Built with `cargo build -p poe-optimizer-cli --release
--no-default-features --locked`, default release profile and target settings. No concurrent
Cargo/parity workload ran during measurements. OS/background load was not controlled.

Each row/mode used three independent runs of 250,000 evaluations and a 60-second deadline,
cycling modes and worker counts. All **6,000,000** requested calculations completed with
zero failures or discarded late results, identical nine-metric checksums across all runs,
and no calculation warmups or result cache. The fixed source was
`tests/fixtures/calibration/spark-mapping.xml`; metrics are the supported native Spark
profile. Report preparation time separately from iterations.

| Threads | Prepared typed results/sec, median (min–max) | Document typed results/sec, median (min–max) |
| --- | --- | --- |
| 1 | 165,064 (159,166–167,229) | 32,802 (32,567–32,852) |
| 4 | 403,810 (391,700–422,923) | 95,812 (93,992–96,070) |
| 16 | 699,762 (617,413–864,572) | 166,516 (164,984–167,140) |
| 32 | 1,074,253 (973,278–1,102,594) | 215,034 (211,105–215,866) |

Prepared mode reuses parsed inputs and reconstructs/validates the complete typed result,
including XML export and diagnostics. Document mode also repeats parsing and engine request
validation. Both include Rayon scheduling, shared attempt accounting and metric checksums;
neither measures raw math alone. Some prepared runs last under one second and show material
variation, so these are initial fixed-input observations rather than precise scaling targets.
They do not measure arbitrary builds, native versus PoB speedup, browser performance or
search quality. The preparation gap motivates keeping compiled native inputs in future
search hot loops instead of reconstructing XML and full presentation output per candidate.

Reproduction: build as above, then `poe-optimizer benchmark-native
<fixture> --mode prepared|document --jobs <1|4|16|32> --evaluations 250000
--timeout-seconds 60 --output <new-json-path>`, three repetitions each. Ignored artifacts:
`runs/native-pipelines-throughput/{prepared,document}-jobs*-r*.json`, `machine.json`,
`summary.json`; driver: `runs/benchmark-native-checkpoint.ps1`. These are local evidence,
not repository fixtures. Per-result finite-metric SHA-256:
`913cc534e08a78f9bc894e9db238723f799c5fa4e8973a1fa4074e44c1b7db49`.

## M2 controlled mutations, tree data and native multipliers — 2026-09-07

This checkpoint replaces fixed-document membership with a separate structural mutation
profile for supplied normal weapons/supports and prepares real tree data for broader
joint integration. It does not change the agreed release scope or calculation boundary.

- `search-experimental --problem <json>` accepts a source template, exact weapon alternatives,
  support choices, independent locks, configurable scalar objective/constraints and proposal
  settings. Exhaustive and guided modes share candidate/scoring/evaluation APIs. The example
  has eight combinations. One template calculation establishes scenario drift evidence and
  one fresh finalist calculation is reserved; failures count and all attempts share the run
  duration. Output contains exact catalogs, constraints, provenance and verification.
- `ControlledMaceCatalog` validates a structural profile, patches only item/support source
  ranges and preserves all other template bytes. It supports Wooden Club/Smithing Hammer,
  item level 1–100, quality 0–20, no support/Brutality I, character/enemy levels 1–100 and
  bounded explicit encounters. It rejects unsupported mechanics before calculation.
  Candidate-derived conditions may change; external inputs and persisted source state stay
  guarded. Parameterized realization is runtime drift evidence, not an independent golden.
- The discrete proposer samples up to 128 axes without constructing huge products. It cycles
  mutation radii, changes multiple unlocked choices and samples full restarts. Fixed axes
  remain fixed. An empty random sample can retry until explicit bounds, so it cannot suppress
  a later coordinated move. Seven tests cover exhaustive reference agreement, parallel
  determinism, extreme cardinalities, locks, fixed domains and empty-sample regressions.
- `extract-tree` runs a distinct supervised offline worker with startup-inclusive deadlines,
  private bounded artifacts, kill/reap and source verification. Its owned snapshot preserves
  all 4,914 physical nodes, eight classes, 23 ascendancies, 5,187 usable undirected edges,
  shared owners, overrides, typed source tables and all 14 dangling pairs. Extraction is not
  allocation legality. Rust snapshot types are currently adapter-owned; no Lua handles enter
  the serialized interface. This work uses only the pinned local submodule.
- Native numeric multiplier programs preserve explicit multiplier layers, BASE/OVERRIDE
  precedence, parent grouping, ordered Multiplier/MultiplierThreshold/Limit evaluation,
  dynamic divisors, caps and existing condition gates. Seven additional differential tests
  bring native coverage to 23 tests. Unsupported recursive or actor-target forms fail
  explicitly; real-build extraction and a complete native backend remain future work.
- Independent reviews caught early random-sample stalls, omitted canonical lock evidence and
  late timer start; regression coverage or integrated domain checks now guard those paths.
  Full exported-frame checks account for nondeterministic Lua table serialization while
  preserving ordered gems and candidate-derived conditions.

| Validation | Evidence |
| --- | --- |
| Full Windows workspace | `cargo test --workspace --all-targets --locked`: **188 passed, zero failures**. Five ignored child helpers are invoked by their parent tests. Log: ignored `runs/m2-mutations-workspace-tests.log`. |
| Formatting and lint | Workspace formatting and `cargo clippy --workspace --all-targets --locked -- -D warnings` pass. |
| Portable boundary | `cargo check -p poe-optimizer-core -p poe-optimizer-engine --target wasm32-unknown-unknown --locked` passes. Host search and PoB extraction are outside this gate; this is not browser execution or a speed measurement. |
| Mutation parity and CLI | Seven adapter tests and four CLI tests pass, including unchanged independent Q0 Mace goldens, Q20/level/Pinnacle realization, source-frame drift, locks, finite/guided search, partial budgets, output collisions and tree CLI behavior. |
| Documented supplied search | Ignored `runs/m2-mutations-64290528.json` and `.xml`: jobs 2, eight combinations, ten total attempts, no failures, consistent fresh verification and source export. Winner `smithing-q20/none`, selected hit DPS `20.1596255`, debug total `15513.8403 ms`. This is a single observation, not a scaling benchmark or new independent numerical golden. Adapter fingerprint `3fbb6e59049912f9e0bd914fd84fd3d46ab5a44a555231f3c203b36ade1353b8`. |
| Tree extraction | Seven source-data tests and two supervisor tests pass. Ignored standalone `runs/tree-0_5-64290528.json` has canonical snapshot SHA-256 `390e699115ac358b0da57b312e7449db919da124822365b888b5ade1dc97c433`; extractor fingerprint `a1c740c1ba3f8af90b9f423e1a973de8e14cd2cd938e1df9f4faebc004dd177c`. Identity-claim checks alone do not authenticate edited snapshots. |
| Integrity and review | Submodule, original supplied exports and independent goldens are unchanged. Independent scoped code reviews, README/docs local file-target checks and `git diff --check` pass. |
| Publication and hosted CI | Published as `55df9d5` to `origin/main`. [Windows/Linux CI run 34159259405](https://github.com/Azaril/poe-optimizer/actions/runs/34159259405) passed both jobs, including formatting, Clippy, full workspace tests and portable core/native WASM compilation. The following resume-document update changes documentation only. |

## M2 candidate/search and calibration CLI checkpoint — 2026-09-07

This checkpoint connects canonical finite candidates to a replaceable search/evaluation
boundary and a real PoB integration harness. It does not narrow the first usable optimizer
scope to weapon/support selection.

- Core candidate/lock contracts retain classes, ascendancies, passives, equipment,
  support gems and supporting skills. Fourteen tests include simultaneous changes in all
  six dimensions while preserving two required skill definitions and two exact items;
  reverse-edge connectivity, owner/point accounting, multiplicity, independent locks,
  resources and explicit unsupported mechanics are covered. Shared physical class and
  ascendancy roots/paid nodes use validated owner sets, including pinned Witch/Sorceress
  and Lich/Abyssal Lich relationships; effective override extraction remains explicit work.
- The host search crate uses Rayon for Rust CPU evaluations and scoped supervisor threads
  for external processes. Twenty tests compare a coupled six-bit search with all 64
  states, compare one/multiple jobs and both schedulers, preserve infeasible exploration,
  and exercise deduplication, failures, exact budget boundaries, cooperative cancellation,
  late results and fresh-verification drift. No Lua/process type enters candidate/scoring
  contracts. Search proposals remain domain-provided complete states.
- The PoB registry resolves only four independently calibrated source hashes, preserves
  original XML bytes and hashes exact item/gem payloads. Candidate membership includes
  immutable source/catalog identity. Fresh realization checks cover class roots, passive
  allocations, equipment payloads, gem identities/configuration and encounter settings;
  only verified upstream normalizations are accepted. See [bridge scope](pob-candidates.md).
- `search-calibration` exposes configurable scalar goals/constraints, jobs, shared duration
  and attempt limits. Four exploration attempts plus one fresh finalist attempt exercise
  the complete catalog. JSON includes catalog, source labels/hashes, effective budgets,
  backend identity, assessments and verification; an exact source export requires a fresh
  verified feasible incumbent. All results retain diagnostic status. The four-case DPS
  winner is Smithing Hammer without Brutality; the support-equipped pair ranks Wooden Club
  higher, preserving the independently demonstrated interaction.
- Native `Condition`/`ActorCondition` evaluation preserves parent/skill/actor fallback,
  override and weapon exceptions plus inactive MORE precision and source error ordering.
  Five additional actual-upstream tests bring the native crate to 16 tests. Explicit
  resolved condition tables are supported; FLAG-derived conditions, multiplier/scaling
  tags, real-build extraction and full native evaluation remain unsupported.
- Review fixes include last-round proposal admission, cancellation precedence and controls,
  late/failure accounting, full fresh assessment/metadata comparison, malformed finite
  evidence rejection, scenario drift, hidden item XML children and shared physical-node
  ownership. No user design input
  was needed; package separation implements the existing portable-core boundary.

| Validation | Evidence |
| --- | --- |
| Full Windows workspace | `cargo test --workspace --all-targets --locked`: **154 passed, zero failures**. Both ignored helpers are explicitly invoked by their parent deadline/fresh-process tests. Log: ignored `runs/m2-search-workspace-tests.log`. |
| Formatting and lint | `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings` pass. |
| Portable boundary | `cargo check -p poe-optimizer-core -p poe-optimizer-engine --lib --target wasm32-unknown-unknown --locked` passes with the shared-root model and condition implementation. Search's host scheduler is deliberately outside this gate. |
| Standalone calibration search | Ignored `runs/m2-search-9b151354.json` and `.xml`: jobs 2, five attempts including one fresh verification, `mace-smithing`, selected hit DPS `18.208694`, verification consistent. Debug search elapsed `7151.7147 ms`; this single observation is not a scaling benchmark. Adapter fingerprint `b2c24db59723ee873659e5cd044ae43bb3d39e0b25605ec8e5f70969833adafb`. |
| Integrity and review | Original fixtures/goldens and submodule unchanged. README/docs local file links and `git diff --check` pass. Independent review findings are addressed and covered by regression tests. |
| Publication and hosted CI | Published as `8e902c2` to `origin/main`. [Windows/Linux CI run 34156176185](https://github.com/Azaril/poe-optimizer/actions/runs/34156176185) passed both jobs, including formatting, Clippy, full tests and portable core/native WASM compilation. The following resume-document update changes documentation only. |

## Current repository and capabilities

- Branch: `main`; remote `origin` is `https://github.com/Azaril/poe-optimizer.git`
  ([repository](https://github.com/Azaril/poe-optimizer)). The user authorized publishing the
  project and supplied builds. The top resume point and checkpoint tables record the latest
  publication and validation state; dated earlier records retain their own commit evidence.
- [Workspace](../Cargo.toml): Rust 2024 / minimum 1.93; portable contracts, calculation
  kernels, authenticated portable game data, native document adaptation and shared import/materialization; a host search library
  and root CLI; and optional PoB supervision and native UTF-8 reference packages.
  [Core contracts](../crates/poe-optimizer-core/src/evaluation.rs) separate `CalculationBackend`
  from `EvaluationEngine`. Typed documents/options/results/coverage and identities expose no
  Lua or process API. The synchronous engine rejects missing/duplicate/undeclared metrics,
  incorrect units/schema, invalid finite metadata and changed backend/request identity.
  Backends enforce their own deadline; native calls are cooperative, PoB calls supervised.
  `EvaluationResult::validate_recorded()` also validates saved evidence, including optional
  passive observations and unused numerical fields. It cannot authenticate edited sources,
  reinterpret units against a new catalog or certify complete mechanics and game legality.
- [CLI](../src/main.rs): `import`, `evaluate`, `metrics`, `assess`, `search-experimental`
  and `benchmark-native` work in native-only builds. The default developer feature also adds
  `search-calibration`, `extract-tree` and hidden PoB workers. `--backend native|pob` selects
  evaluation and controlled search where the reference feature exists; native-only builds
  default to native and reject the unavailable PoB choice. Evaluation reports and the private
  PoB protocol use schema/version 2; controlled-search and benchmark reports use schema 1.
  `--options` supplies supported skill/encounter selection; repeatable `--metric` filters
  typed actor queries, and `--raw` retains backend-specific diagnostic attachments.
  `evaluate --objective` compiles goals against the selected catalog and retains their
  required measurements. `assess` uses saved metric schemas/context without a calculation or
  today's backend catalog. JSON/XML destinations must be new paths. Planner conversion is
  not implemented.
- [Objective assessment](objective-assessment.md): `ObjectiveSpec` compiles into a
  backend-neutral `ScoringPolicy` maximizing/minimizing one typed metric with a conjunction
  of strict/inclusive constraints. It checks units, finite thresholds, positive violation
  scales, unique IDs and contradictory bounds. Primary value/score, constraint observations,
  shortfalls, normalized violations and strict-boundary flags remain explicit. Strict
  equality failure can have zero distance while remaining violated. Missing/nonfinite
  measurements make the assessment unavailable and are never rewarded as numeric scores.
- [Native backend](native-backend.md): `native-poe2` admits restricted Spark/Mace documents
  across all eight classes and 23 ascendancy identities with zero or one ordinary entrance.
  It computes translated pipelines and supports
  immutable preparation, typed evaluation, exact supported exports and host clocks. Native
  controlled Mace search uses Rayon directly and checks fresh realization without PoB.
  General native build mechanics and the supplied complex minion build remain unsupported.
- Metric catalogs are backend-specific. [Native](../crates/poe-optimizer-native/src/lib.rs)
  declares nine player queries: Spark returns nine finite resource/resistance/hit values;
  Mace returns eight finite values plus explicitly unavailable `selected_average_hit` because
  the contract does not aggregate attack hands. [PoB](../crates/poe-optimizer-pob/src/metrics.rs)
  declares 15 definitions and 17 actor queries, including diagnostic EHP and five maximum-hit
  values and selected-player/minion hit metrics. Units and availability are explicit.
  Combined DPS and Full DPS remain diagnostics rather than objective metrics. Unsupported
  actor/metric requests reject according to the selected backend's capabilities.
- [Options/context](../crates/poe-optimizer-core/src/options.rs) retain requested selection,
  external configuration and observed conditions. [Coverage](skill-coverage.md) records
  resolved/unresolved skill provenance, selected action and Full DPS membership/counts.
  [Live passive evidence](passive-coverage.md) adds actual class/ascendancy, allocated nodes,
  point-count buckets, shared roots and effective switch provenance in the PoB reference
  path. Observation is distinct from complete allocation legality or native tree semantics.
- [Importer](../crates/poe-optimizer-import/src/lib.rs): portable exact UTF-8 XML/hash
  preservation, 1 MiB share-code and 8 MiB XML bounds, complete zlib validation, 100,000-node
  parser cap and no DTD/custom entity expansion. Predefined/numeric XML escapes work.
  [Preflight](../crates/poe-optimizer-import/src/preflight.rs) rejects missing/ambiguous
  evaluation identity while preserving the separate container-only import contract.
  [Controlled materialization](../crates/poe-optimizer-import/src/controlled_mace.rs) owns
  finite Mace catalogs and exact source changes. Old PoB paths are compatibility re-exports.
- [PoB backend](../crates/poe-optimizer-pob/src/backend.rs) remains an optional supervised
  reference. Its [runtime](../crates/poe-optimizer-pob/src/runtime.rs) embeds `mlua 0.12.1`,
  LuaJIT `2.1.1787165859` and static `luautf8 0.1.6`; it does not load upstream Windows DLLs.
  [Native-module notes](../crates/poe-optimizer-lua-utf8/README.md) record exact provenance.
  The supervisor enforces startup-inclusive deadlines, bounded I/O, request identity,
  retained stderr and cleanup after kill/reap. Post-load guards reject stale calculations;
  source verification/fingerprints require no Git subprocess in a worker.
- [Independent calibration](calibration-reference.md): two Spark goldens retain 19 raw
  outputs each; four Mace goldens retain 25 top-level outputs and nine main-hand values.
  Mace weapon preference reverses with Brutality, demonstrating a real interaction. Separate
  C/Lua hosts used bundled LuaJIT `2.1.1784580905`; the expected values were not generated by
  the Rust evaluator. Native full-build tests now compare against those unchanged references,
  with fresh optional-PoB comparisons extending supported cases. This remains shared-source
  parity rather than an independent game model or legality certificate.
- [Native calculations](native-engine.md) include six numerical helpers, numeric modifier
  aggregation, Condition/ActorCondition contexts, multiplier programs and ordered PerStat/
  StatThreshold dependencies, plus complete admitted Spark/Mace numerical pipelines with shared
  class attributes and nine explicit ordinary-entrance modifier fields.
  Source-executed numerical and XML compatibility tests cover these slices and the shared import boundary. FLAG-derived conditions, broader tags,
  real modifier extraction and general build pipelines remain explicit gaps. The numerical
  engine has no production Lua/I/O/scheduling dependency; the native adapter adds portable
  parsing, contracts and injected timing. WASM compilation does not establish browser
  execution or speed. [Benchmarking](native-backend.md#fixed-input-throughput-benchmark)
  measures prepared or document typed API throughput, including result and accounting cost.
- [Tree extraction](tree-data.md) preserves pinned topology, shared roots, reverse-listed
  edges and automatic override provenance; 14 dangling connections stay explicit.
  [Authenticated projection](tree-projection.md) admits selected ordinary Normal nodes with
  owner sets and a graph containing only included endpoints. Unsupported selected mechanics
  reject. The portable data crate owns these consumers and the authenticated class/entrance
  bundle; native calculations now apply the supported entrance effects. Broader passive
  effects and class/passive search materialization remain incomplete.
- [Candidates](candidate-model.md) and [search](search-kernel.md) provide finite rules,
  all-dimension locks, bounded parallel evaluation, proposals and fresh verification.
  `search-calibration` retains four immutable PoB sources; the [experimental command](experimental-search.md)
  composes source-preserving Mace choices with native or reference evaluation. General joint
  mutation/legality, shared memory admission, recovery and GUI remain unfinished.
  [objective.toml](../examples/objective.toml) is future notation; current policy input is JSON.
- [CI](../.github/workflows/rust.yml) checks the full development workspace on Windows/Linux
  and portable compilation. Native-only tests/dependency checks ensure deployment excludes
  the optional reference stack. Actual local/hosted outcomes belong in the current checkpoint
  evidence, not inferred from workflow configuration or older successful commits.

## Delivery plan and gates

Milestone IDs are stable references for review notes. A milestone is complete only when its
acceptance evidence is recorded below. M1 and M2 may overlap after the evaluator interface
is concrete; real-build recommendations depend on both.

| Milestone | Status | Deliverable and acceptance gate |
| --- | --- | --- |
| M0: bootstrap and alignment | Complete | Rust scaffold, pinned submodule, end-state design, source/prior-art review, preserved source fixture, and this implementation record. Scaffold validation passes; this does not establish evaluator correctness. |
| M1: evaluator and fixtures | Shared boundary, native class/entrance Spark/Mace pipelines, portable data and optional reference hosting implemented; broader legal mutation/mechanic coverage outstanding | Bounded import, typed metrics, native evaluation with cooperative deadlines, optional supervised PoB parity, controlled mutations across all six dimensions, export/re-import checks and independent calculation-state evidence. Complete the checklist below. |
| M2: reusable core and synthetic joint search | Candidate/lock contracts, synthetic joint search, deterministic parallel evaluation and bounded verification implemented; events, richer operators and shared resource management outstanding | Candidate/domain/lock models, metric policies, coupled mutations and repair, Rayon/job APIs, shared budgets and events. Match exhaustive tiny domains, escape coordinated-change traps, preserve multiple locks, and verify deterministic one/many-worker results plus cancellation/dedup/accounting. |
| M3: first usable joint optimizer | Not started | Integrate all six dimensions with multicore native evaluation and optional PoB parity; ship preflight, evaluation/comparison, search, saved-result reranking, recovery, verified exports, and offline reports. Meet the end-to-end gates below. |
| M4: broader catalogs and upgrade workflows | Not started | Extend mechanic/equipment/skill coverage and conditional upgrade/bundle ranking with explicit inventory, cost, and comparison semantics. Retain parity and lock guarantees. |
| M5: richer objective policies | Not started | Unit-checked expressions, composite and ordered priorities, soft preferences, Pareto selection, and explicit robust aggregation. Test policy-specific selection and preserve hard constraints. |
| Desktop GUI | Deferred until CLI/report contracts stabilize | Choose frontend; Tauri is a candidate. Reuse core jobs, results and comparison models. Verify CLI/GUI parity, responsive cancellation and native evaluator packaging; package optional reference workers separately. Does not depend on finishing every M4/M5 feature. |
| Native Rust calculation replacement | Active; all class/ascendancy identities and ordinary entrances supported by restricted Spark/Mace pipelines; portable data and native weapon/support search implemented | Complete injectable configuration/model work D1–D5 before class/passive materialization and explicit finite search rules, then broaden passive/modifier extraction, actor/skill coverage and full offence/defence while preserving differential parity and strict admission. Fixed-input API benchmarking exists; broader performance, optimizer quality and browser execution still need evidence. |
| PoE1 adapter | Later, separate track | Add a distinct versioned rules/data/evaluator adapter after PoE2 interfaces are proven; do not mix game identities or reuse PoE2 parity claims. |

Narrow passive/item/skill experiments are internal validation steps. The first usable release
must search classes, ascendancies, passives, equipment, support gems, and supporting active
skills together within explicit finite catalogs. It must support 1..N required skills and
1..N exact equipped item instances, independently of other locks.

### M1 checklist: native calculation and optional reference parity

- [x] Pin and inspect upstream loading, calculation, mutation, and export seams.
- [x] Decode the supplied PoB export without altering source bytes; record provenance,
      game/tree identity, and structural checks.
- [x] **M1.1 PoB reference runtime — implemented and validated on Windows/Linux.**
      Embedded LuaJIT and static UTF-8 boot the pinned wrapper and load the supplied build.
      Host callbacks supply paths, time, logging, noninteractive failures and controlled
      scratch writes. Dynamic native loading is disabled; each VM stays in its worker.
      No pin change or syntax overlay was needed. Source-manifest reproducibility,
      mismatch rejection and native provenance checks pass, along with hosted Windows/Linux
      startup, native and fixture tests. Exact dependency/source identities are recorded.
- [ ] **M1.2 Boundary — implemented and locally validated; scenario coverage remains limited.** Shared
      `CalculationBackend` / `EvaluationEngine` separate application contracts from hosting.
      CLI schema 2 and protocol 2 carry explicit skill/scenario options, versioned identity,
      typed measurements, coverage and optional diagnostic attachments. Fake/boxed backend
      contracts, native profiles and reference option/coverage tests exercise this boundary.
      Part/stat-set overrides and a complete resolved scenario model remain unimplemented.
      The synchronous engine has no worker pool or preemptive native execution boundary.
- [ ] **M1.3 Import and metrics — container/preflight complete; semantics partial.**
      PoB typed units/availability cover 15 definitions and 17 actor queries; native profiles
      declare nine player queries, with Mace average hit explicitly unavailable. PoB coverage
      classifies the three unresolved supplied entries and exposes selected ownership,
      Full DPS membership, manual/generated provenance and missing tree connections.
      No automatic repair is performed. Extend selected minion command/usage-model coverage
      and expand action/mechanic coverage before treating measurements as search objectives.
- [ ] **M1.4 Baseline parity — partial, with independent calibration evidence.**
      Two self-cast Spark scenarios retain independently hosted/extracted same-pin PoB
      outputs and prior production parity. Four Mace/weapon/support references now add a
      ranking-reversal interaction. The native class/entrance matrix adds 100 fresh PoB comparisons,
      including coupled ascendancy/entrance selections and exports. Native and optional reference comparisons use these
      unchanged expected values; current validation is recorded in the checkpoint evidence. Per-hand attack AverageHit remains explicitly unavailable in
      the typed catalog. Reference generation uses different host, extractor and LuaJIT
      builds with shared calculations, so this is not independent game-mechanics certification.
      Expand to minion/usage and survival cases, more equipment/support interactions, and
      actual allocated paths. Same-adapter A/B/A/export comparisons remain useful separate
      evidence; cached export values are not a reference oracle.

- [ ] **M1.5 Mutation parity — partial weapon/support structural profile implemented.** Shared
      materialization preserves exact payloads and external settings for native and reference
      checks against the four Q0 goldens. Q20/level cases extend native admission evidence;
      Pinnacle realization is reference-only while native Mace admits normal enemies.
      Class/ordinary-entrance evaluation parity now passes, but reusable class/passive
      materialization and its finite search catalog still need implementation.
      Exercise legal
      passive, item, support, supporting-skill, class and ascendancy changes separately
      and jointly. Preserve locks and verify export, point/resource accounting and effect
      removal against fresh materialization/reload. Include at least two required skills
      and two exact equipped-item locks; neither level-only changes nor the restricted Mace
      profile establish this gate.
- [ ] **M1.6 Isolation and failures — partial.** Parser/protocol failures, blocked-stdin
      timeout/reaping, scratch cleanup and fresh-worker A/B/A cover the reference path.
      Native prepared/document, serial/Rayon, timeout and export/reimport tests cover admitted
      profiles. Complete broader request-order and failure coverage. Persistent PoB reset/reload
      workers and memory-governed pools remain unimplemented; bounded fresh scheduling and
      cooperative library cancellation now exist. Enable reference worker reuse only after
      matching fresh-process results; native execution requires fresh independent calculation state.
- [ ] **M1.7 Measurements — harness implemented; broader benchmark program incomplete.**
      `benchmark-native` measures complete typed API calls in prepared/document modes with
      shared deadlines, bounded Rayon concurrency and explicit accounting. Current results
      belong in the top checkpoint evidence. Measure representative native and optional-PoB
      profiles separately, including preparation/result costs, memory and errors. Preserve
      enabled metric/profile declarations; do not require unimplemented EHP or confuse the
      historical standalone reference timings with a native scaling benchmark. Browser and
      realistic optimizer-quality measurements remain outstanding.

- [ ] **M1.8 Configurable native data — design recorded; implementation pending.** Complete
      D1–D5 above: owned versioned package/model, instance injection, externalized numeric data,
      host loading, identity isolation and unchanged default parity. The current portable tree
      crate is groundwork; global loader access and compiled numerical tables remain gaps.

The planner JSON export is preserved as auxiliary input/provenance. Raw PoB XML and PoB
share codes are runnable import paths; planner conversion follows only when its format
and missing semantics can be mapped faithfully.


### M2 and M3 acceptance details

M2 now has the initial objective/scoring boundary: one user-selected registered metric to
maximize/minimize and a conjunction of typed constraints, including strict/inclusive
comparisons. Unknown metrics, invalid units, contradictory bounds, nonfinite thresholds
and unsupported policies reject. Each constraint has an explicit positive violation scale.
The current API assesses supplied results and drives candidate ranking in the generic search
kernel and controlled CLI. It preserves metric schema evidence and keeps scoring
independent of game metric names; M5 extends policy implementations without redesigning
candidate search. Saved assessment is a new analysis of recorded values, not a new search.

M2 synthetic fixtures must include tiny exhaustively checked joint domains and a case where
every single-change improvement path stalls but a coordinated move wins. Exercise class
and ascendancy legality, tree connectivity/budgets, equipment multiplicity/slots, skill
compatibility/resources, and lock-preserving repair. Keep legal infeasible exploration
separate from verified feasible incumbents.

M3 must demonstrate:

- All six dimensions searchable in one real native-backed run with optional PoB parity, including cross-class
  alternatives. No silent freezing of a requested dimension or removal of a required lock.
- Complete-candidate verification and export/re-import checks independent of optimization
  cache/fast paths, with reserved evaluation/time budget and honest incomplete status.
- Shared CPU/memory limits across native Rayon tasks and any selected reference workers; bounded queues, in-flight
  deduplication, cancellation, capped retries and consistent attempt accounting.
- Preflight coverage/assumptions, common-baseline comparisons, grouped changes, constraint
  shortfalls, useful distinct alternatives, JSON run artifacts and offline HTML reports.
  Saved-result reranking is explicitly distinguished from a new search.
- Atomic recovery checkpoints with actual verification status. Evaluated-only recovered
  seeds are freshly validated before trusted reuse; a warm start is a new run/budget.
  Exact random/search-state continuation remains separate later work.
- Quality against random, greedy and alternating-domain baselines under equal evaluation
  budgets and several seeds. Measure fixed-candidate scaling and end-to-end quality at
  5, 15 and 30 minutes on recorded hardware, in explicit bossing and mapping contexts.
  Report each case, failures and variability; mapping uses documented proxies.
  There are no measured throughput, quality, or scaling results yet.

## Fixture ledger and technical unknowns

[Fixture README](../tests/fixtures/builds/README.md) and
[metadata](../tests/fixtures/builds/pobarchives-Dfz36mCq.metadata.json) preserve the full
source/hash record. The immutable decoded
[XML](../tests/fixtures/builds/pobarchives-Dfz36mCq.xml) is 49,642 bytes with SHA-256
`e3c0d0b40fa682260a1713acb03d52d720f4b769ac91b0501cbe2a84dc468194`.

The source is level 96 Sorceress / Disciple of Varashta, `PathOfBuilding2` root, tree
`0_5`. All 130 allocated IDs exist in the pinned tree. There are 19 skill groups and
16 referenced item records (13 equipment references and three tree jewels).
The planner title's patch 0.5.5 is a source claim; XML `targetVersion="0_1"` is not
independent evidence of the game patch.

| Unknown / limitation | Evidence so far | Next check |
| --- | --- | --- |
| Runtime portability | The selected embedded LuaJIT accepts `count += 1` and the leading `#@` lines; the pinned wrapper boots on Windows without vendor changes. | Hosted Windows/Linux checks pass; retain version/source pins on future updates. |
| Native ABI and source identity | Static `luautf8 0.1.6` links to mlua's vendored LuaJIT. Native/Unicode tests, eight vendored-file provenance checks, source-manifest reproducibility and mismatch rejection pass locally. | Hosted portability checks pass. Repeat provenance checks deliberately when updating pinned inputs. |
| Selected minion metric semantics | Main player action is `SummonSandDjinnPlayer`; selected minion action is `ExplosiveTeleportSandDjinn`, owned by group 1/gem 1. Typed non-finite chaos immunity is represented explicitly. Upstream initially declares average mode, then clears it after applying the self-cast cooldown: final `show_average=false`. The typed value is PoB's selected-action rate, not a rotation guarantee. | Add command/clone/usage-model coverage; do not present raw ~60,384.684 as independently validated sustained minion DPS. Add independent minion/action references. |
| Full DPS membership | Structured coverage shows no included supplied-build groups and raw player `FullDPS` zero. Spark calibration includes one group and compares its raw roll-up. | Keep membership/counts/contributions explicit; Combined DPS and Full DPS are not catalog objectives. Independent Spark agreement does not validate multi-skill uptime or aggregation. |
| Unresolved and granted skills | Three entries remain unresolved. Powered Zealot matches two Spectre monsters; Navira's Well and Kelari's Deception match minion command actions. Coverage distinguishes 15 manual, three tree-granted and one item-granted groups. | Preserve ambiguity and original input; exact relationships are hints, not repair or legality. Exercise supported action changes and coupled support/granted-skill effects without omission or double-counting. |
| Host behavior and topology | Fourteen startup messages identify absent targets in tree `0_5`; all are also absent in the four bundled older trees. Bidirectional reconstruction retains two ordinary entrances per start location and catalog ascendancy starts. Export filtering plausibly explains the dangling references without proving their meaning. | Keep all actual catalog classes in scope; validate routes, overrides, point budgets and requested-versus-realized state on the pinned graph. Preserve diagnostics and distinguish internal consistency from unverified game completeness. See [the investigation](tree-topology-investigation.md). |
| Legality, freshness and parity | Fresh-load guards and A/B/A/round trips pass at the previous checkpoint; Spark and the new Mace matrix provide independent-host reference evidence. Objective assessment evaluates typed numbers and structural replay validation checks records; neither certifies game legality or source authenticity. | Complete legal/coupled mutations across all six dimensions and failure/isolation coverage before reuse, fast paths or real-build recommendations. Preserve exact locks and reject silent pruning or changed identities. |

These are technical unknowns, not requests for the user to guess internal PoB IDs.
Ask for user input when a reproduced issue forces a product trade-off or the supplied
material cannot establish an intended build choice; present the concrete alternatives then.

## Validation evidence

### Historical objective, attack interaction and native modifier checkpoint — published and validated

Implemented on 2026-09-07, published through `86526ae` after `7dd2a09`. The combined run passed 107 tests, Clippy with
warnings denied, and portable core/native WASM compilation. Reference/source audits also
pass. The standalone objective/saved-assessment smoke and final formatting/Clippy rerun
also pass. Document checks and independent code review pass; both hosted jobs pass for `86526ae`. Historical results
below retain their original code identities.

| Check | Current evidence / remaining gate |
| --- | --- |
| Configurable core objectives | Ten new core tests cover configurable metric IDs, maximize/minimize, typed units/schema evidence, strict/inclusive constraints, contradictory bounds, shortfalls, unavailable/nonfinite primary/constraint inputs and malformed policies. Passed in the combined local run. |
| Recorded-result structure | Nine new core tests cover all measurements, versions/duplicates, finite tags, elapsed/context/options, all seven optional coverage numeric fields, coverage schema and live-engine reuse. No current catalog or Lua is required. Passed in the combined local run. |
| Objective CLI | Three new tests exercise direct evaluation assessment, fresh assessment of saved reports without a calculation process, required metrics with filtering, and rejected invalid inputs/corrupt recorded evidence. Passed in the combined local run. |
| Independent attack reference generation | Four committed Mace goldens were generated by the separate C/Lua attack host; two fresh runs per case matched 25 top-level outputs and the complete detail block exactly. Each records nine main-hand values and source/fixture/runtime/generator provenance. Original Spark artifacts remain unchanged. |
| Attack CLI regression | One new regression covers all four cases, fixture/source identity, selected skill/applied support data, 25 raw outputs, typed hit DPS and explicit unavailable attack average hit. The weapon preference reverses when adding Brutality I. Passed in the combined local run; this is not full legality or six-dimension mutation proof. |
| Native modifier differential tests | Six new tests cover the untagged BASE/INC/MORE/OVERRIDE subset against actual pinned ModStore/ModDB, flag/keyword helpers, source filtering, parent/query order, rounding/precision and unsupported input rejection. Existing five numerical-kernel tests remain. Passed in the combined local run. |
| Workspace Clippy and tests | `cargo clippy --workspace --all-targets --locked -- -D warnings` passed. `cargo test --workspace --all-targets --locked` passed: **107 tests, zero failures**, with the ignored subprocess helper explicitly invoked by its deadline test. `cargo fmt --all -- --check` and Clippy were rerun successfully after the final CLI help/comment edits. |
| Portable core/native libraries | `cargo check -p poe-optimizer-core -p poe-optimizer-engine --lib --target wasm32-unknown-unknown --locked` passed. Compilation is not browser execution, numerical browser parity or a performance result. |
| Topology investigation | Read-only source/exporter/graph inspection records actual class catalogs, both ordinary entrances per start, reverse-listed adjacency, absent references and normalization risks. No submodule modification or game-data completeness claim. Proposed fixture plan remains unimplemented. |
| Source and reference audit | Passed for all four new attack cases and both retained independent runs per case. Original fixtures and Spark artifacts remain unchanged. Source/golden identity checks pass. |
| Standalone objective and saved assessment | Passed: ignored `runs/m2-assessment-20260907-1d4afb62.json` and `runs/m2-assessment-20260907-1d4afb62-assessed.json` contain identical `ObjectiveAssessment` values from fresh calculation and saved reassessment. PoB EHP is `22562.9609905472`; fire/cold meet 75%, while lightning at 71% misses by 4 percentage points with normalized violation 0.4. Adapter fingerprint `8be35ce741554f294114ccdebd34602172f6125ff05faaf23fdc4d54e4493695`; evaluator elapsed `2591.3717` ms in this debug run. One observation is not a performance benchmark. Source pin/hash unchanged. |
| Document checks and publication | README/docs relative file links and `git diff --check` pass. Independent review found no outstanding issues in the scalar scope after recorded-result/schema-evidence fixes. Published through `86526ae`; [Windows/Linux CI run 34152960922](https://github.com/Azaril/poe-optimizer/actions/runs/34152960922) passed both jobs, including formatting, Clippy, full tests and portable core/native compilation. The following resume-document update changes documentation only. |
| Joint optimization / scaling / native whole-build backend | Not implemented or measured. New assessment and coupled reference cases do not establish search quality, throughput, full native parity or browser viability. |

The [objective notes](objective-assessment.md), [attack reference notes](calibration-reference.md),
[native coverage](native-engine.md), and [topology investigation](tree-topology-investigation.md)
record current contracts and limits. Diagnostic saved assessment retains backend/context/coverage
and exact assessed measurement schema versions. Its validation cannot authenticate an edited
report, and no current PoB catalog is substituted for recorded metadata.

### Historical contracts, calibration and native checkpoint — published and validated

Recorded on 2026-09-07 on Windows/x64 for published code checkpoint `7dd2a09`.
Preserve the historical results below as evidence for their own commits. Local validation
is complete; hosted results are recorded below.
The final Lua edit before the standalone smoke was a comment documenting cooldown behavior;
formatting/Clippy and the smoke were rerun after it.

| Check | Result / limit |
| --- | --- |
| Core shared contract suite, `cargo test -p poe-optimizer-core --test engine_contract --locked` | **11 passed.** Fake alternate/boxed backends exercise custom catalogs, complete actor queries, explicit unavailable/non-finite values, unit/schema checks, capability rejection, identity/options echo, JSON-safe metadata and typed timeout/failure propagation. |
| Independent CLI calibration, `cargo test --test calibration --locked` | **1 passed, covering both Spark scenarios.** Each fresh CLI process matches all 19 independent raw values, source/fixture identity, typed finite units, absent-minion availability, effective encounter and selected Spark coverage. Absolute tolerance `1e-8`, relative tolerance `1e-9`; applied as `max(absolute, abs(reference) * relative)`. |
| Standalone reference generation, `pwsh -File scripts/reference-calibrate.ps1` | Separate MSVC C driver plus bundled DLLs produced the two committed reference JSONs. Two independent fresh runs per fixture reproduced all 19 metrics exactly. Runtime/compiler/source/fixture/generator hashes and assumptions are recorded in [the reference notes](calibration-reference.md). No Rust evaluator generated expected values. |
| Native numerical parity, `cargo test -p poe-optimizer-engine --locked` | **5 passed.** Six functions compare against actual pinned upstream Lua using source hashes, boundary grids, interpreted/warmed execution, exceptional-value classification and positive-domain properties. The crate remains a resolved-input numerical slice, not a full evaluator. |
| Portable core and native libraries, `cargo check -p poe-optimizer-core -p poe-optimizer-engine --lib --target wasm32-unknown-unknown --locked` | Passed. Production library has no Lua/C/OS dependency. No browser runtime, bindings, scheduler or browser numerical/performance test has run. |
| Options/coverage integration suite | **7 passed.** Exact group/player/minion selection, mapping overrides, capped resistance units, invalid selectors/metrics/fields, nonfinite Config numbers, DamageOverTime/incoming-hit mismatch, unresolved/provenance/tree coverage and classified infinity. MAIN action ownership is checked against the authoritative actor list, not PoB's mutable UI display list. The minion's final average flag is correctly false after cooldown adjustment. |
| `cargo fmt --all -- --check` | Passed after all code edits. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed with warnings denied. |
| `cargo test --workspace --all-targets --locked` | **78 passed**, zero failures; one ignored subprocess helper is explicitly invoked by the deadline test. Includes the original fixture A/B/A and export/reimport regressions, 11 core contract tests, 5 native parity tests, and 5 new preflight tests. |
| Sources, artifacts and documentation | Read-only audit matched all 1,082 source entries and all 16 available reference artifact hashes. Both independent runs match each golden's 19 values exactly. Original user fixtures retain exact hashes/lengths, submodule remains clean; README/docs relative file links and `git diff --check` pass. |
| Publish and hosted Windows/Linux CI | **Passed for `7dd2a09` on both Windows and Linux:** formatting, Clippy, full tests, and core/native WASM compilation. [Hosted run 34150698841](https://github.com/Azaril/poe-optimizer/actions/runs/34150698841). The subsequent living-document update changes documentation only. |
| Standalone CLI smoke | `runs/m1-contracts-20260907-dd6640a0.json` and `.xml` (ignored) contain schema 2, 17 typed measurements, exact selected SandDjinn action and raw diagnostic attachment. Adapter fingerprint `bf6a6c2f134812e9f738a1f0c4581472a5130d239a85d48e1a169e16c261e292`. This debug run took 2,582.5707 ms inside evaluation, including source verification/startup; one observation is not a benchmark. `metrics` emits 15 definitions without launching Lua. |
| Optimization, native speedup/scaling and browser execution | Not implemented or measured. No throughput, quality, scaling or browser-runtime claim follows from parity and compilation. |

The PoB source remains `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, source-manifest hash
`8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`.
Current production hosting is mlua `0.12.1` / LuaJIT `2.1.1787165859` / static UTF-8
`0.1.6`; the independent reference uses bundled LuaJIT `2.1.1784580905` on x64. The
Spark comparisons establish host/extractor parity for those shared PoB calculations.
Native kernel differential checks translate the pinned implementation; neither kind
of evidence independently certifies game mechanics.

### Historical M1 evaluator checkpoint — published and validated

Recorded on 2026-09-07 on Windows/x64 with Rust 1.93. Evaluator commit
[`71c0af7`](https://github.com/Azaril/poe-optimizer/commit/71c0af7a8813776036b4e9750f4ac713bb52a832)
is published; both hosted Windows/Linux jobs passed.

| Check | Result / limit |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed. |
| `cargo test --workspace --all-targets --locked` | **47 passed**: 13 importer, 9 preflight, 6 source, 2 runtime, 11 supervisor, 2 native and 4 CLI tests. One ignored subprocess fixture is explicitly invoked by the timeout test. |
| Fresh-worker isolation and export | A/B/A changes level 96 → 95 → 96. Export/reimport compares all finite actor metrics using `1e-8 * max(abs(expected), 1)` tolerance, alongside actor/summary checks. This is same-adapter evidence. |
| Container/evaluator separation | Empty `PathOfBuilding2` imports as a valid container but evaluation rejects it. Unsupported target versions reject before returning default/startup metrics. |
| Output safety and supervision | Output aliases reject before evaluation; bounded I/O, blocked-stdin deadline/kill/reap and scratch cleanup tests pass. |
| Sources and native provenance | Original source/decoded fixture hashes unchanged; pinned submodule clean. All eight vendored native/header file identities match provenance. Source manifest reproduces and wrong-pin rejection passes. |
| Hosted CI | [Run 34147041293](https://github.com/Azaril/poe-optimizer/actions/runs/34147041293) passed on both `windows-latest` and `ubuntu-latest`: checkout, Rust setup, formatting, Clippy and workspace tests. |
| Independent PoB reference / joint mutation parity | Not established; independent calibration and all-dimension mutation coverage remain M1 work. |

The standalone debug observation used `mlua 0.12.1`, LuaJIT `2.1.1787165859`
(`luajit-src 210.7.3+1ee778a`), and static `luautf8 0.1.6`, on an AMD Ryzen 9 9950X3D
with 16 physical cores, 32 logical processors and approximately 64 GiB RAM. Evaluator
elapsed time was **2,603.4796 ms**; CLI wall time was **2,996.6718 ms**, including source
hashing, bootstrap, calculation and export. Filesystem-cache state was uncontrolled;
this is one observation, not a cold-disk, throughput or scaling benchmark.

The sample retained raw minion `TotalDPS = 60384.68419307677`, energy shield `9403`,
player `FullDPS = 0`, three unresolved-skill warnings, explicit non-finite output keys,
and 429 bytes of diagnostics. Agreement with source-cached DPS is not independent parity.
Local ignored artifacts: `runs/m1-evaluator-20260907-b9f5b961.json` and
`runs/m1-evaluator-20260907-b9f5b961.xml`; these are not committed fixtures.

Recorded artifact identities:

- Source hash: `8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`.
- Adapter hash: `250468f42ebc774348a986bf573fb7bcff211fd16862ad9e03026f61563945fd`.

The earlier 25-test pass predates review changes and is superseded by the 47-test result.

### Historical scaffold baseline

Recorded on 2026-09-07 for baseline `72bf6a7` and its scaffold ancestors:

| Check | Result / limit |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --all-targets --locked -- -D warnings` | Passed on Windows with Rust 1.93.0. |
| `cargo test --all-targets --locked` | Passed, **zero tests**; verifies only the starter target builds. |
| CLI help/version/invalid argument | Checked; invalid arguments exit with code 2. |
| Fixture decoding and structure | Source/output hashes checked; bounded complete zlib decode and XML parsing; 130 tree IDs checked. This was an investigation, not an implemented importer test suite. |
| Submodule and documentation | Recorded pin clean; local document links and `git diff --check` passed at the prior checkpoint. |
| Lua evaluation, mutation/export parity, search/scaling | Not run at this historical scaffold baseline. |
| Hosted CI | Workflow configured; no hosted run result recorded at this checkpoint. |

The earlier living-document checkpoint changed documentation only. Local file/heading links and
git whitespace checks were validated; fixture bytes and the submodule pin were preserved.
Rust checks were not repeated because code, manifests and workflow were unchanged.

For each implemented change, record the appropriate validation and actual outcome here or
link a committed result summary. Do not count future checklists as evidence. Generated
benchmark artifacts belong under ignored `runs/`; commit concise reproducible summaries
and fixture/config identities when they become available.

## Checkpoint protocol

At session start, read the resume point and inspect the working tree. At each coherent
feature completion, runtime experiment that changes direction, and before session handoff:

1. Update milestone/task status to match the code and observed behavior.
2. Record commands, outcomes, runtime/data versions and relevant evidence paths. Preserve
   failures and unresolved questions; distinguish untested hypotheses from reproduced blockers.
3. Replace the resume point with the next small executable task and any prerequisites.
   Note unfinished files or local-only setup that the next session must preserve.
4. Update the design only if scope, architecture, invariants or public contracts changed.
   Keep agreed end-state capabilities even while implementation is incomplete. Update
   integration evidence if a new pin or experiment changes an earlier finding.
5. Keep README capability claims accurate, review the diff, and commit a coherent checkpoint
   when appropriate. Record the commit on the next update or identify the checkpoint by
   its dated label; a document cannot contain its own final Git hash.

### Checkpoint history

| Date | Reference | Outcome |
| --- | --- | --- |
| 2026-09-07 | `fbfe216` | Initialized Rust scaffold, PoB submodule and initial design. |
| 2026-09-07 | `7926b60` | Clarified configurable objectives and extensible scoring. |
| 2026-09-07 | `c278da5` | Designed multicore execution and reusable CLI/GUI boundaries. |
| 2026-09-07 | `72bf6a7` | Reviewed WoW prior art; confirmed joint scope and cross-class search; preserved/decoded supplied minion fixture. |
| 2026-09-07 | `8eb2e23` | Separated end-state design from delivery tracking; recorded M1 resume point and evidence; connected the user-created GitHub repository. |
| 2026-09-07 | `46473e3` | Recorded explicit user authorization to push the project and supplied fixtures to GitHub; cleared the pending publishing question. |
| 2026-09-07 | `1be92d7` | Preferred mlua hosting inside isolated Rust evaluator workers. |
| 2026-09-07 | `71c0af7` | Added bounded import, evaluator preflight, embedded/native hosting, source verification, fresh-worker supervision and CLI evaluation/export. Formatting, Clippy, 47 local tests and hosted Windows/Linux CI pass; independent parity and optimization remain outstanding. |
| 2026-09-07 | `7dd2a09` | Added backend-neutral contracts, CLI schema/protocol 2, typed catalog/options/coverage, two independent Spark scenarios and the first six native numerical functions. All 78 local tests, formatting, Clippy and core/native WASM compilation pass. Both hosted Windows/Linux jobs pass in run `34150698841`; broader M1 coverage remains open. |
| 2026-09-07 | `bb08697` + `86526ae` | Added configurable scalar assessment and saved-report validation, four independent Mace interaction goldens, native untagged numeric modifier aggregation and a source-level tree topology investigation. All 107 local tests, formatting, Clippy, core/native WASM compilation and standalone objective/reassessment smoke pass; source/golden audit passes. Document checks and independent code review pass; published through `86526ae`, with both hosted Windows/Linux jobs passing in run `34152960922`. Canonical all-dimension candidates, locks and joint search remain next. |
| 2026-09-07 | `8e902c2` | Added canonical all-dimension candidates/locks with shared physical-node ownership, bounded host search and fresh verification, a four-build PoB calibration-search CLI and parity-tested native conditions. All 154 local tests, Clippy, formatting, WASM compilation, standalone search/export and review checks pass; hosted CI evidence is in the checkpoint table above. |
| 2026-09-07 | `55df9d5` | Added structural weapon/support mutation, supplied-problem CLI search, coordinated discrete proposals, isolated pinned-tree snapshots and native multiplier programs. All 188 local tests, Clippy, formatting, portable WASM and documented example checks pass; hosted CI evidence is in the checkpoint table above. |
| 2026-09-07 | `a09c406` + `115a902` | Added native Spark/Mace build pipelines, switchable backends, native-only packaging, Rayon controlled search, fixed-input throughput measurements and live passive/tree parity groundwork. All 260 workspace tests, 16 native-only CLI tests, formatting, lint, dependency checks and four portable WASM library builds pass locally; both hosted Windows/Linux jobs pass in run `34166302194` on main. Full native game coverage remains incomplete; class/passive calculations are next. |
| 2026-09-07 | `fa5136b` + `36fc552` | Added portable authenticated game data, native class/ascendancy identity and ordinary-entrance evaluation, source-alignment guards and fixed-input multicore measurements. All 277 workspace tests, 16 native-only CLI tests, formatting, lint, dependency checks and five portable WASM libraries pass. One hundred fresh PoB evaluations show zero observed numeric difference; both hosted Windows/Linux jobs pass in run `34170105192` on main. Next: source-preserving class/passive materialization, explicit search budgets and equipment requirements; full native game coverage remains incomplete. |
| 2026-09-07 | Injectable game-data design (after `2675ffb`) | Recorded the user-directed configuration/model seam, immutable compiled-data injection, instance/prepared identity, strict compatibility and D1–D5 migration gates. Reprioritized this work before broader class/passive search. Documentation only; runtime injection and external package selection remain unimplemented. |

### Hosting decision checkpoint

On 2026-09-07 the user preferred `mlua` for Lua hosting/interaction where feasible. The
design and M1 resume point now prioritize a Rust worker embedding LuaJIT. Direct LuaJIT
execution is a diagnostic/reference fallback. Process isolation, shared budgets, fresh
verification and all joint-search requirements remain in force.

Official [mlua 0.12.1 feature metadata](https://raw.githubusercontent.com/mlua-rs/mlua/v0.12.1/Cargo.toml)
confirms `luajit` and `vendored` support and Rust 1.88 as its declared minimum, below this
project's Rust 1.93 minimum. At that decision checkpoint this was source-level feasibility evidence, not a successful
local build, ABI validation or PoB calculation; subsequent results are recorded above. Read-only inspection also identified the
Windows native-module dependency and bootstrap requirements recorded in the
[integration notes](pob-integration.md#embedding-bootstrap-checks). Pin the dependency/runtime selection
when implementing M1. No Cargo dependencies or runtime code changed at this checkpoint.

## Decisions still deferred

No answer is needed before M1. Choose the project's distribution license before a public
release; confirm the desktop framework and packaging before GUI work; choose acquisition
data sources before trade/upgrade ingestion. Benchmark-specific metrics and usage profiles
must be documented when those fixtures are made runnable, without turning their choices
into mandatory goals for all users.
