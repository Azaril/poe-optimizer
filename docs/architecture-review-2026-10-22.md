# poe-optimizer: Architecture & Implementation Review (2026-10-22)

**Audience:** the agent actively developing this repository.
**Scope:** read-only review of the whole project at `main` (`fa49fcea`) plus the uncommitted working tree. No code was changed.
**Method:**
- Two broad survey passes: architecture/hot paths, and testing/docs/process.
- Seven deep-dive reviews, one each for:
  - optimizer/perf
  - engine correctness
  - data layer
  - import/PoB
  - CLI/errors/concurrency
  - extensibility
  - build/release
- Every deep-dive finding went to an **adversarial verifier**, which re-opened the cited code and confirmed, adjusted or refuted it. The severities below are the verified ones.
- The load-bearing claims were spot-checked again by hand before writing.
- `file:line` references are relative to the repo root.

How to read this:
- **§1 is the project map**, the shared mental model.
- **§2 is the prioritized review.** Work top-down.
- **§3 is the data-format / DSL proposal**, one finding among many but a big one.
- **§4 is a suggested sequence of work.**
- **§5 lists the strengths to preserve.**

---

## 0. Executive summary

1. **There is no optimizer yet.** The search crate (890 lines) has exactly one production caller, `search-calibration`. That command scores at most 64 user-supplied PoB XML files (`src/catalog_search.rs:27`, `max_rounds: 1`), and each evaluation runs in a fresh PoB/LuaJIT process. Nothing connects the native evaluator to search. The README itself says *"none of the five supplied real builds yet completes natively"* (`README.md:29-31`).
2. **Verification work dominates the codebase.** There are about 183k non-test lines of production Rust, against about 682k lines of integration tests in about 792 test binaries, about 920k words of prose, and 293 MB of tracked JSON evidence.
   - Most tests check that data, evidence and receipts agree with each other, including **SHA-256 pins of production `.rs` files**. They mostly don't test behaviour.
   - A full `cargo test --no-run` cannot finish on the dev machine (the paging file is exhausted at 113 s).
   - CI needs 16 test shards and already records disk exhaustion.
3. **The data pipeline is fragile at the release level.**
   - The latest validated data release exists only in the gitignored `runs/` directory.
   - The tracked `current/` directory is a historical baseline, 22 operation versions behind.
   - The release is **3.7% under a hard 64 MiB ceiling** that every load path enforces. About 25-30% of those bytes are one repeated namespace object.
4. **The core of the native evaluator is well engineered**: typed failure lattice, finite-only numerics, deterministic `BTreeMap` iteration, an explicit-stack VM, and a compiled indexed effect graph. But three things block it from serving an optimizer:
   - candidate values are baked into each compiled plan as constants, so every candidate pays a full recompile;
   - every value carries three heap `String`s;
   - the search contract forbids caching.
5. **Layering has drifted from the ADR.**
   - The `legacy` Lua-emulation stack and a 26.7 MB embedded `game-data.json` ship in every binary, despite the README.
   - `import` depends on `engine` only for legacy-gated modules.
   - About 69% of `import` is really the offline data-release toolchain.
6. **The data format** (user concern):
   - **The XML is not the project's data format.** It is the Path of Building interchange format, used for input only (21 fixtures), and it has to stay.
   - **The project's own data is JSON, and it really is unreadable.** Mechanics are referenced by ~7.5k opaque ids (`def.000000000000335c`) that have no names anywhere. The rule language is a hand-written SSA IR.
   - Example: **7 lines of upstream Lua became about 2,200 lines of JSON**, plus about 1,500 lines of tests.
   - **Recommendation:** keep the checked SSA IR as the compile target. Add four things on top:
     - (a) a symbol table of human names;
     - (b) a small purpose-built **formula DSL (`.rule`)** that lowers to the existing IR;
     - (c) a compact namespace-hoisted wire form plus an interned binary runtime package;
     - (d) declarative packet manifests (KDL).
   - Details in §3.

---

## 1. Project map

### 1.1 What it is

The project is an experimental, constraint-driven build optimizer for **Path of Exile 2**, written in Rust (edition 2024, toolchain pinned to 1.98.1).

The intended architecture (`docs/domain-architecture.md:24-30`):
- The project owns its build, definition and rule schemas.
- Game mechanics are compiled offline from a pinned **Path of Building (PoB) PoE2** checkout into an "owned" data package.
- A native Rust engine evaluates builds against that package.
- A search layer explores build variations under constraints.
- PoB itself (Lua, run via mlua/LuaJIT) is an **optional parity oracle**.

Project history:
- Single author (agent-driven): 587 commits between 2026-09-07 and 2026-10-21, about 22 per active day.
- About 5.4M lines inserted against 0.29M deleted.

### 1.2 Components and responsibilities

| Crate / area | Responsibility | Non-test src LOC | Test LOC | Test bins |
|---|---|---|---|---|
| `crates/poe-optimizer-core` | Shared types: owned definition IDs (`DefId<K>`), rule IR (`RuleExpression`, `owned_rules.rs:999`), schema/coverage (`DeclaredSet`), objective/scoring, legacy `Candidate`, worker protocol | 17.7k | 17.9k | 32 |
| `crates/poe-optimizer-data` | Embedded legacy package (`data/game-data.json`, 26.7 MB, `include_bytes!` at `src/game_data.rs:31`), class tree, decoders for owned packages | 27.3k | 31.8k | 59 |
| `crates/poe-optimizer-engine` | **Owned evaluator** (`owned_rules` compile/execute, `owned_plan` graph, timing, resistance; about 21k lines) **plus** the legacy Lua-semantics stack (`parser_program` 9.7k, `modifier_parser`, `lua_pattern`, `lua_number`, Spark/Mace; about 20k lines) | 30.0k | 94.0k | 114 |
| `crates/poe-optimizer-import` | PoB XML/share-code decoding (`lib.rs:134 decode_build`, roxmltree), XML gates (`xml_compat.rs`, `source_xml.rs`), item-text state machine (`item_loading/machine.rs`, 3.1k lines), `owned_normalize` (build → owned draft) **plus** the offline release toolchain (`owned_release*`, `owned_successor`, `owned_recipe*`, `owned_*_catalog`; about 65k lines) | 77.3k | 100.6k | 94 |
| `crates/poe-optimizer-pob` (feature `pob`) | mlua/LuaJIT host for vendored PoB, process supervisor (`supervisor.rs`), source-manifest verifier (`source.rs`), offline extractors | 24.3k | 154.7k | 200 |
| `crates/poe-optimizer-search` | Generic deterministic batch beam search on Rayon (`lib.rs`), discrete mutation space (`discrete.rs`) | 0.9k | 2.0k | 5 |
| `crates/poe-optimizer-lua-utf8` | C UTF-8 module statically linked into LuaJIT | ~0 | – | 0 |
| `src/` (CLI `poe-optimizer`) | 51 clap subcommands (35 default, 16 `pob`-gated, 3 hidden workers). About 30 of them are offline data-authoring commands (`compile-owned-*`, `extend-*`, `publish-*`) | 5.9k | 281.6k | 286 |
| `data/owned/poe2/3887ae68/` | 269 "packet" directories (one per mechanic or feature), `current/` (historical tree baseline), `releases/`, `import/`. About 1,714 files, 293 MB | – | – | – |
| `vendor/path-of-building-poe2` | PoB submodule pinned at `3887ae68`, the source of truth for extraction and the oracle | – | – | – |
| `scripts/` | 11 Python exporters/checkers (`export-owned-*.py`, `check-owned-boundaries.py`), PowerShell/Lua/C reference drivers | 7.4k py | – | – |
| `runs/`, `local/` (gitignored) | Agent working area: about 10k entries, 22 worktrees, **and the latest data release endpoint** | – | – | – |

Dependency graph: `core ← data ← engine ← import ← pob`.
- `search` depends only on `core` (plus rayon).
- The CLI depends on core, data, engine, import and search, plus pob when built with the `pob` feature.
- `data` and `engine` both have `default = ["legacy"]`.

### 1.3 Data and control flow (as implemented today)

```
                    OFFLINE AUTHORING (per mechanic, agent-driven)
 vendor/PoB Lua ──► scripts/export-owned-*.py ──► packet dir (JSON: schema defs, SSA rule programs,
                                                   queries, policy, source-vectors, README, review.json)
                                         │
                       Rust publishers (extend-owned-*, publish-owned-successor, test "stage()" fns)
                                         ▼
          release N  ──apply packet──►  release N+1  ── … ×~260 ──► latest endpoint (runs/…, UNTRACKED)
          (each step byte-exact against its predecessor; content-addressed manifests)

                    RUNTIME (one CLI invocation = one evaluation)
 PoB XML / share code ─► import::decode_build ─► ImportedBuildInstance
        ─► normalize-owned (10 explicit artifact paths) ─► owned draft / request
        ─► check-owned-draft ─► evaluate-owned --release DIR | --schema --rules --routing --metrics
              load_release: read+hash+decode ALL artifacts, re-assemble, byte-compare
              CompiledRulePackage ─► OwnedEffectPlan::compile(request)  (≈20 builder passes, digests)
              ─► indexed graph execute (per-worker scratch) ─► OwnedMetricReport (JSON to stdout)

                    "SEARCH" (pob feature only)
 ≤64 XML files ─► search::search(max_rounds=1) ─► per candidate: spawn `poe-optimizer __worker`
        ─► verify PoB tree (217 KB manifest, SHA every Lua file) ─► Lua::unsafe_new ─► load PoB ─► calc ─► exit
```

### 1.4 How a user actually runs it

1. `cargo build --release` from a full clone: 352 MiB tracked plus the 439 MB PoB submodule. MSVC is required on Windows. There is no binary release and no license.
2. `poe-optimizer import <build.xml|share-code>`
3. `poe-optimizer normalize-owned --policy … --registry … --definitions … --mapping … --roles … --rewards … --items … --item-source … --queries … --output DIR`. All ten files live in `data/owned/poe2/3887ae68/current/`, but there is no `--bundle` shortcut.
4. `poe-optimizer check-owned-draft --selection … --definitions … --owned-output …`
5. `poe-optimizer evaluate-owned --input REQUEST --release DIR`
   - The tracked release cannot evaluate the latest mechanics.
   - The latest endpoint has no `evaluation` section, so `--release` fails on it (`src/owned_metrics.rs:235-237`).
6. Optional, with `--features pob`: `evaluate` (PoB oracle), `search-calibration` (ranks ≤64 XMLs).

### 1.5 Build, test and tooling

- **CI:** `.github/workflows/rust.yml`.
  - Validation job: fmt, three clippy passes, `check-owned-boundaries.py` (a depfile closure check, which is good), a cargo-tree guard keeping Lua out of the default CLI, and wasm32 checks.
  - Test matrix: 2 OS × 8 suites, with CLI tests split by `.github/scripts/invoke-cli-shard.ps1`.
- **Dependencies:** 93 locked crates with **zero duplicate versions**, and every cargo call uses `--locked`.
- **Tests:** about 5,914 integration `#[test]`s plus 809 inline. There are 2,079 `#[path]` includes of support files, compiled again into every binary that includes them. 393 test files read `data/owned` directly.
- **Docs:** 179 markdown files and about 720k words. `docs/implementation-history.md` alone has 266k words. There is no index.

---

## 2. Prioritized review

Severity labels show the verified severity. **P0** = direction-level; address before adding more mechanics. **P1** = structural; schedule soon. **P2** = significant cleanup. **P3** = hygiene and nits.

### P0-1: No native optimization loop exists *(critical)*

**Evidence:**
- Search is used only by `src/catalog_search.rs:13,364`, with `MAX_ENTRIES = 64` (`:27`), `max_rounds: 1` and `ExecutionKind::ExternalProcess` (`:243`).
- `CandidateEvaluator` returns `MetricMeasurement`s (`crates/poe-optimizer-search/src/lib.rs:104-108`). Only the PoB host and core produce these. The native path produces `OwnedMetricReport` (`src/owned_metrics.rs:330-332`), and no adapter connects the two.
- `DiscreteSpace` (`discrete.rs`) has no production caller.
- `build_search.rs` and `mutation_search.rs` were deleted in 20614ca3.
- `docs/action-timing.md:70-71` still documents the removed `search-build` and `benchmark_assembly`.

**Impact:**
- The product's reason to exist cannot run.
- The 5-30 minute optimization goal (`docs/design.md` §Search strategy) is untested.
- With no throughput benchmark, nobody knows whether import, compile, execute or reporting dominates.

**Recommendation:** make "one real build, one real decision axis, native, end to end" the next milestone:
1. Get one of the five supplied builds to complete natively.
2. Implement a native `CandidateEvaluator` that maps `OwnedMetricReport` to `MetricMeasurement`, or teach the objective layer about owned `MetricDefId`s.
3. Choose one structured axis, for example the support gem per socket group, or one rune or jewel slot.
4. Run `DiscreteSpace` over that axis.
5. Add a release-mode benchmark that times request build, plan compile, metric-plan compile and execute separately (p50/p95 plus allocations). `crates/poe-optimizer-data/examples/profile_allocations.rs` already exists to build on.

### P0-2: Evidence ceremony and change-detector tests crowd out product work *(high)*

**Evidence:**
- **Volume:** test code is 2.9-3.7× production code. `tests/support` alone (188k lines) is larger than any production crate, and 1,254 `#[test]` fns live *inside* support files.
- **Source-text pinning:** `data/owned/.../generated-firebolt-dispositions/review.json:22-25` pins the SHA-256 and byte length of `crates/poe-optimizer-import/src/owned_normalize/generated_skill_inputs/dispositions.rs`, and `tests/support/owned_generated_firebolt_dispositions.rs:47-51` asserts it. Across `data/owned` there are 61 JSON files pinning 105 Rust source paths. Test files also hold 508 hard-coded 64-hex digests.
- **Shape-counting tests:** `tests/owned_incoming_critical_native.rs:35-38` asserts `schema.len() == 19`, `owners.len() == 2`, `queries.len() == 3`, `policy.len() == 2`. Lines 39-49 assert that two files the format forces to duplicate IDs still agree.
- **Ceremony vocabulary:** "evidence", "receipt", "review", "proof" and "accounting" appear about 11k times in tests, and 880 of 5,274 test-fn names contain such terms.
- **Build cost:** `cargo test --no-run` fails locally with E0786 / os error 1455 (paging file). `rust.yml:21-22,113-114` disables debuginfo and incremental builds after "disk exhaustion".

**Impact:**
- Every refactor of a pinned file fails tests until JSON receipts are regenerated. That is why the current working tree touches `review.json` next to every `.rs` edit.
- The tests freeze code instead of checking behaviour.
- The suite cannot be run locally, so the agent can no longer run its own safety net.

**Recommendations:**
1. **Delete source-file hash pins.** Provenance belongs in a generated report or commit metadata, not in assertions. If identity of a producer is needed, pin a **declared semantic version constant** bumped on purpose, or a digest of the producer's *output on fixtures*.
2. Replace shape-count assertions with invariant checks, for example "every declared query is resolvable" and "vectors replay".
3. Build **one data-driven packet harness**: point it at a packet directory and it loads `source-vectors.json`, builds the finite world generically, replays and asserts. That replaces about 1.5k hand-written lines per mechanic (`tests/support/owned_incoming_critical_native.rs:107-391` hand-builds a whole `BuildInput`/`Scenario`/`Recipe`).
4. **Consolidate test binaries to fewer than 30.**
   - Set `autotests = false` in each manifest.
   - Declare `[[test]] name="owned_cli" path="tests/owned_cli/main.rs"` with today's files as `mod` children.
   - Turn support code into ordinary modules, or a `poe-optimizer-test-support` crate, instead of 2,079 `#[path]` copies.
   - Use `cargo nextest --partition` instead of the custom PowerShell shard planner.
5. Put a freeze in place: no new `review.json`/receipt types until 1-4 land.

### P0-3: The latest data release is untracked, and `current/` is mislabeled *(high)*

**Evidence:**
- `data/owned/poe2/3887ae68/current/README.md:1-4` reads *"Historical tree publication baseline … not the current evaluator release"*.
- `docs/implementation.md:3134-3136` names `runs/owned-enemy-armour-component-publication-01/package` as the latest endpoint. `runs/` is gitignored (`.gitignore:2`).
- The tracked baseline is at `operations_version` v6; the endpoint is at v28.
- 148 tracked data files contain 410 `"runs/…"` references.
- `releases/README.md` reproduction starts from a `runs/` predecessor.
- CI runs no `--ignored` suites, so **the chain from v6 to v28 is never validated by CI**.

**Impact:**
- A fresh clone, CI or another contributor cannot evaluate against what the docs call current.
- Losing one local `runs/` directory means replaying about 260 packet applications by hand.

**Recommendations:**
1. Track the latest validated endpoint as a named release, `data/owned/poe2/<pin>/releases/<id>/`, or publish it as a CI artifact.
2. Rename `current/` to `tree-baseline/`.
3. Commit an **ordered replay manifest** (packet sequence plus the input/output digest of each step) so the endpoint can be rebuilt with one command and checked in CI.
4. Make tests read only tracked or CI-built endpoints.

### P0-4: The release is within 3.7% of a hard 64 MiB ceiling *(high)*

**Evidence:**
- `crates/poe-optimizer-import/src/owned_release.rs:114-116` sets max input, output and artifact bytes to `64*1024*1024`.
- `crates/poe-optimizer-core/src/owned_content.rs:11` sets `MAX_OWNED_CONTENT_BYTES = 64 MiB`, and `digest_owned` rejects any higher limit.
- One budget is shared by all files (`src/owned_release_cli.rs:316`).
- The endpoint measures 64,647,660 B against 67,108,864 B. Since the tracked baseline, schema has grown 14.9→32.2 MB and rules 2.4→17.5 MB.
- `docs/implementation.md:2045-2046` already noted the shrinking headroom, but nothing guards it.

**Impact:** a few more component packets will make `assemble`, `publish` and `evaluate` all fail with "canonical content exceeds".

**Recommendations:**
- Short term: separate the runtime budget (schema, rules, routing) from the authoring budget (mapping, roles, items, tree).
- Make the limit a policy knob.
- Add a CI size metric with an alert at 80%.
- Structural fix: namespace hoisting and compact `DefId` (§3, step 0) recovers about 25-30% at once.

### P1-1: Native evaluator throughput blockers *(high/medium)*

The execute phase is well shaped: an immutable `Arc` plan, precomputed topological order, index-based reads and per-worker scratch (`engine/src/owned_plan/graph.rs:633-760`). The surrounding design cancels most of that out:

1. **Candidate values are compiled in as constants** *(high)*.
   - `EffectOperation::GeneratedInput{value}` is documented as "a checked immutable request value" (`engine/src/owned_plan.rs:509-529`), and `ReadBinding::Constant` is at `graph.rs:104`.
   - Every request goes through `compile_inner` (`owned_plan/compile.rs:401-640`): `bind_owned_request` (`:424`, which digests the whole request), four `digest_owned` calls (`:451,:461,:468,:474`, each a serde_json→SHA-256 stream), discovery, and about 20 builder passes over about 25 `BTreeMap`s.
   - Compile cost is unmeasured but structurally far larger than execute cost.
   - **Fix:** split the plan into a structural *skeleton* and a *parameter vector* read through indexed fact slots. Hash only when the skeleton is rebuilt.
2. **The no-cache contract** *(medium)*.
   - `search/src/lib.rs:112` reads *"Every call must compute fresh (no hidden cache)"*.
   - **Fix:** change the rule to "results must be independent of cache state". Allow explicit, content-addressed caches (plan skeletons, prepared supports), and test that a cold cache and a warm cache give identical results.
3. **String-carrying values** *(medium)*.
   - `FiniteQuantity{value, unit: UnitDefId}`, where `DefId` holds 3 `String`s (`core/src/owned_definitions.rs:206-210, 317-320`).
   - Every arithmetic op clones the unit: `execute.rs:15-16` `FiniteQuantity::new(value, unit.clone())`.
   - A fact read clones 3 or more times along the chain `graph.rs:380` → `execute.rs:566` → `execute.rs:215`.
   - **Fix:** intern definition ids to `u32` handles at package-compile time. Make `NodeValue` `Copy` (`f64` plus unit index). Keep `String` `DefId` only at the wire and report boundary.
4. **Search loop** *(medium)*.
   - `for chunk in eligible.chunks(budget.jobs)` (`lib.rs:354`) puts a barrier on every batch.
   - Validate, propose and verify run serially.
   - `ExternalProcess` spawns one OS thread per candidate per chunk (`:378-383`).
   - **Fix:** use a bounded in-flight queue that integrates results by proposal sequence number, which keeps determinism.
5. **The PoB oracle** spawns a fresh process and tempdir for every evaluation and re-verifies the whole PoB tree each time (`pob/src/supervisor.rs:183-198`, `pob/src/runtime.rs:77,89`).
   - Keep PoB out of any inner loop.
   - For calibration, use a warm worker pool that verifies the tree once.
6. Minor: `CompiledRulePackage::evaluate` streams facts and results through serde→SHA purely as a size bound (`execute.rs:496,527`). Archive sort-on-insert (`lib.rs:444-446`). Per-effect scratch reset recomputes shared nodes (`execute.rs:571-590`).

### P1-2: The legacy stack ships everywhere and layering has drifted *(high)*

**Evidence:**
- `crates/poe-optimizer-data/Cargo.toml:10` and `crates/poe-optimizer-engine/Cargo.toml:11` both set `default = ["legacy"]`.
- The root `Cargo.toml` and `crates/poe-optimizer-import/Cargo.toml:9-10` take both crates with default features.
- `src/data_loading.rs:2-5` calls `bundled_snapshot()` unconditionally.
- So the "native-only" CLI (`README.md:203-209`; CI shard label at `invoke-cli-shard.ps1:103`) still ships the Lua-emulation engine and the 26.7 MB `game-data.json`.
- Import uses engine **only** for legacy-gated modules (`lua_pattern` in 9 files, `source_program`, `modifier_parser`, `parser_program`, `item_tools`, `item_runes`, `item_format`, `selection_keys`), plus `owned_rules` in two files.
- The wasm32 "portable import" check (`rust.yml:101-102`) is run with legacy enabled.

**Impact:**
- Legacy retirement, which the 2026-09-14 ADR requires, is structurally blocked.
- The documented boundary is not true and nothing tests it.

**Recommendations:**
1. Extract a `poe-optimizer-pob-compat` crate (Lua number/pattern/bits, `source_program`, `parser_program`, `modifier_parser`, item tools) that depends on core and data.
2. Set `default-features = false` everywhere: root, import, pob.
3. Gate `data_loading.rs`/`inspect-*` behind a CLI `legacy-inspect` feature.
4. Make CI fail if `cargo tree -p poe-optimizer-cli -e features` shows `poe-optimizer-data feature "legacy"`.
5. Until then, fix the README.

### P1-3: The import crate is two products *(high)*

**Evidence:** about 65k of import's 94k lines are `owned_*` release tooling:
- `owned_release.rs:1` "Standalone offline assembly…"
- `owned_successor.rs:1` "Explicit offline package succession"
- Giant functions such as `finalize_successor_operation` (`owned_successor.rs:1229`, about 579 lines) and `compile_owned_tree_catalog_extension` (`owned_tree_catalog.rs:546`, about 642 lines).

`pob` depends on all of import just for `decode_build`/`xml_compat`. It then hashes import's files by relative path (`pob/src/runtime.rs:264-271`).

**Recommendation:** split into `poe-optimizer-import` (decode, XML gates, projectors, item loading, normalize) and `poe-optimizer-authoring` (release, succession, recipes, catalogs). Only the CLI needs both.

### P1-4: The authoring model doesn't scale: opaque hand-allocated IDs and publishing in test code *(high)*

This is the systemic issue behind the data-format complaint. The format fix is in §3; the process issues are:

1. **IDs are hand-predicted.**
   - `owned_recipe_extension.rs:201-221` requires each schema entry to be *exactly the next* registry allocation.
   - So authors hard-code hex ids in JSON *and* do hex arithmetic in test Rust.
   - The in-progress `incoming-critical-effect` packet is already stale: its pinned base `0x334d` (`tests/support/owned_incoming_critical_publication.rs:196-200`) has been used by a committed packet (`tests/support/owned_sand_preparation_publication.rs:217`). Rebasing means renumbering about 130 sites by hand.
   - It also re-models `enemyCritChance` with different ids and semantics from the committed `configuration-enemy-critical-chance-inputs` packet (`def.3371`).
   - The normalizer would reject publishing both, but only with a generic `Policy("configuration projection source agreement")` error (`owned_normalize/configuration_inputs.rs:420-427`).
2. **Publishing choreography lives in tests.**
   - 173 test-support files define `fn stage(prior: &StagedOwnedRelease)`, with hard-coded receipts, watermarks and counts (e.g. `owned_incoming_critical_publication.rs:193-373`).
   - The production `extend` CLI (`src/owned_extension_cli.rs:21-80`) can't carry contribution queries or configuration inputs.
   - `RecipeMembershipPatch` has one variant (`owned_recipe_membership_patch.rs:~46-52`), so adding one Encounter input means restating the whole 342-line Encounter definition.
3. **Recommendations:**
   - Use symbolic names in packets, with the publisher allocating ids (§3).
   - Add a production `publish-packet <dir> --onto <endpoint>` driven by a declarative manifest.
   - Add query-registry and member-set append operations.
   - Add `check-packet`, which runs `engine/src/owned_rules/compile.rs` `infer` (unit and type checking) without a test build.
   - Keep a generated index from source field / PoB output to owned id and packet, so duplicate modeling is found at authoring time.

### P1-5: The owned evaluator has no full-build parity check *(medium)*

**Evidence:**
- `tests/calibration.rs` checks the **PoB-host** backend (`backend.id == "pob-poe2-mlua"`, `:121-167`) against independent references, at tolerances of 1e-8 absolute / 1e-9 relative.
- The engine golden tests cover only legacy Spark/Mace.
- The owned plan evaluator has only two fragment observation files.

**Recommendation:**
- Run the owned evaluator on the six calibration builds through `compare_reference`, reporting Applied/Unresolved status per metric next to the deltas.
- Make "owned matches reference on all calibration builds" a gate before retiring legacy.

### P1-6: Identity is tied to source text *(medium)*

**Evidence:**
- There are 229 `include_str!("*.rs")` calls in production code.
- `crates/poe-optimizer-data/src/lib.rs:60-110` hashes about 45 files plus the root `Cargo.toml`.
- `pob/src/runtime.rs:242-293` hashes `Cargo.lock` and *another crate's* files by relative path.
- `src/build_inspect.rs:108-130` does the same for `Cargo.toml`/`Cargo.lock`.

**Impact:** rustfmt, a doc-comment fix, or `cargo update` changes identity and cascades into receipts. It also couples crates through each other's file layout.

**Recommendation:**
- Use explicit semantic version constants, or digests of compiled artifacts (rule program, schema).
- Each crate should export its own fingerprint.
- Leave `Cargo.lock` out of behavioural identity.

### P2-1: Data loading is redundant and recursion-heavy *(medium)*

- **`evaluate-owned --release`:**
  - reads, hashes and decodes **every** artifact, including authoring-only ones (`src/owned_release_cli.rs:180-265`);
  - re-assembles and byte-compares;
  - then deep-clones schema and rules again (`src/owned_metrics.rs:240-262`).
  - A lean `--schema/--rules/...` path exists.
  - **Fix:** give the evaluator its own loader that verifies manifest hashes for runtime artifacts only and moves values instead of cloning them. Keep the full reproduce check in `publish`/CI.
- **Legacy `bundled_snapshot()`** makes five passes over 26.7 MB: SHA, `serde_json::Value`, typed decode, `to_value`, and recursive `reject_discarded_fields` (`crates/poe-optimizer-data/src/game_data.rs:672-716`).
  - It runs again in the CLI (`src/data_loading.rs:20-22`) even though engine caches it in a `OnceLock`.
  - `build.rs:5-13` raises only the main binary's stack to 8 MiB because the debug "typed data loader" overflows. Test binaries, worker threads and wasm don't get the bigger stack.
  - Likely cause (hypothesis): internally tagged recursive enums (`modifier_parser/programs.rs:207-208`) at JSON depth 40, while the owned files are at depth ≤17.
  - **Fix:** deserialize straight from bytes, make the walkers iterative, and add a regression test that loads on a 1 MiB stack.

### P2-2: Coupling to one PoB snapshot *(medium)*

- `poe2/3887ae68` appears in 403 `.rs` files.
- `UPSTREAM_REVISION` is defined three times with no agreement check: `engine/src/lib.rs:75` (legacy-gated), `pob/src/runtime.rs:15` and `pob/src/source.rs:24`.
- `modifier_parser_program_policy.json` has 80 line anchors. They are SHA-guarded, so drift fails loudly.
- 131 Lua probes assert exact upstream line numbers (e.g. `initEnv` at `CalcSetup.lua:717`), and `debug.sethook` with `currentline==N` appears 76 times.

**Impact:** upstream PoE2 PoB changes weekly, and the project is effectively frozen at one commit, with no upgrade runbook.

**Recommendation:**
- Keep one active-pin constant and derive paths from it.
- Compute anchors when source is loaded, by unique pattern match in the manifest-verified file.
- Move probe Lua out of Rust strings into `.lua` files.
- Write a pin-bump runbook.

### P2-3: Import robustness and structure *(medium)*

- **XML lexing:** three hand-written PoB-XML lexers sit beside roxmltree, and each projector re-parses the document: `build_inspect.rs:45-77` triggers five parses.
  - Whitespace rules differ: `xml_compat.rs:31-33` vs `source_xml.rs:335`.
  - `decode_build` documents numeric character references as supported (`lib.rs:125-132`), but every later gate rejects them. The layering is deliberate but undocumented.
  - **Fix:** write one xml.lua-faithful tokenizer, parse once, and pass `&Document` to the projectors.
- **The `item_loading/machine.rs` state machine** (3,099 lines) is stringly typed:
  - `apply_text` runs about 468 lines from `:668`;
  - it has 41 header literals;
  - `retained_fields: BTreeMap<String, ItemScalar>` (`:332`) means a typo reads `None` silently;
  - it clones `raw_lines` on every apply (`:684`).
  - **Fix:** split it by ParseRaw stage, type the fields, and pin the mirrored Item.lua spans as ModParser does.
- **`normalize_fresh`** is about 1,422 lines (`owned_normalize.rs:1109-2532`), and it hardcodes `"treeVersion"` (`:1750`) next to `policy.allocation_attribute` (`:1736`).
- **Two divergent LuaJIT `tonumber` implementations:** `engine/src/lua_number.rs:17-57` vs `import/src/item_loading/syntax.rs:83-114` (no `0b`, `1e1048576`→inf, clamped hex exponents). Delete the import copy.
- **No fuzz or property testing** of the byte-offset lexers or inflate. Differential tests exist (`pob/tests/xml_compat_parity.rs`); add generated inputs and panic-freedom checks.

### P2-4: CLI ergonomics, errors and output hygiene *(medium)*

- **No path in I/O errors:** `main.rs:271-279` prints only `Error: {error}`, and `owned_effects.rs:47-58` has no path context. With up to 10 input files the user can't tell which one failed.
  - **Fix:** one shared `BoundedReader::read(path, role, max)` that replaces the ~10 copy-pasted readers, plus printing the full `source()` chain.
- **Non-atomic multi-file output:** `normalize-owned` writes its outputs non-atomically (`src/owned_normalize.rs:234-238`), while about 30 other commands use `publish_artifacts` (staged tempdir plus no-replace rename).
- **Machine-specific paths in committed artifacts:** committed files contain `\\?\C:\code\poe-optimizer\runs\…` paths (e.g. `data/owned/.../self-flat-selected-bindings/bindings.json:45879`), from canonicalised output paths in reports (`src/owned_normalize.rs:183,218`). Reports cannot be reproduced on another machine.
- **No library API:** `core::Engine<B>` has one implementor (PoB), and the native path is CLI glue.
  - **Fix:** add `engine::owned::Evaluator` that takes a release and returns a report, and make the CLI a thin wrapper.
  - Group authoring commands under `data`/`dev` subcommands and add `--bundle DIR`.
- **Error types:** about 121 bespoke error types, `Result<_, String>` in search and core public APIs, and one exit code for everything. Use typed errors and map them to exit-code categories (usage, unsupported, internal).

### P2-5: Docs, repository and release hygiene *(medium)*

- **Docs:** 720k words with no index, and a circular authority chain (design → domain-architecture → migration → implementation). Every overview opens with dated status prose.
  - **Fix:** add `docs/README.md` as the single entry point. Keep one current-state architecture doc and one status doc. Move `implementation-history.md` and the checkpoints to an archive folder that nobody loads by default.
- **Repository:**
  - 293 MB of tracked JSON; `current/recipe.json` is a byte-for-byte concatenation of registry, schema, rules and routing (19 MB, single line).
  - 84 `.lua.gz` copies, 47 of them same-named as upstream files.
  - The empty `crates/poe-optimizer-native/`.
  - `.git` is 1.5 GB, of which worktrees take 981 MB.
  - **Fix:** stop tracking derived blobs, use a `textconv` pretty-printer for diffs, and use LFS or CI artifacts for big releases.
- **Release:**
  - no release workflow, no `[profile.release]`, and **no LICENSE** (`README.md:391-393`);
  - `--no-default-features` in the README is a no-op.
  - **Fix:** choose a license compatible with the PoB-derived data, add `lto="thin"`, `codegen-units=1`, `strip`, and a tag-triggered release with a hashed owned-release bundle.
- **CI:**
  - `on: [push, pull_request]` runs everything twice;
  - `actions/checkout@v4` and `setup-python@v7` aren't SHA-pinned;
  - there is a redundant clippy pass;
  - MSRV 1.93 is declared but never tested;
  - there is no `[workspace.package]` and no shared dependencies (sha2, serde, roxmltree, flate2, mlua and thiserror are each repeated);
  - there is no cargo-deny or cargo-audit.

### P3: Numeric-semantics nits *(low; none produces a wrong `Applied` value today)*

- **Direct numeric folds** use canonical-ID order and per-prefix integer range checks (`owned_plan/graph.rs:225-253`). `exact_integer_sum` (`:258-301`) already exists; route direct integer `Sum` through it. The same applies to the ModifierTransform chain (`:160-175`).
- **Failure-selection order:**
  - rule-level `All`/`Any` return the *first* operand failure (`execute.rs:296-314`);
  - `Reduction` returns the first failure (`graph.rs:250`);
  - other reductions use `diagnostic_order::compare`.
  - **Fix:** unify on one failure lattice and add Kleene logic for `All`/`Any`.
- **Bitwise quantity `==`** (`execute.rs:53-55`) and **RequireAgreement** (`graph.rs:848`) are Lua-faithful but fragile under reassociation. Lint `Equal` on computed quantities unless one side is a literal or a rounded value.
- **`NearestTiesPositive`** deliberately differs from PoB `round()` (`execute.rs:166-176`) yet is used 70 times in source-mirroring pools. Add an explicit `FloorAddHalf` mode.
- **Platform `powf`/`log10`** in legacy formatting paths, on a declared wasm32 target (`item_tools.rs:299,558`). Use exact power-of-ten tables.
- **PoB oracle sandbox:** it uses `Lua::unsafe_new` plus a deny-list (`pob/src/runtime.rs:89`, `host.lua:14-29`), and LuaJIT `ffi` is still reachable. Acceptable for manifest-verified code, but an allow-list is better.

---

## 3. Data format and authoring DSL

### 3.1 First, correct the premise

| Format | Where | Role | Verdict |
|---|---|---|---|
| **XML** | `tests/fixtures/**/*.xml` (21 files), user input | **Path of Building's** export/share format, parsed in `crates/poe-optimizer-import/src/lib.rs:134`, `source_xml.rs`, `xml_compat.rs` | External interchange. **Cannot be replaced**, since users bring PoB builds. Fix *how* it is parsed (one tokenizer, parse once; P2-3), not the format. |
| **JSON** | `data/owned/**` (about 1,300 files, 293 MB), `crates/poe-optimizer-data/data/game-data.json` | **The project's own data**: definition schema, rule programs, policies, queries, registry, releases | **This is the unreadable part**, and it is in scope. |

### 3.2 Why the owned JSON is unreadable

The canonical example is the in-progress packet `data/owned/poe2/3887ae68/incoming-critical-effect/`. It ports these **7 lines** of upstream Lua (`vendor/path-of-building-poe2/src/Modules/CalcDefence.lua:2268-2274`):

```lua
local enemyCritChance = enemyDB:Flag(nil, "NeverCrit") and 0 or enemyDB:Flag(nil, "AlwaysCrit") and 100 or (m_max(m_min((modDB:Override(nil, "enemyCritChance") or env.configInput["enemyCritChance"] or env.configPlaceholder["enemyCritChance"] or 0) * (1 + modDB:Sum("INC", nil, "EnemyCritChance") / 100 + enemyDB:Sum("INC", nil, "CritChance") / 100) * (1 - output["ConfiguredEvadeChance"] / 100), 100), 0))
if modDB:Flag(nil, "EnemyUnluckyCrit") then
    enemyCritChance = enemyCritChance / 100 * enemyCritChance
end
output["EnemyCritChance"] = enemyCritChance
local enemyCritDamage = m_max(((env.configInput["enemyCritDamage"] or env.configPlaceholder["enemyCritDamage"] or 0) + enemyDB:Sum("BASE", nil, "CritMultiplier")) * (1 + enemyDB:Sum("INC", { flags = ModFlag.Hit }, "CritMultiplier") / 100), 0)
output["EnemyCritEffect"] = 1 + enemyCritChance / 100 * (enemyCritDamage / 100) * (1 - output.CritExtraDamageReduction / 100)
```

The port into owned form costs:

| Artifact | Size |
|---|---|
| `extension.json` | 2,192 lines, 65 KB |
| `policy.json`, `queries.json`, `native-inputs.json`, `source-vectors.json` | about 2,160 more lines |
| README | 82 lines |
| Tests (4 files) | 1,538 lines |
| Proposal doc | 127 lines |
| **Total** | **about 6,100 lines over 12 files, for 7 lines of formula** |

Here is one *trivial* step, "use the configured crit chance if present, else 5%", as it appears in `extension.json:1215-1260`:

```json
{
  "id": "chance-default",
  "expression": {
    "kind": "literal",
    "value": {
      "kind": "quantity",
      "value": {
        "value": 5,
        "unit": {
          "kind": "unit",
          "namespace": { "game": "poe2", "version": "owned-mechanics-v1" },
          "key": "def.0000000000000002"
        }
      }
    }
  }
},
{
  "id": "chance-configured",
  "expression": {
    "kind": "select",
    "condition": "chance-present",
    "when_true": "chance-raw",
    "when_false": "chance-default"
  }
}
…
{
  "id": "chance-configured",
  "when": "is-hit",
  "effect": {
    "kind": "derive",
    "entity": "current",
    "stat": {
      "kind": "stat",
      "namespace": { "game": "poe2", "version": "owned-mechanics-v1" },
      "key": "def.0000000000003357"
    },
    "value": "chance-configured"
  }
}
```

The root causes, in order of importance:

1. **No names.** By design (`core/src/owned_definitions.rs:1-5`: *"Symbols … contain no display labels"*), every stat, unit, input, option and actor is `def.<16-hex sequence>`.
   - The registry (`current/registry.json`) stores only sequence, kind and state.
   - Meaning lives in prose: README tables, and 92 hex fragments such as "`33b2/33b3`" in `docs/implementation.md`.
   - **`def.0000000000000002` means "percent". No artifact says so.**
2. **An IR used as the authoring language.** `RuleExpression` (`core/src/owned_rules.rs:999-1095`) is SSA/ANF: binary ops only, and every intermediate value is a named node.
   - The actor program above has 12 reads, **45 nodes** and 3 effects for two Lua expressions.
   - 17 nodes are pure `read` pass-throughs, and 8 are `percent_as_factor`.
   - This is a good *compile target* and a poor *source language*.
3. **Encoding noise.**
   - Every reference carries `kind` plus a full `namespace` object plus `key`, and often a second wrapping `kind`/`value` tag.
   - Measured with node: the namespace object alone is **23.7% of `schema.json`, 24.1% of `recipe.json`, 27.4% of `registry.json` and 27.7% of `extension.json`**. That is 76k copies in the 19 MB recipe.
   - It is the main contributor to the 64 MiB ceiling problem (P0-4).
4. **Minified single-line multi-MB files in git.** `recipe.json` is 19 MB on one line, so diffs can't be read and `review.json` SHA pins are approved without anyone reading them.
5. **Restating instead of appending.** Adding one Encounter input restates the full 342-line Encounter definition (there is no member-append operation).
6. **IDs are hand-allocated** (P1-4). Authors must predict the next registry sequence.

Note on performance: JSON *parsing* is not the bottleneck. Node parses the 19 MB recipe in about 107 ms, and serde is faster. The runtime costs are elsewhere: re-validating everything, deep clones, and `String` ids (P1-1, P2-1). The format problem is mostly **human cost, diff/review cost and size-ceiling cost**.

### 3.3 Options evaluated

Criteria: human readability (R), runtime parse/memory (P), size (S), expressiveness for *formulas plus declarations* (E), tooling and validation (T), migration cost (M, where ✓ means cheap).

| Option | R | P | S | E | T | M | Notes |
|---|---|---|---|---|---|---|---|
| Status quo JSON (SSA, opaque ids) | ✗ | ~ | ✗ | ~ | ✓ (serde, deny_unknown_fields) | – | Unreadable; at the ceiling. |
| JSON + hoisted namespace + compact `"stat:335c"` ids + labels | ~ | ✓ | ✓ (−25-30%) | ~ | ✓ | ✓✓ | Cheap and necessary, but formulas are still 45-node SSA. **Do it as step 0.** |
| YAML / JSON5 | ~ | ~ | ~ | ✗ | ~ | ✓ | Only removes punctuation. YAML adds ambiguity (Norway problem, anchors) to byte-exact identity. Doesn't fix SSA. |
| TOML | ✓ for flat config | ✓ | ✓ | ✗ | ✓ | ✓ | Very bad for deep trees and expressions. Good only for flat manifests. |
| **KDL** (v2) | ✓ | ✓ | ✓ | ~ (nodes, args, props, children) | ~ (`kdl` crate, schema emerging) | ✓ | Good for **declarative** packet content: definitions, policies, queries, membership appends. Still no infix math. |
| RON | ~ | ✓ | ✓ | ~ | ~ | ✓ | Rust-shaped, close to serde, but still a data notation: formulas stay trees. |
| Embedded scripting (Lua, Rhai) | ✓ | ✗ | – | ✓✓ | ~ | ✗ | Rejected. It loses static unit checking, lazy-select guarantees, the work budget, determinism and wasm, and contradicts the ADR ("no interpreter"). |
| CEL / general expression language | ✓ | ~ | – | ~ | ~ | ~ | No unit or quantity types, no domain constructs (contribution queries, derive/contribute effects). You would fork it anyway. |
| **Purpose-built formula DSL → existing SSA IR** | ✓✓ | ✓ (authoring-time only) | ✓✓ | ✓✓ | needs building (parser, fmt, LSP-lite) | ~ (a mechanical raise/lower tool makes it safe) | Fixes root cause 2. The IR, checker and engine are unchanged. |
| Binary runtime package (`postcard`/`rkyv`) with interned `u32` ids | ✗ (not for humans) | ✓✓ (zero-copy possible) | ✓✓ | n/a | ~ | ✓ | The right **runtime** form. JSON becomes the debug/diff dump. |

### 3.4 Recommendation: four layers, one IR

Keep the **checked SSA `RuleProgram` as the single IR**. It already has the static checker (`engine/src/owned_rules/compile.rs:1005` `infer`), lazy `Select`, typed failures and versioned operations. Add source and runtime forms around it:

```
 authoring (humans/agents)                 canonical (git, diffs, digests)          runtime
 ───────────────────────────               ───────────────────────────────          ─────────────
 *.rule   formula DSL          ─lower─►    SSA RuleProgram  (compact JSON,  ─pack─► binary package
 *.kdl    packet manifest      ─lower─►    hoisted namespace, "stat:335c")          interned u32 ids,
 names.kdl  symbol table  ◄──raise──       pretty-printed, 1 file/program           rkyv/postcard
```

**Layer A: a symbol table, `names.kdl`.** It is non-semantic, excluded from digests, and validated against the registry.

```kdl
// data/owned/poe2/<pin>/names.kdl   — slug → stable opaque key (identity stays opaque)
unit    percent                           key="0000000000000002"
unit    factor                            key="0000000000000001"
actor   player                            key="000000000000332a"
encounter pinnacle_no_boss_skill          key="00000000000031d1"
option  damage_category.dot               key="0000000000003254"
input   enemy.damage_category             key="0000000000003251"
stat    enemy.crit_chance_configured      key="0000000000003357" doc="configInput enemyCritChance, default 5"
stat    enemy.crit_damage_configured      key="0000000000003358"
stat    incoming_crit_chance_inc          key="000000000000335c" doc="INC EnemyCritChance (receiving actor)"
stat    enemy.crit_chance_inc             key="000000000000335d"
stat    configured_evade_chance           key="000000000000335e"
stat    enemy.crit_bonus_base             key="000000000000335f"
stat    enemy.crit_bonus_inc_hit          key="0000000000003360"
stat    crit_extra_damage_reduction       key="0000000000003361" doc="min(BASE ReduceCritExtraDamage,100), see CalcDefence.lua:1995"
stat    incoming_crit_chance              key="0000000000003362"
stat    incoming_crit_bonus               key="0000000000003363"
stat    incoming_crit_effect              key="0000000000003364"
```

In a *new* packet, names have no `key=`. The publisher allocates the next ids at stage time and writes them back into `names.kdl`. That removes hand-predicted hex and the renumbering on rebase (P1-4).

**Layer B: the formula DSL, `.rule`.** Here is the same `incoming-critical-effect` actor program, all of `extension.json`'s actor program (about 860 lines):

```rust
// incoming-critical-effect.rule — mirrors CalcDefence.lua:2268-2274 @ 3887ae68
ops v22

program incoming_critical_effect for actor player {
  in category    : option  = enemy.input(damage_category)
  in never       : bool    = enemy.flags(incoming_critical_never)      // Flag/Any query, Partial
  in always      : bool    = enemy.flags(incoming_critical_always)
  in unlucky     : bool    = self.flags(incoming_critical_unlucky)
  in cfg_chance  : percent = enemy.stat(crit_chance_configured)
  in cfg_damage  : percent = enemy.stat(crit_damage_configured)
  in self_inc    : percent = self.stat(incoming_crit_chance_inc)
  in enemy_inc   : percent = enemy.stat(crit_chance_inc)
  in evade       : percent = self.stat(configured_evade_chance)
  in bonus_base  : percent = enemy.stat(crit_bonus_base)
  in bonus_inc   : percent = enemy.stat(crit_bonus_inc_hit)
  in reduction   : percent = self.stat(crit_extra_damage_reduction)

  let is_hit = not (category == damage_category.dot)

  let chance = clamp(cfg_chance * (1 + pct(self_inc) + pct(enemy_inc)) * (1 - pct(evade)), 0%, 100%)
  let base   = if never { 0% } else if always { 100% } else { chance }
  let final  = if unlucky { base * pct(base) } else { base }
  let bonus  = max((cfg_damage + bonus_base) * (1 + pct(bonus_inc)), 0%)
  let effect = 1 + pct(final) * pct(bonus) * (1 - pct(reduction))

  derive self.incoming_crit_chance = final  when is_hit
  derive self.incoming_crit_bonus  = bonus  when is_hit
  derive self.incoming_crit_effect = effect when is_hit
}
```

That is about 30 lines against about 860, and a reviewer can check it line by line against the Lua.

Lowering rules. Each must be exact, because the project cares about float association:

| DSL construct | IR lowering |
|---|---|
| `a + b + c` | Left-associative, lowered in source order: `Add(Add(a,b),c)`. The lowerer **never reassociates**; it preserves *"the source's arithmetic association"*, which the packet README requires. |
| `x * f` where `f` is dimensionless | `Scale{value:x, factor:f}`. `pct(p)` → `PercentAsFactor{percent:p, unit:factor}`. |
| Numeric literals | Unit inferred from context or suffix: `1` in factor context → `Literal(quantity 1 factor)`; `0%` → percent. Ambiguity is a compile error, never a guess. |
| `clamp(x, lo, hi)` | Sugar for `Maximum(Minimum(x, hi), lo)`. This is exactly `m_max(m_min(x,100),0)`. Order is fixed and documented. |
| `if c { a } else { b }` | Lazy `Select`. `else if` nests. |
| `in name : T = …` | `ReadSpec` plus an implicit `Read` node, so pass-through nodes disappear from source. |
| `let` names and generated temporaries | Become SSA node ids, so error messages and IR diffs use author names. |
| Type and unit checking | The **existing** `infer` (`compile.rs:1005`) runs on the lowered IR. The DSL front-end adds name resolution and literal-unit inference only. |

Grammar sketch (EBNF; hand-written recursive descent or Pratt parser, roughly 1.5-2.5k lines of Rust with good spans):

```ebnf
file        = header , { item } ;
header      = "ops" , VERSION ;
item        = program | shared ;                         (* shared = reusable let-blocks, e.g. is_hit *)
program     = "program" , IDENT , "for" , owner , "{" , { stmt } , "}" ;
owner       = ( "actor" | "encounter" | "item" | "skill" ) , PATH ;
stmt        = input | let | effect ;
input       = "in" , IDENT , ":" , type , "=" , source ;
source      = entity , "." , ( "stat" | "input" | "flags" | "query" ) , "(" , PATH , [ "," , ARGS ] , ")" ;
entity      = "self" | "enemy" | "current" | IDENT ;
let         = "let" , IDENT , [ ":" , type ] , "=" , expr ;
effect      = ( "derive" | "contribute" , KIND | "capability" ) , target , "=" , expr , [ "when" , expr ] ;
expr        = ifexpr | orexpr ;
ifexpr      = "if" , expr , block , { "else" , "if" , expr , block } , "else" , block ;
orexpr      = andexpr , { "or" , andexpr } ;               (* → Any  *)
andexpr     = cmp , { "and" , cmp } ;                      (* → All  *)
cmp         = sum , [ ( "==" | "!=" | "<" | "<=" | ">" | ">=" ) , sum ] ;
sum         = prod , { ( "+" | "-" ) , prod } ;            (* left-assoc, never reassociated *)
prod        = unary , { ( "*" | "/" ) , unary } ;
unary       = [ "not" | "-" ] , atom ;
atom        = NUMBER , [ UNIT_SUFFIX ] | PATH | call | "(" , expr , ")" ;
call        = IDENT , "(" , [ expr , { "," , expr } ] , ")" ;   (* pct, clamp, min, max, round(x, q, mode), quantize, lookup *)
type        = "bool" | "int" | "option" | UNIT_NAME ;
```

**Layer C: declarative packet manifests in KDL**, replacing the restatements in `policy.json`, `queries.json`, `native-inputs.json` and the `stage()` test choreography:

```kdl
packet "incoming-critical-effect" onto="release:owned-enemy-armour-component-publication-01" {
  source "vendor:src/Modules/CalcDefence.lua" lines="2268..2274" sha256="…"
  rules "incoming-critical-effect.rule"

  config-input "enemyCritChance"  presence=enemy.crit_chance_present  value=enemy.crit_chance_raw  unit=percent default=5
  config-input "enemyCritDamage"  presence=enemy.crit_damage_present  value=enemy.crit_damage_raw  unit=percent default=30

  query incoming_critical_never   entity=enemy kind=flag combine=any coverage=partial flag="NeverCrit"
  query incoming_critical_always  entity=enemy kind=flag combine=any coverage=partial flag="AlwaysCrit"
  query incoming_critical_unlucky entity=self  kind=flag combine=any coverage=partial flag="EnemyUnluckyCrit"

  append encounter.pinnacle_no_boss_skill.external_inputs enemy.crit_chance_present enemy.crit_chance_raw
  vectors "source-vectors.json"      // replayed by the generic packet harness (P0-2)
}
```

KDL was chosen for this layer because it is node-oriented with properties, which fits "declare N things with attributes" far better than TOML (no deep tables) or YAML (implicit typing). Each `config-input` row also replaces about 60 lines of `policy.json`. This layer also gives `publish-packet` (P1-4) its input format.

**Layer D: the canonical and runtime encodings.**
- **Canonical, tracked in git:** compact JSON.
  - Namespace declared once per package (`Check::namespace` in `owned_schema.rs` already enforces it is uniform).
  - `DefId` serialized as `"stat:335c"`, with the kind checked against the expected `K` on decode (keeps today's guarantee).
  - Duplicate inner tags dropped.
  - Pretty-printed, one program per file.
  - Registry stored as `(kind, state)` rows, since sequence and key derive from position (`owned_mapping/registry.rs:55,62`).
  - Expect about 30% less size and readable diffs.
- **Runtime:** a `postcard` or `rkyv` package with ids interned to `u32` at pack time. That feeds directly into the `Copy` value representation (P1-1.3), and the evaluator never parses JSON or allocates `String` ids.

**Tooling** (one subcommand group, `poe-optimizer rule …`):
- `fmt`: canonical formatting, so diffs stay stable.
- `check`: parse, resolve, lower, `infer`. No test build needed.
- `lower --emit-ir`: DSL → canonical JSON.
- `raise`: IR → DSL (see migration).
- `explain <stat>`: shows the formula for a stat with inputs resolved to names. It is also the debugging story for "why is this number wrong".

### 3.5 Migration path (low risk, incremental)

0. **Wire hygiene and labels (days).**
   - Add the compact `DefId` string plus namespace hoisting behind a new `schema_version`; decoders accept both forms during the transition.
   - Generate `names.kdl` from the existing README tables and packet docs. The 312 stats, 49 inputs, 17 units, 9 actors, 6 capabilities and 63 modifiers in the endpoint schema have no mapping row, so seed them from docs and review by hand.
   - Add a `.gitattributes` `textconv` pretty-printer.
   - This immediately recovers about 25-30% of the 64 MiB budget.
1. **Raise existing programs (about a week).**
   - Write `rule raise`. It inlines single-use SSA nodes into expressions, keeps multi-use nodes as `let`, and maps keys through `names.kdl`.
   - Run it over all 7,227 existing program instances (average 5.4 nodes, maximum 133).
   - Prove equivalence mechanically: `lower(raise(ir))` must be **alpha-equivalent** to `ir`, meaning the same graph up to node-id renaming. CI checks this for every program.
   - Node-id renaming changes digests, so do one deliberate re-pin. That is another reason to remove source and digest pins first (P0-2).
2. **Author new packets in `.rule` plus KDL (ongoing).**
   - `publish-packet` allocates ids and appends membership.
   - The JSON files under `data/owned` become generated output, not hand-authored input.
   - Start with the stale `incoming-critical-effect` draft. Rebasing it then becomes a re-publish instead of renumbering 130 sites.
3. **Binary runtime package (with P1-1).**
   - Pack to `postcard`/`rkyv` with interned ids.
   - `evaluate-owned` loads the pack, and JSON stays the review and debug form.
4. **Retire hand-authored JSON.**
   - Lint rejects hand edits to generated JSON, for example by stamping a header with the generator digest.

What *not* to do: don't put the DSL on the runtime path, don't let it reassociate arithmetic, and don't embed a general scripting language. The checked IR is a real asset, and the goal is a readable front-end for it.

---

## 4. Suggested sequence of work

1. **Stop adding to the drag.**
   - Freeze new receipt and evidence types.
   - Remove source-file SHA pins.
   - Consolidate test binaries.
   - Add `docs/README.md`.
   - (P0-2, P2-5)
2. **Make data releases real.**
   - Track the endpoint and add a replay manifest.
   - Add the size guard and split the budgets.
   - Do §3.5 step 0 (compact ids, namespace hoisting, labels).
   - (P0-3, P0-4)
3. **Deliver the product loop.**
   - One build completes natively.
   - Native `CandidateEvaluator`.
   - One decision axis.
   - Benchmark.
   - (P0-1)
4. **Make it fast enough to search.**
   - Plan skeleton plus parameters, interned `Copy` values, an explicit cache contract and a streaming search loop.
   - (P1-1)
5. **Untangle layers.**
   - `pob-compat` crate, split import from authoring, `default-features = false` and a CI guard.
   - (P1-2, P1-3)
6. **Authoring at scale.**
   - `.rule` DSL, KDL manifests, `publish-packet`, the generic packet harness and owned-evaluator parity on the calibration builds.
   - (§3.5 steps 1-3, P1-4, P1-5)
7. Then work through P2/P3 opportunistically.

---

## 5. Strengths to preserve

- **Owned evaluator:**
  - typed failure lattice instead of numeric defaults (`owned_rules.rs:139-170`);
  - finite-only `FiniteQuantity` with −0 normalised;
  - checked `BoundedInteger` (±2^53);
  - explicit-stack VM with no recursion (`execute.rs:285-352`);
  - lazy `Select`;
  - work-budgeted execution;
  - `BTreeMap`-only determinism.
- **Static checking of rule programs:** units and types via `infer`, and `deny_unknown_fields` throughout.
- **Content-addressed releases:** manifests with bytes and sha256, and a strict release-directory inventory that rejects symlinks and extra files (`src/owned_release_cli.rs:129-194`).
- **Hardened input handling:**
  - bounded decoding (8 MiB XML, 1 MiB share code, chunked inflate budget);
  - roxmltree with DTDs off and a node limit;
  - XML gates that faithfully mirror PoB's `xml.lua`, with differential parity tests.
- **Output safety:** atomic, no-clobber output via `publish_artifacts` (renameat2 NOREPLACE / MoveFileExW), with race tests.
- **Dependency and CI hygiene:**
  - zero duplicate crate versions and `--locked` everywhere;
  - a depfile-based boundary checker (`scripts/check-owned-boundaries.py`);
  - wasm32 checks;
  - a Lua-free default CLI.
- **Coverage honesty:** coverage is stated inside the data (`declarations_preserved_not_certified`, Partial vs Complete), and packet READMEs carefully list what they do not claim.
- **Search scaffolding:** deterministic SplitMix seeding, separate feasible and infeasible archives, and fresh finalist verification. It is a sound frame waiting for a native evaluator.
