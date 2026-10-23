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
| Code baseline | Semantic baseline `fa49fcea32f0413cde5f5983e179c34e6c7acbdf`; release source composition `b9ad595d`; clap convergence `265b2afb`; whole-program DSL source cutover `b89f8e4d`; immutable recipe prerequisite `12c18b5d`; current checkpoint removes runtime release reassembly and separates runtime role loading |
| Complete native originals | **0/5**; selected input issue counts **123/137/127/144/5** |
| Closest build | Original05 Sniper; five input obligations are not its entire remaining numerical/legality work |
| Native engine | Checked typed IR, explicit-stack VM and occurrence/effect graph exist; no native optimizer integration |
| Authoring | Bounded Rust DSL, official kdl-rs manifests, exact typed names/slots, transactional appends, stable named allocation and production release command. Temporary Authoring -> Import dependency reuses the sole allocator/assembler; no runtime -> Authoring dependency. Connected offline modules still need to move out of Import |
| Runtime packaging | Legacy Data/Engine default features and embedded legacy snapshot remain; default builds are not yet free of that stack |
| Loader extraction | Checked runtime reader and shared complete-release validation implemented; no publication reassembly in metric/source-action loading. Runtime roles are separated from catalog generation. Offline module/command relocation remains |
| DSL / KDL / binary pack | Every retained program now comes from tracked `.rule` sources with semantic KDL names and membership packets. The normal builder rejects embedded programs. Exact graph/placement and historical non-program preservation gates pass; named-registry filesystem CAS, broader declarations and binary pack remain pending |
| Current release | Tracked `migration-source/release.kdl` composes program-free metadata and DSL through one compiler/assembler. Node renaming changes rule/input identity; other emitted constituents remain byte-exact. No local predecessor needed; evaluation bundle still absent |
| CLI parsing | All retained Rust CLI argument parsing uses the shared workspace `clap` dependency. Authoring command/release tests, example builds/tests and executable behavior checks pass locally; hosted confirmation remains pending |
| CI | At 01:14 UTC on October 23 (October 22 EDT), baseline `ae90ec82` run `39857163789` had 15 successful jobs and three Windows CLI jobs running, with no failures. `12c18b5d` run `39879512548` remained pending with no jobs. Pending main runs coalesce; this checkpoint needs its own hosted result. No green suite claim |

Current reproducible source: `data/owned/poe2/3887ae68/migration-source/release.kdl`.
Use `cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --output rebuilt-release`
with a fresh output directory. Current input identity:
`d5f7086228a1fe2bfbc7fcdde033fd62741e719aa3af188fc5f3686b33b02406`.
The 2,784 declared source files use 5,772,456 encoded and 58,564,599 decoded
bytes, within the existing 64 MiB bounds. The 183,380-byte root manifest and
4,572,462 expanded rule bytes also fit current limits without increases.
Metadata JSON remains temporary; gzip is not the compact runtime format.
The CLI still reports the 80% decoded-size warning; compact-format work remains
required before expanding content. The production command rebuilt the tracked
source into `runs/owned-authoring-dsl-release-01` with the expected input identity,
17 constituent artifacts and all 227 provenance rows.

Historical input
`a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`
and its receipt remain independent regression evidence. Original rule IR lives
only in Authoring's `tests/fixtures/retained-rules.json.gz`, outside production
inputs. Actual-source graph checks plus restoration of just historical program
members prove all non-program metadata/provenance and unrelated artifact bytes
are unchanged. `runs/dsl-conversion-02/conversion-report.json` records local
conversion evidence; no `runs/` file or one-off conversion tool is needed by a
normal source build or CI.

The unfinished generated-singleton import slice is superseded and backed up on
[branch codex/backup-2026-10-22-pre-dsl](https://github.com/Azaril/poe-optimizer/tree/codex/backup-2026-10-22-pre-dsl)
at `7c43da94b6ab386e40071fdb5ec11a60e6d434e7`. Main was restored to the validated
baseline before this documentation change. That snapshot had 244 passing Import
library checks, 18 passing and two failing focused checks (expectation repairs
not rerun), nine passing ordinary CLI checks, and a **failed** all-five publication
at the normalization work limit. It is not a published/current release. Preserve
useful malformed-token, exact-provider and complete-census cases in the shared
harness; do not revive the old publication chain to advance content.

**R0 checkpoint in progress:** Core, Data, Engine and Search now register nine
integration binaries instead of 210. Core/Data/Search retain all 967 compiled test
identities and pass. Engine retains every original root and test body, with 1,312
tests passing and 15 existing optional oracle tests ignored. Its three process-wide
allocator tests remain isolated. Native-only Data/Engine validation also passes
(1,018 tests). The Import regression target passes 16 tests; the Mana target passes
eight in each feature profile, with its explicit authoring utility ignored.
Lint passes for the consolidated suites, both native-only and optional-PoB CLI
profiles, and every external consumer affected by shared fixtures. Engine now
shares 15 stateless fixture modules; its library tests and 11 external targets
use the same graph. Lua oracle helpers remain separate. After that wiring repair,
both Engine profiles, the affected condition/mixed parity cases, both external
Import targets and all eight native CLI targets pass. The native CLI subset has
45 passing tests and 19 explicitly ignored publication/reference utilities.
Publication tests initially failed with OS error 5 inside the filesystem sandbox;
the unchanged target and complete affected native CLI subset pass outside it.
The optional-PoB CLI profile also passes the same 45 tests, and all three affected
PoB support-reference tests pass. Windows linking emits the existing LNK4098 CRT
warning in optional-reference profiles; it is not an ignored test failure.
No production data or evaluation semantics changed. Shared stateless fixtures
replace repeated inclusion where their external helper API can remain intact.

Import now registers three integration targets instead of 94. All 1,458 original
test identities, flags and source paths are retained; normalization stays isolated,
and test-free shared helpers are registered once. Strict all-target Clippy and
formatting pass. All 1,701 library/integration tests pass, with the one existing
fixture-generation utility explicitly ignored (244 library, 205 legacy/raw,
850 owned and 402 normalization tests passed). Core/Data/Engine/Search/Import now
register twelve integration binaries instead of 304. Build cost is still material:
the fresh optimized Import test build took 12m37s locally; suite consolidation is
not a claim that verification is now cheap.

**R2 expression frontend:** `poe-optimizer-authoring` parses into a flat postorder
arena with UTF-8 byte spans, then resolves explicitly injected read names and known
schema units. Iterative lowering emits the existing typed IR with separate syntax
and output-node limits. Arithmetic order, variadic Boolean operands, lazy branches,
static rounding quanta and explicit conversion units are preserved. No folding,
operand commutation, Boolean flattening or implicit conversion is performed.
Bare integers are parsed exactly; quantities use the existing finite Core type,
including its signed-zero normalization. `quantity(literal, unit.symbol)` covers
unit names the suffix grammar cannot express.

The result is an **unchecked expression-local fragment**. Its local node keys
must be remapped when composing a complete program, and that program must pass
the existing Engine checker. Symbol binding does not grant read/provider/effect
authority. There is no frontend evaluator. At that initial checkpoint Core was
the only normal dependency, with Data/Engine and JSON fixtures confined to tests;
the later naming/release checkpoints below add the temporary offline Import seam.
The default CLI dependency
guard now explicitly excludes Authoring as well as PoB/Lua. No new game content
or production evaluation semantics changed.

Validation uses the existing Engine, independent expected arithmetic/rounding
values, missing-input and numerical-failure branches, exact unit/type refusals,
compiler authority rejection and 1 MiB-stack long expressions. A retained Life
contribution guard lowers to the original graph up to node names and replays its
13 source emission cases. Its observed amount is an input to that guard test,
not evidence that the whole Life calculation or a build is implemented. The later
whole-corpus checkpoint below now verifies complete program graph equivalence;
production source cutover remains an R2 requirement.

Expression checkpoint `e1841725`: all **32 Authoring tests pass**, with no ignored cases. Formatting and strict
all-target Clippy pass. Independent review found no remaining blocker. Final
local logs: `runs/r2-expression-lowering-tests-final-20261022.log` and
`runs/r2-expression-lowering-clippy-final-20261022.log`. The first test run exposed
a fixture using current operations with omitted query/application inventories;
the fixture now declares its actually empty inventories, with production checks
unchanged. The optimized rebuild took 28 seconds and tests completed in under a
second; this measures the frontend subset, not release-wide build/evaluation cost.

**R2 named blocks:** ordered `let name = expression;` declarations now use the
same parser/lowerer as individual expressions. Earlier named values retain their
exact node identity; an alias emits no additional node. Equal direct reads remain
separate, and unused calculations remain in the graph. No declaration eagerly
executes: existing effects still demand values through the native VM. Duplicate
names, read shadowing and forward/self references fail. Whole-source byte/token/
syntax/statement limits and whole-block emission limits are checked; per-statement
scratch covers only that statement, avoiding repeated scans of earlier syntax.

Named-block checkpoint `15de8529`: all **43 Authoring tests pass**, with no ignored cases. Formatting and strict
all-target Clippy pass; independent review found no blocker. Tests include a
1,000-alias block on a 1 MiB stack, lazy missing-input/failure branches, and an
existing multi-effect program whose applicability result also guards a
contribution. Exact graph remapping and independent eligible/ineligible results
preserve inactive effects as distinct from applied values. Final local logs:
`runs/r2-named-block-tests-final-20261022.log` and
`runs/r2-named-block-clippy-final-20261022.log`.

**R2 explicit programs:** source now includes stable program/read/effect IDs,
execution context, typed read declarations and guarded effects. All 25 read-source
variants and all nine effect variants construct the existing Core enums directly.
Readable definitions and exact declared slots use a bounded registry checked
against the schema; wrong kinds, missing/unmapped addresses, foreign namespaces
and inconsistent index responses are rejected. Source aliases and semantic read
IDs stay distinct. Metadata literals never become executable strings.

Ordered declarations reuse the same parser and formula lowerer; effects refer
to existing named values. Guards remain `RuleEffect.when`, and owner membership
and completeness stay outside the source program. The result is deliberately
unchecked until the existing Engine validates its containing package. There is
no second native evaluator or JSON escape hatch. This is a compiler migration
checkpoint, not new game coverage or a complete data release.

Explicit-program checkpoint `21db1486`: all **67 Authoring tests pass**, with no ignored cases; strict all-target Clippy
and formatting pass. Full support-program lowering is alpha-equivalent to the
retained IR and preserves independent enabled/disabled, missing-input and lazy
numerical-failure results through the existing Engine. Constructor matrices cover
every read/effect variant as transport checks, not as authority to execute them
in arbitrary contexts. Wrong owners and read types still fail Engine compilation.
Review found and fixed cross-schema registry reuse and late budget enforcement;
regressions cover both. An older malformed-JSON test now expects the parser's
expression error because quoted keys/colons are valid metadata tokens; JSON is
still refused. Final logs: `runs/r2-program-tests-final-20261022.log` and
`runs/r2-program-clippy-final-20261022.log`.

**R2 expression coverage:** `check_domain`, `lookup`, `option` and
`ordinary_timing` now lower directly to existing Core operations. Domains carry
exact static inclusive bounds. Table reads use explicit package keys and declared
return types; Option literals bind exact schema identities. Timing preserves one
atomic recipe and all eight ordered input dependencies, with explicit precision,
time/rate units and output channel. No interpolation, inferred rows, clamping,
formula expansion or evaluation is added to Authoring.

One bounded `from_inputs` binding constructor replaces the earlier read-only API;
the old wrapper is removed. Combined limits cover units, reads, Options and table
signatures. Only table IDs/types are cached, never rows or a second table model.
The existing Engine still checks complete row/domain/type consistency. Program
compilation receives the actual table declarations and reuses the same literal
implementation for source metadata and formulas; the duplicate Option path is
removed. This finishes syntax for every current expression variant, not migration
of all retained programs or completion of a build.

Expression-coverage checkpoint `329808b8`: all **82 Authoring tests pass**, with no ignored cases, plus strict all-target
Clippy and formatting. Independent expected results cover inclusive/adjacent
range boundaries, exact signed table keys and all scalar result types, choice
identity, ordinary timing precision/caps and every missing timing dependency.
Lazy branches/guards preserve failures, and malformed table rows/domains still
fail the existing Engine checker. Independent review found no remaining issue.
Logs: `runs/r2-operations-tests-20261022.log` and
`runs/r2-operations-clippy-20261022.log`. No Core/Engine semantics, imported
original or production mechanic data changed; the five-build status is unchanged.

**R2 reverse conversion:** `raise_program` emits one source declaration per
original IR node and returns an explicit original-node/name map. Stable topological
ordering accepts forward references; read and node storage permutations do not
change output. Operand lists, repeated edges, unused nodes and effect order remain
intact. Every read/effect/expression variant uses the existing DSL and exact symbol
addresses. Quantities round-trip by float bits, including extreme/subnormal values;
Core's signed-zero normalization remains unchanged.

The iterative graph check rejects duplicate IDs, missing references and unused
cycles. Separate limits cover lists, references, symbol aliases and bytes checked
while writing. Missing symbols and the invalid Modifier execution context fail
explicitly. This is structural conversion, not another semantic checker: complete
Engine compilation remains mandatory. Raising is deterministic for a given keyed
graph; it is **not** an idempotent source formatter across arbitrary node renaming.

Raising checkpoint `86bbe9b9`: all **94 Authoring tests pass**, with no ignored cases; strict all-target Clippy
and formatting pass. Independent review found no remaining issue. Tests compare complete
graphs through explicit node remapping, then execute existing fixture programs and
independent arithmetic/guard/table/domain/timing expectations through the same
Engine. Invalid-authority cases still fail; a reversed 2,048-node graph raises,
lowers, compares and drops on a 1 MiB stack. Shared fixtures/read matrices are
reused rather than copied. Logs: `runs/r2-raise-tests-20261022.log` and
`runs/r2-raise-clippy-20261022.log`. Raising/lowering remain library APIs; their
CLI commands, whole-release migration, KDL/publication and runtime cutover remain ahead.

**R2 source formatting and developer command:** a bounded token/trivia formatter
preserves authored names, numeric spellings, grouping, unit adjacency and exact
comment bodies/attachment. It parses first and never resolves, lowers or raises a
program. Structural whitespace uses four-space indentation and LF. Literal CR
and trailing whitespace inside `//` comments remain intact under the lexer
contract. Successful output fits both source and output limits for repeatability.

The separate `poe-author rule fmt [--check] [--] <path|->` executable formats to
stdout or reports formatting differences, with bounded file/stdin decoding and
no file mutation. Statuses are 0 success, 1 check mismatch, 2 error. Authoring stays
out of the runtime CLI dependency graph and gains no normal dependencies.
Validation covers exact token/AST/comment preservation, idempotence, every viable
comment insertion boundary, precise budget failures, a 1,500-statement 1 MiB-stack
case, unchanged lowered IR and independent Engine results. CLI tests cover paths,
stdin, malformed/oversized input, check statuses and unchanged file bytes.
Independent review found and fixed output growth past the source limit; both
library and CLI controls now exercise it. All **109 tests pass** (105 library,
four CLI), with none ignored; strict all-target Clippy and formatting pass.
Independent review found no remaining issue. Local evidence is in
`runs/r2-format-tests-20261022.log` and `runs/r2-format-clippy-20261022.log`.

**R2 name/membership KDL and packet assembly:** `load_names` resolves the namespace
once and constructs existing typed definition/slot addresses directly.
`parse_packet` declares explicit existing owners and virtual source labels;
`lower_packet` parses/lowers each injected source once, bounds expanded membership
cost, and rejects unused/duplicate sources and owner-local ID collisions.
`apply_to` checks the exact schema identity, namespace, base inventory and every
destination before appending. Existing order, coverage/gaps and all unrelated
package fields remain unchanged. The Engine still validates the complete package.

The retained inherent-Life bridge now has `.rule`, `names.kdl` and `packet.kdl`
sources. Its Authoring test compiles the entire program, verifies exact historical
graph equivalence under a bijective node-ID remap, preserves the actual Partial
owner closure, proves the whole append inverse and replays all thirteen observed
emission cases. **This migrates its compiler fixture, not the historical release
publisher.** Old JSON/builders and the later checked-range successor remain in
their current consumers until those consumers move together; no complete-build
coverage or input issue count changed.

At the user's request, KDL now uses official `kdl-rs` through the published
`kdl = "=6.5.0"` crate, with default features disabled and explicit `parse_v2`.
This preserves the declared Rust 1.93 minimum; the newer 6.7.1 release requires
Rust 1.95. The Git-pinned streaming parser and separate lexical preflight are
removed. No alternate decoder, parser fork or process remains.

These are maintained offline source files, not public uploads. Input bytes are
limited before the standard parser runs; retained node/entry/depth/text limits
and strict duplicate/type/field checks follow its parse. Commented-out blocks
are not retained declarations. The upstream parser exposes no recursion/work
budget, and these checks do not claim adversarial parse-time protection. User
build ingestion and native runtime package loading remain separate. The old
streaming-parser stack-stress test is removed with that unsupported guarantee;
normal syntax, exact limits, all 25 definition/six slot domains, packet atomicity
and the thirteen Life vectors remain. CI retains build-profile decoding checks
and excludes `kdl` from normal runtime dependencies.
The syntax fixture uses newline-terminated `/-` nodes: the standard parser
incorrectly rejects a semicolon after such a node (also present in the 6.7.1
source). This is an explicit upstream limitation, not malformed KDL, and not a
reason to add a source rewrite or alternate decoder. The overview records its
minimal reproducer. No production mechanic or retained emission vector changed.

Validation: **141 tests pass** (137 library, four CLI), none ignored; nineteen
focused manifest tests also pass with optimization enabled and debug assertions
disabled. Formatting, strict all-target Clippy and the normal runtime dependency
guard pass. Independent review found no remaining adapter issue. Logs:
`runs/r2-official-kdl-tests-final-20261022.log`,
`runs/r2-official-kdl-clippy-20261022.log`,
`runs/r2-official-kdl-release-decoding-20261022.log`. Validation used Rust 1.98.1;
direct MSRV compilation remains an R0 requirement, not a result claimed here.

**R2 stable named allocation:** `NamedIdRegistry` stages allocations through the
existing `OwnedIdRegistry`; it does not implement a second allocator. The names
snapshot binds the exact allocation-registry digest and retains sorted typed
name bindings. Definition names allocate first, then slot names, independently
of request order. Repeating the same kind/owner reuses the existing ID. Aliases
must target the immutable base, preventing guessed same-batch IDs. Stale checks
include alias-only changes, and loaded successor checks preserve old bindings.
Retired names remain reserved but cannot become executable symbols. Final schema
validation still rejects missing/unmapped descriptors; Engine permissions remain
separate. All failures leave the base unchanged. This is a library transaction,
not filesystem publication: the future host must atomically publish both files
and compare the actual stored predecessor identity.

Authoring temporarily depends on Import to reuse that allocator. This also brings
Import's legacy Data/Engine features into the offline build; it does not introduce
an Import/Engine -> Authoring edge. Remove the dependency in R1 by moving the
connected offline cluster, after separating immutable registry/role data and the
runtime published-release loader. Runtime metrics/source-action loading currently
calls release reassembly; moving those authoring modules without first cutting
that call would violate the intended boundary. No borrowed facade or duplicate
allocator was added to conceal this remaining work.

The generated-Firebolt fixture no longer requires six Import and two reference
Rust files to match their historical text hashes. Its source XML, upstream/data
artifact integrity, exact destination accounting and replay assertions remain.
Historical hash metadata stays sealed as acquisition evidence; it is not repinned.
The next checkpoint below removes the mixed attribute-observer Rust pins;
implementation-fingerprint APIs remain R0 work.

Validation at this checkpoint: all **156 Authoring tests pass** (152 library,
four CLI), none ignored; the focused generated-Firebolt destination regression
also passes (one executed, fifteen unrelated cases filtered). Its fresh CLI
build took 8m40s and execution 0.01s, so broader suite/dependency cleanup remains
material. Strict all-target Authoring and affected CLI Clippy, formatting and the
normal runtime dependency guard pass. Independent
review found and fixed a symbol-projection budget check that ran after cloning;
the final review is clear. Logs: `runs/r2-named-allocation-tests-20261022.log`,
`runs/r2-named-allocation-clippy-20261022.log`,
`runs/r0-firebolt-source-pin-cleanup-20261022.log`,
`runs/r0-firebolt-source-pin-clippy-20261022.log`.

**R1 final-release construction:** `poe-author release build` reads a closed KDL
manifest, bounds encoded and decoded source bytes independently, decodes existing
typed constituents and calls the sole release assembler. Optional expected input
identity is checked before any staging. The library accepts injected bytes; the
CLI restricts files to the manifest directory. Unknown or duplicate declarations,
incomplete evaluation groups, missing/extra files, corrupt gzip and trailing
members fail. Official kdl-rs remains the parser; no streaming parser was added.
The shared Host crate now owns file and directory publication for both CLIs;
the old implementation was removed. New directories publish atomically without
replacement. Named-registry filesystem CAS remains unimplemented.

Tracked bootstrap inputs reproduce all 17 constituent artifacts and the exact
reference receipt in an ordinary Rust test and through the production CLI. No
`runs/` predecessor, test-stage helper or PoB checkout is required by this build
command. This is the retained final composition, not release-wide DSL conversion.
Its evaluation bundle remains absent; all partial owners/routing and both
owner-specific Life programs remain unchanged. No build coverage was added.

Validation: **166 Authoring tests pass** (152 library, four formatter CLI, ten
release), plus **six Host tests**; no ignored cases in either crate. Strict
all-target Clippy passes for both. The first test compile needed an explicit
`sum::<usize>()` annotation; no production check was weakened. Windows atomic
rename tests require running outside this environment's filesystem sandbox;
the unchanged Host suite passes there after OS error 5 inside it. Independent
review is clear. Logs: `runs/r1-release-tests-final-20261022.log`,
`runs/r1-host-tests-unrestricted-20261022.log`,
`runs/r1-release-clippy-20261022.log`, `runs/r1-release-cli-20261022.log`.

**R2 release source composition and tracked cutover:** the same production command accepts
explicit `rule-packet`/`rule-source` declarations, shared exact names and plain
UTF-8 DSL sources. All releases require program-free rule metadata, including
those without packets. Embedded-program admission has been deleted. Every source has
one role, unused rule files fail, and reused memberships charge expanded work
individually. Lowering uses the existing packet frontend; the sole assembler
still checks types, ownership, complete stages and all dependent identities.
The builder does not repair stale headers or fall back after DSL errors.

Distinct source capacity is now 4,096, with separate bounds of 128 constituents
(including query sets), 128 packets and 16 MiB expanded rule text. The corpus
now has 103 packets, 2,561 distinct complete sources and 8,212 exact memberships.
Its 5,011 semantic aliases include exact declaration owners; bounded local name
sets avoid a global symbol limit increase. Final expanded text is 4,572,462 bytes.
The conversion preserves all 6,550 owner/coverage rows, including empty owners,
and both different owner-qualified inherent-Life programs. Shared files are
decoded and charged once; aggregate expanded
program/read/node/effect limits additionally reuse the Engine's release bounds.
Data is now a normal Authoring dependency for its existing checked schema index;
the native runtime still has no Authoring dependency.

Validation: **179 Authoring tests pass** (154 library, seven CLI and eighteen
release), none ignored. Strict all-target Clippy, per-package formatting and the
normal runtime dependency guard pass. Integration covers independent metric
observations, complete artifact equality, shared sources and distinct same-ID
bodies, exact aggregate bounds, invalid symbols/types/authority, cross-packet
duplicates, source-header mismatches and stale evaluation bindings. The CLI
publishes DSL-derived artifacts and leaves no output/staging after invalid DSL.
The actual tracked-source gate compares every graph and exact owner/program
position independently of production assembly, then compiles the complete
package. The archived fixture is authenticated by the historical receipt;
restoring only its program members reproduces that receipt exactly. The actual
DSL release changes only rules, derived recipe manifest and release receipt.

Name review excluded unpublished families that reused current IDs for different
meanings. Shared percent/count, skill-level/quality and damage channels use their
current roles, rather than a first-introducing family label. Units retain exact
identities: Count vs Life and action Rate vs flask-charge Rate are not merged.
Source aliases do not infer mechanics from build names or arbitrary array offsets.
One-off Rust conversion tooling remains ignored local evidence and is removed
from Cargo targets; tracked DSL is now the maintained program input.

Logs: `runs/r2-tracked-dsl-tests-final-20261022.log`,
`runs/r2-tracked-dsl-clippy-20261022.log` and
`runs/r2-tracked-dsl-runtime-dependencies-20261022.log`; actual CLI build:
`runs/r2-tracked-dsl-cli-20261022.log`. Hosted results remain
pending. No original build input, game law, coverage gap or metric changed.

The mixed attribute source audits no longer compare seven historical `.rs` hash
pins to current Rust files. Lua observer, report/XML, data artifact and upstream
authentication remain intact, as do the semantic assertions. Historical receipt
metadata was not rewritten. This does not finish the remaining R0 fingerprint
and suite-consolidation work.

The affected native CLI targets pass **32 tests**, including all ten release CLI
regressions. Their 33 existing historical publication/reference cases remain
explicitly ignored; no claim is made that those old stage chains were rerun.
The optimized build took 15m57s while each target executed in under two seconds;
removing duplicated fixture/staging compilation remains material R0 work.
The focused log is `runs/r1-release-r0-cli-tests-20261022.log`.
Strict Clippy also passes for the affected native CLI targets (including the
Sniper item-attack fixture consumer) and the optional-PoB CLI binary. Formatting
and the default runtime dependency guard pass: Authoring, KDL, PoB and Lua are
absent from normal CLI dependencies. This does not remove the legacy Rust stack
noted above. Logs: `runs/r1-release-cli-clippy-20261022.log`,
`runs/r1-release-pob-cli-clippy-20261022.log`,
`runs/r1-release-runtime-dependencies-20261022.log`.

Two hosted failures were isolated to fixtures: an unavailable skill catalog
retained an inconsistent reverse-primary census, and the Mana equipment review
compared an obsolete serialized-fixture seal after the Item wire shape changed.
The fixes retain strict production checks and independently authenticated source
values. CI now runs pushes on main and pull requests, avoiding backup-branch and
duplicate feature-branch push runs; existing action versions are pinned to commits.

R0 is not complete. The blocking baseline is a runnable retained behavioral suite,
repairs for actual CI failures, and removal of source-text pins from code being
migrated. Remaining target consolidation, dependency boundaries and MSRV work
can proceed alongside R1/R2; all remain cutover requirements before R4 completes.
Do not turn general cleanup into a prerequisite for the first usable DSL compiler.
Preserve retained laws, not redundant copies of the same fixture or the historical
number of tests. The manifest-driven harness is completed with R2, whose authoring
declarations make generic fixture construction possible.

**Whole retained-corpus migration gate:** the ordinary Authoring unit suite now
loads the tracked schema/rule bootstrap, compiles the original package, and raises
every exact owner/program occurrence. Bounded groups pass through actual KDL
names/membership parsing, DSL lowering and transactional packet application. An
independent node-remapping comparator checks each program; a separately assembled
expected package checks exact owner/member placement after application. Restoring
only original program bodies proves all other fields are unchanged, including
coverage gaps, receivers, contribution queries, tables and operation versions.
The complete reconstructed package passes the existing Engine compiler.

The measured corpus has **8,212 occurrences**, **2,561 distinct complete program
bodies**, but only **247 program IDs**. No ID-only deduplication is permitted.
The test uses **103 groups**, at most **921 symbols**, **58,524 name-manifest
bytes**, **9,983 bytes per program**, and **22,777 membership-manifest bytes**.
Expanded DSL source totals **3,198,372 bytes**, excluding declarations/metadata.
All current default limits remain unchanged. Test aliases are synthetic exact
bindings; they are not production names or inferred game mechanics.

Validation: all **153 Authoring library tests pass**, with none ignored, in the
bounded optimized test profile. This includes existing independent execution
vectors and malformed-input checks. Log: `runs/r2-whole-corpus-tests-20261022.log`.
The initial test review caught a placement gap; comparing only pre-assembly
programs would not detect assignment to the wrong same-context owner. The final
test independently checks the assembled result. This validates representation
conversion, not complete-build parity. No native original is newly complete.

**Source migration completed at `b89f8e4d`:** the release builder consumes
program-free metadata, maintained semantic names, packet manifests and `.rule`
sources through the existing packet APIs. Every empty/partial owner and both
distinct owner-local Life programs are preserved. The whole-corpus gate and
independent numerical vectors remain validation requirements. The immutable
release-reader extraction is implemented in this checkpoint. Next move the
connected offline Import modules out of the runtime dependency path, as detailed
at the end of this plan. Do not resume the historical succession chain.

Naming seeds under `data/owned/poe2/3887ae68/` include
`inherent-life-contribution/names.kdl`, family `bindings.json` files such as
`global-minion-gem-level`, `strength-life`, `minion-life-source` and `command-damage`,
and `attributes/policy.json` / `actor-baselines/native-policy.json`. Their explicit
role-to-address bindings are review evidence, not a current-release certificate:
validate every typed address and slot declaration against the final schema and
active registry, reject collisions, and disregard historical predecessor seals.
For content labels, join the final mapping to pinned catalog identities; labels
must not establish mechanics or authority. The maintained migration-source names
now cover retained programs; broader declaration naming and named-registry
filesystem CAS remain separate from this cutover.

Remaining consolidation can use three PoB and four CLI suites. CLI helpers
require explicit parent-relative bindings before their roots become modules.
Some shared helpers contain tests: move each unique
law into one registered module instead of multiplying its executions. PoB needs
extra care: 91 source-test roots spawn their own executable with test selectors.
Qualify those selectors after nesting and assert the worker actually executed;
a child that runs zero tests must not count as successful reference validation.

Concrete retained-chain retirement target:
`tests/owned_extension_cli.rs::local_recipe_data_publishes_evaluates_and_preserves_original_gaps`
still replays the long historical extension sequence, including callers changed
by the registry-snapshot extraction. Preserve its identity, publication and
coverage laws in the declarative harness, then delete the sequence and obsolete
helpers. It is regression evidence, not the current production source builder.

Remove implementation fingerprints coherently across Data identity, Import item
loading, CLI diagnostics and PoB adapter/extractor reports. Retain validated
content identities, explicit semantic versions, upstream pins and executable
comparisons. Tree extraction currently rejects changes to its own Rust source via
`TreeSourceIdentity.extractor_sha256`; remove that field and gate together and
rebuild the affected data instead of repinning source text. Report/evidence fields
have transitive consumers, including calibration attachments and item catalogs.
Archived acquisition integrity is distinct from a check requiring today's Rust
file to match history; preserve that distinction during removal.

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
systems. Their exit conditions are cumulative cutover obligations, not a demand
to finish all cleanup in one phase before starting any work in the next. After
the retained behavioral baseline is usable, prioritize the Authoring seam and DSL
compiler; continue independent suite, boundary and parser cleanup alongside them.
No new game-mechanic packets on the old path. Use existing real mechanics as
migration cases; do not expand content to work around a compiler limitation.
Independent crate/test/build work can run in parallel with explicit file ownership.

### R0 — Make verification usable and establish a reproducible baseline

**In progress.** Inventory test targets and live consumers; select the
minimum retained semantic fixture set from the current release and all five raw
originals. Remove implementation-source pins and replace them with behavioral
checks before mass mechanical edits. Consolidate support code and retained vectors
into shared Rust test support; complete generic manifest-driven construction in
R2. Convert integration targets to
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

#### CLI parsing convergence

**Implemented and locally tested; hosted confirmation pending.** All retained argument parsing now uses
`clap`, with its dependency centralized under `[workspace.dependencies]`.
The root `poe-optimizer` CLI and its command modules already used it. New CLI
work must follow the same standard; libraries continue to receive typed inputs.

Converted scope:

- `crates/poe-optimizer-authoring/src/main.rs` and
  `crates/poe-optimizer-authoring/src/release_cli.rs`: the
  handwritten dispatch, argument loops and help text for `poe-author rule fmt`
  and `release build` are replaced with one `clap` command tree.
- `crates/poe-optimizer-data/examples/seal_package.rs` and `profile_allocations.rs`:
  argument handling uses `clap`; profiling arguments are parsed and temporary
  allocations dropped before measured intervals, preserving allocator isolation.
- `crates/poe-optimizer-pob/examples/regenerate_game_data.rs` and
  `inspect_parser_programs.rs`: argument handling uses `clap`, retaining callback
  count, positive-ID and uniqueness validation before data/extraction work.
  These examples use dev-dependencies; no CLI type enters their library APIs.

Validation must retain OS-native paths, `--` handling, formatter stdin `-`,
required arguments, rejection of invalid/extra arguments and the intentional
formatter exit contract (0 success, 1 formatting difference, 2 failure).
Accept normal `clap` help/diagnostics rather than copying custom-parser quirks.
Keep bounded reads/decompression, syntax-only formatting and atomic no-clobber
publication unchanged. Exercise actual commands in Rust integration tests and
remove superseded parsing tests/code; no compatibility parser or fallback.

The Authoring commands pass seven Windows CLI tests and all ten release tests,
including exact bootstrap reproduction, no-clobber publication, stdin, bounded
reads, invalid/extra arguments before I/O and dash-prefixed paths. Unix also has
a non-UTF8 path regression. An initial assertion incorrectly required a usage
banner in every clap error; it now tests rejection before work without pinning
incidental diagnostics. Log: `runs/clap-authoring-tests.log`.

All four developer examples build; the three callback-argument tests pass. Actual
executables pass help, required/extra/unknown/duplicate-argument checks, callback
count/zero/overflow/uniqueness checks, OS-path/`--` cases, no writes on argument
failure, and existing-output preservation. The allocation counter's isolated
`--self-check` passes after parsing. No custom argv loops remain in `src/` or
`crates/`. Logs: `runs/clap-example-tests.log`, `runs/clap-example-build.log`,
`runs/clap-command-checks.log`. The optional PoB build still emits its existing
LNK4098 CRT linker warning; it is not a test failure.

Together with the corpus checkpoint, **170 Authoring tests** and **three callback
tests** pass, with no ignored cases; **22 example executable checks** pass.
Strict all-target Authoring Clippy and focused four-example Clippy pass. The
normal runtime dependency guard still excludes Authoring, KDL, PoB and Lua.
Formatting passes for all three affected packages, using the per-package approach
already used by CI to avoid Windows' workspace-wide command-line length limit.
Logs: `runs/clap-authoring-clippy.log`, `runs/clap-example-clippy.log`,
`runs/clap-runtime-dependencies.log`. This guard does not prove retirement of the
legacy Rust data/parser stack, which remains explicit R1/R4 work.

**Exit:** every retained Rust CLI entry point uses `clap`; documentation and
help agree, parsing failures perform no command work, and focused Windows/Linux
tests pass. This commit's hosted Windows/Linux results remain to be recorded.

### R1 — Separate authoring and define one compact release

**Immutable validation prerequisite completed at `12c18b5d`:** registry snapshots
separate checked history from allocation, and checked recipes retain the shared
executable with its stored-source binding. Metric loading reuses those handles.
The checkpoint passed 1,722 selected checks; its detailed evidence is in Git and
`runs/immutable-recipe-*.log`. The ordinary source-action CLI regression added
there remains required during this cutover. No native original became complete.
**Immutable reader and role cutover (current checkpoint):**
`load_owned_release_artifacts` returns `CheckedOwnedRelease`, retaining the checked
recipe, import indexes/policies, evaluation packages and ordered queries. It uses
the same complete constituent validator as offline assembly. Borrowed typed views
preserve canonical input commitments; the runtime path does not reconstruct an
`OwnedReleaseInput`, invoke assembly, or allocate a second encoded publication.
Canonical serialized chunks are compared directly against the supplied bytes.
The host still rejects unsafe/non-file paths and bounds reads; the library checks
its own complete byte inventory, hashes, bindings and aggregate limits.

Recipe manifests and release claims are derived from checked packages. Declared
coverage, optional support groups, exact source unions, historical source-pin
subsets, query order and cumulative query-policy budgets remain binding. Recipe
canonicalization now updates its commitment without compiling the same executable
again. Offline publication/revision commands explicitly request input reconstruction
or staging; this does not put those operations back into runtime consumers.

Runtime role contracts/indexes/validators moved to `owned_skill_roles`; offline
`owned_skill_catalog` generation imports them in one direction. All consumers use
the new module directly, with no compatibility re-exports. The extraction preserves
wire data and the 15 audited validation/derivation sections; evidence is in
`runs/role-split-preservation-check.json`.

The dependency audit also found runtime DTOs and read helpers owned by offline
modules. `NamedQuerySet` now lives beside `ImportQueryTemplate` in normalization;
the immutable evaluation containers are named `CheckedOwnedReleaseEvaluation` and
`CheckedOwnedReleaseSupport`. `owned_release_loader` owns CLI filesystem loading,
and `owned_io` owns shared bounded reads/errors. Runtime consumers no longer call
publication modules for these operations. The closed inventory validator is shared
by filesystem and in-memory readers, before any declared path is opened.

Found validation work is implemented in the existing release suites: manifest
forgeries with fresh outer hashes, constructor-sortable but noncanonical data with
fresh nested and outer hashes, false receipt claims, query order changes under
stale commitments, strict fields/omission rules and exact aggregate limits. Full
support loading and an explicit 2 MiB execution-stack control accompany these
checks. Final-source Import and Authoring all-target tests pass: 1,893 tests
(1,714 Import and 179 Authoring; one existing ignored Import authoring utility).
Strict Clippy passes for both libraries/all targets and the CLI binary plus ten
affected integration targets. All 103 changed/new Rust files pass formatting;
the optional PoB/reference caller audit found no stale feature-only references.
Evidence: `runs/checked-release-final-library-tests.log`,
`runs/checked-release-library-clippy.log`, `runs/checked-release-cli-clippy.log`
and `runs/checked-release-format.log`. The CLI binary and six focused integration
targets pass all 48 tests, covering release/source-action loading, evaluation,
effects, support metrics and normalization. The default runtime dependency guard
passes with no Authoring/KDL/PoB/Lua dependency. Evidence:
`runs/checked-release-final-cli-tests.log` and
`runs/checked-release-runtime-dependencies.log`. This is 1,941 passing tests on
final source; it is not a full root all-target run. Protected untracked
incoming-critical work remains unchanged. No game data, original-build coverage
claim or release identity changed in this checkpoint.

Move the maintained allocator and recipe/release publication core into
`poe-optimizer-authoring` (Rust library + developer commands). Retire historical
transformation and catalog-family compiler chains after transferring their useful
behavioral assertions; do not relocate obsolete APIs merely to preserve test
machinery. Keep required optional-acquisition DTOs and structural validators in
neutral contracts. Runtime Import keeps
bounded decoding, external-format normalization and owned project construction.
PoB extraction/reference tooling is optional. Remove Import -> legacy Engine
coupling by replacing required parsing with owned import recipes, not by moving a
complete Lua emulator into a permanently required compat crate.
Alongside the DSL work, parse external XML once through a bounded checked boundary
and pass the parsed document to projections. Type item fields and split parsing
stages. This parser cleanup is required by R4, but does not block R2 unless a
concrete compiler/import dependency requires it. Replace
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
work and memory separately. Add a warning/CI regression guard at 80% budget
utilization (20% remaining headroom), with
an explicit review process for budget changes, not an unbounded size bypass.
Stop tracking concatenated duplicate packages; use canonical review artifacts
and a content-addressed runtime release generated by CI or fetched by hash.

The tracked `migration-source/release.kdl` now captures the final semantic
composition: registry, schema/rules/routing, Import catalogs/policies, five query
sets and 227 historical provenance rows. Those rows are metadata, not a plan to
replay past stages. This bootstrap includes the later flat-Life membership/policy
refinements and generated-Firebolt policy replacement that were absent from the
older tracked constituents. Replace this temporary JSON/gzip capture with
maintained declarations and the still-missing evaluation bundle. Keep the single
existing release assembler and shared Host publisher; do not introduce another
succession chain.

The current-release census contains 8,212 program occurrences, 2,561 distinct
serialized programs across 2,631 nonempty owners, and 18 integer tables with 975
rows. Every program now passes the whole-corpus Rust KDL/DSL migration gate and
complete Engine compilation. The schema has 9,391 definitions and 3,873
slots: use bounded per-source symbol groups rather than assuming one default-size
names manifest fits; the tested groups require at most 921 symbols without raising
limits. All eleven core/Import constituent files differ from the
tracked `current/` baseline. The production `poe-author release build` path reads
the final captured composition without local `runs/` or historical stage helpers.
Next replace its temporary constituents with declarations.
Preserve all 5,827 partial rule owners, five partial routing outputs and the
missing evaluation bundle as explicit limitations during that conversion.

**Exit:** clean-clone release construction and validation are ordinary CI, with
relocatable artifacts and no developer-local inputs. Unsupported old wire forms
fail clearly; the temporary offline migration reader is isolated and scheduled
for deletion at R4. No compatibility reader enters the normal CLI/library path.

### R2 — Formula DSL and declarative authoring

**Retained program source cutover implemented:** complete program syntax, exact
symbols, raising/formatting, name/membership KDL, whole-corpus conversion gates
and production release composition. All retained program occurrences now come
from tracked DSL; normal release construction rejects embedded program members.
Broader declarations and historical publication-consumer retirement remain.
Complete the shared behavioral harness; do not create a second evaluation model.
Unchecked compiler output is not a supported release format or a substitute for
full package validation.

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

The existing Engine accepts forward node references and canonically sorts reads
and nodes by ID. Its retained `canonical_indices_ignore_declaration_order_but_keep_effect_ledger_order`
test establishes that declaration order is nonsemantic while effect ledger order
is semantic. Raise in deterministic topological order (stable ready queue), retain
every node, and compare keyed graphs using the explicit old-to-new node mapping.
Do not compare raw node vectors or expect content digests to survive node renaming.
Use short disjoint read/local aliases, existing exact symbol names, round-trip finite
numeric literals and a byte-bounded writer. Reject duplicate/missing/cyclic
references; the complete package still passes the one existing semantic checker.

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

1. Read this plan and Git status. Next **retire obsolete transformation paths and
   move the maintained publication core into Authoring**. Do not blindly relocate
   the historical builder closure. The maintained `release.rs::build_release`
   decodes final constituents, composes declared rules and calls the sole release
   assembler. Production Authoring does not use succession, revision/migration or
   catalog-family transformation chains. Named allocation is the other maintained
   mutable dependency. These are the roots for the next dependency cut.

   Preserve still-relevant behavioral assertions in the declarative release/DSL
   harness, then delete the historical succession, recipe extension/membership
   patch, schema/Gem refinement and release revision/migration paths and their
   obsolete CLI commands/helpers. Review family compiler callers against maintained
   acquisition/authoring; do not preserve a compiler solely because a retired test
   chain calls it. Required gameplay/validation evidence must survive; historical
   transition formats, predecessor seals and compatibility-only tests need not.

   Existing Authoring gates already preserve exact program graphs/owner placement,
   non-program constituents, historical receipt restoration, publication atomicity
   and allocation stability. Before deleting the old closure, transfer independent
   arithmetic/rounding/coefficient-change and missing-input assertions from
   `tests/owned_extension_cli.rs`, pending coverage for all five originals,
   source grammar/condition/role and quality checks, exact occurrence provenance,
   the independent Cleric native regression and acquisition authentication.
   Register these directly in the declarative harness. Then delete predecessor,
   stage and republish helpers and obsolete transition-count fingerprints.

   Start with `crates/poe-optimizer-authoring/tests/release.rs`: share its tracked
   release loader, reuse `Sources::{build,replace_plain}` and the checked compiled
   rules, and transfer the nine top-level numeric vectors plus empty-input
   `Unresolved` assertions from `tests/owned_extension_cli.rs`. Preserve the
   quality-divisor change (8 to 10, expected attack rate 2.14) as a DSL edit found
   by its named program header, with both refreshed and stale `expected_input`
   cases. Normalize all five originals directly with the current checked release
   and ordered queries. Reuse the source/Gem/skill/provider/output correspondence
   laws in `tests/support/owned_selected_actions.rs`; obsolete assertions that all
   items/allocations remain pending must disappear. This first slice does not
   replace the old test's nested `check_*` calls or activate the ignored Cleric
   regression: give those remaining laws direct homes before deleting their parent.

   Move the mutable allocator and sole recipe/release publication machinery into
   Authoring. Keep checked recipe/release packages, DTOs, manifests, receipts,
   commitments, runtime indexes and the single validators in Import. The runtime
   reader, role contracts, ordered query DTO and CLI read helpers are separated.
   Runtime commands keep normalization, action resolution and native evaluation;
   retained authoring commands belong in `poe-author`, with direct imports and no
   compatibility wrappers. Deleting obsolete consumers avoids creating the
   Import -> Authoring -> Import cycles that a staging-only move would otherwise
   introduce.

   Resolve the known private seams explicitly: delete offline normalization
   `rebind_*` and source-action `rebind_input` when their retained callers are gone;
   share required catalog/role validation budgets without copying validators;
   keep allocator mutation state in Authoring and checked snapshot internals
   immutable; expose narrow read/construction APIs for checked views and manifest
   generation. Mixed catalog error/limit naming can converge during the same cut.

   Optional PoB exporters still provide useful acquisition. Their intrinsic/item/
   profile catalog DTOs and structural validators must stay in neutral contracts
   when compiler code is retired or moved, avoiding PoB -> Authoring -> PoB cycles.
   Per-build `owned_augments` and augment reconstruction remain Import concerns.

   Acceptance: no offline builder or mutable allocation in runtime dependencies,
   one working source-driven `poe-author` release build, preserved identities,
   coverage, metric/source-action results and malformed-release failures. Retain
   whole-corpus DSL and historical non-program artifact gates, but not historical
   publication machinery merely to run them. Legacy Data/Engine default features
   remain an R1/R4 deletion gate. Named-registry filesystem CAS, broader schema/
   table declarations and the compact runtime codec remain separate work.

   Focused source verification remains `cargo test --locked -p
   poe-optimizer-authoring --all-targets` with CI's bounded optimized profile.
   Follow the `clap` standard. Continue R0 source-pin removal and CLI/PoB suite
   consolidation only in disjoint files, retaining actual upstream/content
   authentication. Serialize all Cargo jobs.
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
