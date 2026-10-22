# Implementation plan and resume point

Updated: 2026-10-22 (EDT).

This is the only active delivery schedule. The [architecture](domain-architecture.md)
defines the end state; [documentation index](README.md) explains authority. The
[owner-provided review](architecture-review-2026-10-22.md) is adopted with the
adjustments below. Historical checkpoints remain in Git and
[implementation history](implementation-history.md), not in this resume point.

## Current state

**Next work is the DSL/data/runtime migration, before further mechanic content.**
The first complete unchanged build must use the new format and sole native path.
Legacy deletion is a gate before that milestone, not work deferred until five builds.

| Area | Verified state at this checkpoint |
| --- | --- |
| Code baseline | `fa49fcea32f0413cde5f5983e179c34e6c7acbdf` on main; October 22 changes adopt the plan only |
| Complete native originals | **0/5**; selected input issue counts **123/137/127/144/5** |
| Closest build | Original05 Sniper; five input obligations are not its entire remaining numerical/legality work |
| Native engine | Checked typed IR, explicit-stack VM and occurrence/effect graph exist; no native optimizer integration |
| Authoring | Hand-authored IR JSON, packet publication in test staging and source-file hash dependencies remain |
| Runtime packaging | Legacy Data/Engine default features and embedded legacy snapshot remain; default builds are not yet free of that stack |
| DSL / KDL / binary pack | Adopted target; not implemented |
| Current release | Latest validated artifacts are local under ignored `runs/`; a fresh clone cannot yet reproduce the current release with one supported command |
| CI | No passing hosted result verified for fa49 in this checkpoint. Prior snapshot was pending; do not label CI green from local tests or a push |

Last validated release: `runs/owned-reverse-primary-cutover-01/package`.
Its endpoint is `runs/owned-reverse-primary-cutover-01/endpoint.json`,
**66,145,838 bytes** against the old 67,108,864-byte bound (about **98.56%**).
Endpoint SHA-256: `337266997cd5ead1949288f29aad62b9b9604b2acda2b96fb0aacf0cea505ad2`.
Input identity: `a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`.
Validation: `runs/owned-reverse-primary-validation-01/validation.json`.
These paths locate historical local evidence; **they are not acceptable required
inputs to the replacement build or CI**. Capture the useful source fixtures and
semantics in the new tracked manifest during R1/R2.

The unfinished generated-singleton import slice is superseded and backed up on
[branch codex/backup-2026-10-22-pre-dsl](https://github.com/Azaril/poe-optimizer/tree/codex/backup-2026-10-22-pre-dsl)
at `7c43da94b6ab386e40071fdb5ec11a60e6d434e7`. Main was restored to the validated
baseline before this documentation change. That snapshot had 244 passing Import
library checks, 18 passing and two failing focused checks (expectation repairs
not rerun), nine passing ordinary CLI checks, and a **failed** all-five publication
at the normalization work limit. It is not a published/current release. Preserve
useful malformed-token, exact-provider and complete-census cases in the shared
harness; do not revive the old publication chain to advance content.

**This documentation checkpoint:** adopted the review, consolidated the active
documents and repaired local/historical links. Local links and `git diff --check`
pass; the nine protected files match their baseline hashes. No runtime code or
data semantics changed, so Cargo tests were not rerun for this checkpoint. R0
implementation and hosted CI repair remain next work.

## Accepted review adjustments

- Keep one checked SSA IR and native VM. Add a Rust formula DSL, symbolic names,
  KDL manifests, compact canonical IR and an interned binary runtime package.
- Break formats in place and rebuild/reimport. No dual readers, compatibility
  aliases, permanent old evaluator, or `legacy-inspect` product feature.
- Delete source-file hashes and layout/count assertions used as behavioral gates.
  Keep artifact integrity, actual semantic invariants, independent references and
  upstream pin provenance. Provenance is not a reason to hash Rust text at runtime.
- Symbol labels/documentation are nonsemantic; symbol-to-ID bindings affect the
  compiled program. Changing a binding must invalidate the resulting content.
- Do not adopt the review's suggestion to retain legacy until all calibration
  builds pass. Move valuable regression cases to the one harness; retain actual
  PoB as optional oracle. Old native paths must be gone before the first new build.
- Preserve current numeric behavior during representation migration. P3 arithmetic,
  failure-order and rounding suggestions need separate semantic tests/decisions.
- The first one-axis native search is an engineering milestone. The first usable
  optimizer still supports joint classes/ascendancies, passives, equipment, support
  gems and supporting skills with the user's locks and configurable goals.

## Delivery plan and gates

R0-R4 are one migration with intermediate checkpoints, not permanent parallel
systems. No new game-mechanic packets on the old path. Use existing real mechanics
as migration cases; do not expand content to work around a compiler limitation.
Independent crate/test/build work can run in parallel with explicit file ownership.

### R0 — Make verification usable and establish a reproducible baseline

**Next executable phase.** Inventory test targets and live consumers; select the
minimum retained semantic fixture set from the current release and all five raw
originals. Remove implementation-source pins and replace them with behavioral
checks before mass mechanical edits. Consolidate support code and data-driven
packet vectors into shared Rust test support. Convert integration targets to
explicit test modules (`autotests = false`); target fewer than 30 integration
binaries across the workspace while preserving meaningful laws, not test counts.
Use nextest partitioning instead of duplicating compilation per packet.

Inspect current GitHub failures and repair actual tests/build limits. Add one
explicit current integration suite that cannot skip silently for missing artifacts.
Keep optional PoB tests separately labeled. Establish a Rust dependency-boundary
check and a representative release-mode timing/allocation/size baseline. This is
measurement, not a promise that existing timings are already acceptable.
Remove duplicate push/PR work and redundant lint passes, pin CI actions to reviewed
commits, and test the advertised MSRV or deliberately correct it. Consolidate
workspace dependency declarations while retaining locked builds. Dependency and
license/advisory checks support the later distribution gate.

**Exit:** retained behavioral suite can compile/run on the development machine and
Windows/Linux CI; target coverage is enumerated, not silently dropped; no source
text pins block a formatting-only change. Record hosted run ID and conclusion.
Missing current-release inputs remain an explicit R1 failure, never a skipped pass.

### R1 — Separate authoring and define one compact release

Move offline catalogs, recipes, assembly and publication into
`poe-optimizer-authoring` (Rust library + developer commands). Runtime Import keeps
bounded decoding, external-format normalization and owned project construction.
PoB extraction/reference tooling is optional. Remove Import -> legacy Engine
coupling by replacing required parsing with owned import recipes, not by moving a
complete Lua emulator into a permanently required compat crate.
Parse external XML once through a bounded checked boundary and pass the parsed
document to projections. Type item fields and split parsing stages. Replace
duplicated number parsing with the single necessary external contract. Add Rust
property/fuzz cases for malformed XML, byte offsets and bounded decompression;
document decoded syntax versus supported import semantics.

Implement one current namespace-hoisted, typed-ID canonical format and separate
runtime/import/authoring/reference artifact roles. Build a tracked manifest that
names every source input, dependency and intended output. The latest release must
be produced by a supported production command from a clean clone; no `runs/`
predecessor and no `tests/support::stage` API. Rename/remove the misleading
`current/` baseline with its callers. Derive one active PoB pin from a manifest.

Split bounded authoring and runtime resource policies; report bytes, entries,
work and memory separately. Add an 80%-headroom warning/CI regression guard with
an explicit review process for budget changes, not an unbounded size bypass.
Stop tracking concatenated duplicate packages; use canonical review artifacts
and a content-addressed runtime release generated by CI or fetched by hash.

**Exit:** clean-clone release construction and validation are ordinary CI, with
relocatable artifacts and no developer-local inputs. Unsupported old wire forms
fail clearly; the temporary offline migration reader is isolated and scheduled
for deletion at R4. No compatibility reader enters the normal CLI/library path.

### R2 — Formula DSL and declarative authoring

Implement Rust parsing, spans, symbol resolution, formatting and exact lowering
of `.rule` files into the existing checked IR. Add KDL definition/packet manifests
and a validated stable symbol binding registry; allocate identities in the
publisher transaction instead of predicting hex keys. Cover every live operation,
read scope, guard, effect and applicability declaration. Use the real Life,
critical, contribution, preparation and generated-source cases as compiler fixtures.
The old IR remains the single semantic compiler target, not a second human
language to fall back to when DSL coverage is missing.

Provide developer `rule fmt/check/lower --emit-ir/raise/explain` and
`check-packet` / `publish-packet` commands. Manifests support definitions, inputs,
queries, member appends, source/output ownership, routing and vectors. Produce
useful duplicate-authority diagnostics at authoring time. A deterministic
production release builder replaces the hand-written succession chain.

Raise all retained live programs, then check `lower(raise(IR))` for graph
alpha-equivalence (node renaming only), including operator order, lazy failure,
units, reads and ownership. Run executable differential vectors as an independent
check. Assign human names from reviewed semantics; never infer mechanics from
build names. Rebuild canonical output and all five imported projects.

**Exit:** all retained mechanics are authored through `.rule` + KDL; no manual
SSA JSON, per-packet Rust publisher, Rust-source review pins or duplicate
configuration definitions. Formatter/checker and shared harness run without a
per-mechanic test binary. Fresh generation equals the checked-in canonical output.

### R3 — One native library evaluator and efficient runtime data

Pack checked rules/definitions into bounded binary runtime data with package-local
interned IDs and compact values. Benchmark a straightforward serializer such as
postcard against rkyv before selecting the codec; validation and portability are
mandatory, zero-copy is not. Runtime loading validates only runtime artifacts
once and moves data into immutable shared storage. Full source reproduction stays
offline. Retain debug mappings without String IDs in arithmetic hot paths.
Exercise package loading with a small worker stack (including a 1 MiB regression)
and malformed indices/lengths; do not depend on the main executable's stack override.

Expose a library Evaluator used by CLI and search, with explicit dataset,
build/scenario/query inputs, preparation, worker scratch and typed reports.
Split reusable structural plans from indexed candidate parameters. Record which
inputs affect topology, applicability, preparation or coverage; changes to them
must rebuild/invalidate affected plans. Changing a gem level is not automatically
safe to treat as a value-only edit. Cache identity includes semantic operation,
package, structural inputs and scope, not source-file text.

Replace the no-cache contract with cache-state-independent results. Bounded
explicit caches must agree with cold execution. Keep metrics, unavailable
coverage and same-run requirement assessment together; absence/zero/unknown do
not collapse during report or objective conversion. CLI readers include path,
role and error chain with typed exit categories; normalize/publish writes are
atomic and references portable.

**Exit:** native and Wasm builds load/evaluate without Lua, PoB, authoring crates
or legacy snapshot. Shared immutable plans plus private worker scratch pass
serial/Rayon, cold/warm, cancellation and parameter-invalidation tests. Benchmark
load, import, prepare, compile, execute and report separately, with allocations
and p50/p95; quantify improvements without using performance to weaken validation.

### R4 — Cut over and delete the old system

Use the [retirement map](architecture-migration.md) to remove legacy modules,
feature flags, profiles, generated snapshots, old wire readers, test publishers,
obsolete CLI commands and exclusive tests. Migrate useful laws into the common
harness before removing their scaffolding. Delete temporary converters after the
one-time raise/rebuild is checked. Keep the small general `raise` developer tool
only if it remains useful for the current IR; no historical decoder retained.

Audit Lua-specific types/behaviors at every surviving boundary: necessary external
syntax belongs to Import, source execution to offline extraction/oracle, and
proved game semantics to typed rules/native operations. Do not carry nil/table
truthiness, metatables, UI lifecycles or callback captures into native data.
Replace/remove existing Python utilities with their Rust workflow where still
needed. No new Python/JS tools. Remove dead compressed source copies and empty
crates; dependency closure and shipped contents are checked on native/Wasm CI.
Audit the optional Lua host's exposed capabilities and prefer an explicit allowlist
for required source/runtime modules. This is reference isolation work, not native
runtime functionality or a claim that processes form a security sandbox.

**Exit:** one authoring path, one current format, one model, one native evaluator;
all five unchanged originals pass through the same new importer with honest
remaining diagnostics. Clean clone reproduces the current release. Actual PoB
is the only optional reference backend. Required CI is passing. Update overview,
README and surviving contract docs so removed systems are not described as live.
**Only now resume missing mechanic content.**

### R5 — First complete unchanged build, then native optimization

Re-run all five and choose the closest complete path; Original05 remains the
starting target unless new evidence changes that choice. Fix the earliest actual
input/semantic/legality blocker each checkpoint. Finish exact generated-field
accounting, complete selected parameter/contributor inventories, obligation and
resource consumers, action damage/timing and requested final metrics as required
by the real request, not by the old packet count. The old generated Djinn audit
is useful evidence, not permission to weaken multi-effect/selector checks.

**Build gate:** public CLI + library load the new release, import the unchanged
original, resolve all selected obligations, evaluate the intended metrics and
requirements with complete declared coverage, and match stable independent PoB
reference observations under stated metric tolerances. Trace exact actor/action,
scenario and aggregation semantics; report all affected exception rows explicitly.
No intrinsic-only result, reduced fixture, filled-in default or skipped requirement
counts as the working original. Cold/warm and repeated parallel runs agree.

After this gate, connect native reports directly to configurable objectives and
exercise one real legal mutation axis with locks. Independently reconstruct and
re-evaluate finalists. Benchmark end-to-end throughput and search quality. Use a
bounded in-flight queue and sequence-ordered result integration; native Rayon
workers execute candidates directly, with no subprocess per candidate.

**Optimizer gate:** a reproducible CLI run produces a legal, verified best-found
alternative or correctly reports no improvement. Count-budget runs are reproducible;
wall-clock runs expose their budget/stopping semantics. This prototype is not the
full joint optimizer release.

### R6 — Breadth, joint optimization and product delivery

Complete the remaining four originals, reassessing the next closest each time.
Add independent holdouts covering bossing/mapping, attacks/spells/minions,
resources, conditional effects, item/tree grants and interactions. Use curated
PoB exports (including poe.ninja when its export route is verified); record origin,
game/patch and immutable bytes. No scraping/mass extraction of PoE databases.
[Generality](build-generality-review.md) and [breadth](breadth-validation.md)
checks must expose structural gaps without per-build production branches.

Expand search to all required dimensions jointly, including class/ascendancy,
independent point pools/connectivity, inventory/socket feasibility, support and
supporting-skill interactions. Preserve 1..N skill/item requirements, explicit
locks, configurable constraints/multi-objectives and scenario policies. Test
coordinated mutations/restarts against equal-budget baselines for 5–30 minute runs.
Only fully validated candidates may be reported feasible.

After useful CLI optimization, evaluate a shared web/desktop UI (Tauri host with
Yew/Dioxus or justified alternatives), accessibility, tree visualization, progress,
cancellation, comparison/explanations and browser worker capabilities. Choose a
license before distributing binaries/data; choose trade acquisition policy before
live trade integration. These decisions do not block R0-R5.

## Follow-up register

These are retained obligations, not extra competing phases.

| Work | Placement / acceptance |
| --- | --- |
| Generic Actors, exact source/recipient contributions and typed unknowns | Preserve through R2-R4; player, supplied units and scenario enemy use one Actor model with explicit role permissions |
| Generated preset raw inputs and usage; scenario intent | Preserve exact provider applicability, dormant vs unresolved, single-writer slots, preparation vs participation; test in migration and finish actual R5 blockers |
| Support-generated skills and physical selector/supply separation | Existing proposals remain unapproved where explicitly marked; reassess against the new model before adding authority, not another parallel graph |
| Resource obligations | Accepted Skill-owned payer/resource model remains required; amount-channel signature proposal still needs a concrete decision if R5 requires it |
| Separate socket configurations | Accepted but incomplete; finish when needed for the first build or R6 equipment search; keep rolls/configurations/physical copies separate |
| Scoped numerical coverage / computed source selection | Pending, not implied by DSL approval; unknown reach stays blocking. Ask only when a concrete required case needs a public contract change |
| Lua cleanup and resistance terminology | R4 audit classifies game law vs external syntax vs obsolete application code; ordinary resistance is a game channel, legacy is not |
| Numeric P3 suggestions | After semantics-preserving migration, add targeted cases for exact integer folds, failure order/All/Any, equality, rounding and decimal scaling; discuss output-changing policy before adoption |
| Curve/segment storage | [S1-S4 investigation](owned-scaling-data-investigation.md) in R6 unless an earlier real mechanic requires it; lossless data/domain/rounding equivalence, no fitted extrapolation |
| Seeded jewels / legion-rune opportunity | [J1-J5](seeded-jewel-feasibility.md) in R6: establish game/pin applicability, provider parity, bounded seed/socket/path search, reproducible reporting and independent cost/quality checks; opt-in |
| UI autocomplete | Bounded query/index over loaded named definitions; no database/ORM milestone |
| PoB updates | One pin manifest, offline extraction and selective differential tests, stable semantic anchors and a pin-bump runbook; no automatic network/source work in cargo build |
| Repository/release hygiene | R0-R4 dependency boundaries, deduplicated tests/artifacts/docs, one MSRV policy and locked workspace dependencies; R6 release packaging, dependency audit and measured release-profile tuning |

## Review finding coverage

| Review finding | Delivery |
| --- | --- |
| P0-1 native search missing | R3 API, R5 build + search, R6 joint scope |
| P0-2 verification/test cost | R0 shared behavioral harness, R2 manifests, R4 deletion |
| P0-3 untracked release | R1 clean-clone manifest/build, R4 release gate |
| P0-4 size ceiling | R1 separate bounded policies/compact format, R3 binary pack |
| P1-1 throughput | R3 plan/parameters/IDs/cache/benchmarks, R5 bounded queue; optional oracle pool only after reset-isolation proof |
| P1-2 legacy shipping | R1 separation, R4 removal; no permanent compatibility feature |
| P1-3 mixed Import/authoring | R1 crate/tool split |
| P1-4 authoring and IDs | R2 symbols/manifests/ownership diagnostics/publisher |
| P1-5 full-build parity | R5 then R6; old native evaluator not retained as oracle |
| P1-6 source identity | R0 remove source pins, R1/R3 semantic/artifact identity |
| P2-1 repeated loading | R3 runtime loader and bounded low-stack validation |
| P2-2 pin coupling | R1 manifest, R4 source isolation/update runbook |
| P2-3 import structure | R1/R4 parse once, typed fields, bounded decode and Rust property/fuzz tests; retain only needed external syntax |
| P2-4 CLI/API | R3 thin CLI, typed/path errors, atomic portable outputs |
| P2-5 repo/docs/CI/release | This documentation checkpoint; R0-R4 cleanup/CI; R6 distribution |
| P3 numeric/oracle details | Targeted semantic review after exact migration; reference host capability audit in R4 |

## Checkpoint workflow and next action

1. Read this plan and Git status. Start **R0: test target/live-consumer inventory,
   behavior-focused harness and current CI diagnosis**, paired with the R1 tracked
   source-input/release inventory. Serialize Cargo jobs in this shared checkout;
   parallel agents own disjoint files.
2. Record the current phase, concrete blocker, changes, tests actually run and
   next command. During R0-R4 report migration gates, not invented build progress.
   Re-run all five when import/data semantics change; do not replay expensive
   publication chains for prose-only edits.
3. Preserve independent semantic fixtures while deleting their obsolete machinery.
   Do not regenerate source hashes just to satisfy tests. Missing coverage and
   failed tests stay visible; never edit the originals to make admission pass.
4. Update this resume point and design only where contracts change. Keep detailed
   logs as artifacts, not another sprawling active document. Commit/push coherent
   checkpoints to main, verify the remote, and report CI separately.
5. Discuss significant unapproved domain changes; the DSL/format/runtime direction
   and breaking cleanup here are already authorized. No question blocks R0.

Protected work: leave the nine untracked incoming-critical files unchanged until
explicitly resuming that work. They are backed up at
`codex/backup-2026-10-20-incoming-critical`, commit
`ac6df890be9525f62a614c225aa54b9be317433b`. Reuse useful laws from that branch,
not its stale IDs/duplicate definitions. Do not edit
`crates/poe-optimizer-engine/src/owned_allocations.rs` or its tests during this
checkpoint. Do not read credentials when accessing GitHub logs.
