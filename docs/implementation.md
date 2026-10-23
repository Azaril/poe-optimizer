# Implementation plan and resume point

Updated: 2026-10-23 (EDT).

This is the only active delivery schedule. The [architecture](domain-architecture.md)
defines the end state; [documentation index](README.md) explains authority. The
[owner-provided review](architecture-review-2026-10-22.md) is adopted with the
adjustments below. Historical checkpoints remain in Git and
[implementation history](implementation-history.md), not in this resume point.

## Current state

**Next work is the DSL/data/runtime migration, before further mechanic content.**
The first complete unchanged build must use the new format and sole native path.
Legacy deletion is a gate before that milestone, not work deferred until five builds.
The computed-support unused-input repair, immutable typed Stat declaration compiler,
original-source dependency validation and complete Import content construction
under R2 are implemented. Typed Stat staging uses explicit persisted names and
packet-local symbols in the same compiler. The current checkpoint constructs
evaluation packages from authored content against the final checked recipe,
alongside rule/routing and Import dependencies. Runtime headers are rejected in
the replaced evaluation source format; complete final validation remains mandatory.
Authored and runtime evaluation share one aggregate preflight traversal.
Atomic authoring checkpoint publication now integrates declarations with the CLI,
using an explicit bootstrap or checked immutable predecessor and one persisted
name ledger. Initial skill-type declarations now use that path: 69 generic
Boolean Skill Stats and one Sniper producer, retaining Partial coverage.
Support-source checks now include declared potential outputs even when no Action
is queried. Gem domain proof tests and exact saved-selection finalization are
part of the maintained release harness; the replaced Gem publisher is removed.
Fifteen additional intrinsic source domains are now declared in the existing
metadata component: eight Sniper/minion owners, the Ascendancy and two roots,
and four pure UsagePolicy projections. No IDs or numerical coverage are added.
The root/ascendancy proof laws now run in the maintained release harness; their
five replaced test/helper files and historical publishers are removed.
Continue independent harness retirement while completing the current release's
execution declarations. Do not add a new provenance API
for dormant support flags.

Checkpoint triage follows the owner's October 23 clarification: complete the
clean data-model/KDL migration and retire replaced paths, reassess the closest
unchanged build, complete it, and demonstrate one native optimization pass before
expanding supported builds and search breadth. Track every discovered task, but
defer work that does not protect the model or unblock that sequence. Deferred
debt keeps an explicit owner milestone and acceptance condition; it must not
silently become a second supported implementation.

| Area | Verified state at this checkpoint |
| --- | --- |
| Code baseline | Semantic baseline `fa49fcea32f0413cde5f5983e179c34e6c7acbdf`; whole-program DSL source cutover `b89f8e4d`; checked runtime loading `a6dc1991`; maintained-release behavioral harness `a7069aa1`; unused computed-support repair `1a88f37c`; typed Stat declaration staging `dde32fef`; shared source checks and defence laws `8fd6ab5c`; content construction, nested budgets and metadata laws `22899910`; private normalization owner `8e93cd7f`; complete Import construction `96e5533e`; declaration integration `f5e5f49d`; evaluation construction `15d93996`; atomic authoring publication `509f2008`; first production declarations `caca40a5`; canonical source reference `b815f0c5`; potential-output census and harness transfer `e59f8c88`; intrinsic Sniper domains `5f3305cc`; current checkpoint adds seven root/UsagePolicy domains and retires their replaced publishers |
| Complete native originals | **0/5**; selected input issue counts **123/137/127/144/5** |
| Closest build | Original05 Sniper; five input obligations are not its entire remaining numerical/legality work |
| Native engine | Checked typed IR, explicit-stack VM and occurrence/effect graph exist; no native optimizer integration |
| Authoring | Bounded Rust DSL, official kdl-rs manifests, exact typed names/slots, transactional appends, stable named allocation, typed Stat staging, complete Import and evaluation content construction. The CLI now publishes immutable authoring checkpoints with explicit bootstrap/predecessor selection, KDL declaration sources and a fully checked persisted ledger. Temporary Authoring -> Import dependency reuses the sole allocator/assembler; no runtime -> Authoring dependency. Move the maintained publication core and retire obsolete transformation chains |
| Runtime packaging | Legacy Data/Engine default features and embedded legacy snapshot remain; default builds are not yet free of that stack |
| Loader extraction | Checked runtime reader, shared complete-release validation and original-source dependency checks before DSL lowering implemented; no publication reassembly in metric/source-action loading. Runtime roles are separated from catalog generation. Final dependency construction and offline module/command relocation remain |
| DSL / KDL / binary pack | Every retained program comes from tracked `.rule` sources with semantic KDL names and membership packets. The normal builder rejects embedded programs. Typed Stat declarations publish through the checkpoint CLI. Remaining descriptor kinds and binary pack remain pending; mutable-head CAS is deferred until a head exists |
| Current release | Tracked `migration-source/release.kdl` composes program-free metadata and DSL through one compiler/assembler. Production declarations add 69 Stats, one Sniper initial-type program and fifteen intrinsic support-source domains. Historical restoration admits only those exact deltas. There are 1,001 source domains (972 Known, 29 Unmapped). Fixed first-allocation genesis is the predecessor for later builds; evaluation bundle still absent |
| CLI parsing | All retained Rust CLI argument parsing uses the shared workspace `clap` dependency. Authoring command/release tests, example builds/tests and executable behavior checks pass locally; hosted confirmation remains pending |
| CI | At 18:06:17 UTC on October 23, `5f3305cc` run `39998928097` remained pending with no created jobs. Earlier `96e5533e` run `39962517895` had sixteen of eighteen created jobs passing and two running, with no failures; only Windows CLI-1/2 remained running. Both PoB jobs and Windows CLI-0/3 had passed. Baselines `a7069aa1` and `ae90ec82` completed all twenty jobs successfully. Workflow-level main-branch serialization preserves the running revision and coalesces pending revisions. Baseline success does not certify later commits |

Current reproducible source: `data/owned/poe2/3887ae68/migration-source/release.kdl`.
First reproduce the allocation predecessor using the
[canonical genesis reference](../data/owned/poe2/3887ae68/authoring-genesis.md), then use
`cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor genesis-checkpoint --expected-predecessor e89ae92ac3895a3e9f38591452d340b8c640d4af737a544cc18759de4d98b109 --output rebuilt-release`
with a fresh output directory; runtime consumers use `rebuilt-release/runtime`.
Successful publication prints the checkpoint identity to
stderr and checkpoint receipt JSON to stdout. Bootstrap reproduces the fixed first-allocation
source revision only; later builds must preserve its ledger as a checked
predecessor. Current runtime input identity:
`c05e48a46bbfa433622fe5cdec25bf228aa99c1ab97d4d22f096ef02b607991f`.
The 2,787 declared source files use 5,784,276 encoded and 58,583,076 decoded
bytes, within the existing 64 MiB bounds. The 183,544-byte root manifest and
expanded rule bytes also fit current limits without increases.
Metadata JSON remains temporary; gzip is not the compact runtime format.
The CLI still reports the 80% decoded-size warning; compact-format work remains
required before expanding content. The production command rebuilt the tracked
source into `runs/root-usage-domains-successor-01` with the expected input identity,
17 constituent artifacts and all 227 provenance rows.

Historical input
`a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`
and its receipt remain independent regression evidence. Original rule IR lives
only in Authoring's `tests/fixtures/retained-rules.json.gz`, outside production
inputs. Actual-source graph checks plus restoration of just historical program
members, with only the exact 69-Stat/one-program/fifteen-domain delta removed through
maintained content constructors, prove all historical metadata/provenance and artifact bytes
are preserved. `runs/dsl-conversion-02/conversion-report.json` records local
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
`tests/owned_extension_cli.rs::remaining_historical_family_laws` still replays the
long historical extension sequence. Top-level arithmetic/data-mutation checks and
the five-original coverage/action-origin checks now have direct homes in Authoring's
release suite; the duplicated old arithmetic/replay block is removed. The Cleric
whole-plan regression still needs its older staged fixture until current readiness
declarations exist. Preserve remaining semantic laws in that
same harness, then delete the sequence and obsolete helpers. It is regression
evidence, not the current production source builder.

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

Local iteration finding, October 23: separate Authoring/Data/Import root selections
change the proc-macro feature union. Current fingerprint evidence shows identical
compiler/profile/config/rustflags but different `serde_derive`/`thiserror-impl`
dependencies through `syn` with and without `full`. Locked `clap_derive` 4.6.4
enables `syn/full`; Authoring's normal Clap dependency and Data's dev dependency
include it, while the selected Import tests do not. This accounts for repeated
Data/Engine compilation in the current serial validation sequence; there is no
evidence of a sandbox/elevation cache-path cause.

Before the next broad shared-library verification, measure a combined selected
harness compilation under one feature union, recording Cargo timings and exact
executable/test inventories. Execute only the same required suites. Do not blindly
combine `--lib` across all three packages: that also selects Data's legacy library
tests. Do not add artificial production dependencies or change test semantics to
stabilize the cache. CI uses intentionally separate package jobs/cache keys; this
local finding alone does not justify changing its parallelization.

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

**Direct behavioral harness checkpoint:** Authoring's existing `release` target
shares one cached tracked-source build and checked executable. New registered
modules cover nine independent local arithmetic vectors and missing-input refusal,
a DSL divisor change (2.16 to 2.14) with stale-commitment rejection, all five
unchanged XML originals with 110 ordered queries, and four exact action origins
(including the current Actor-supplied attack-Skill grant).
Three further tests transfer catalyst selection/lazy input laws, seven local
contribution chains and four receiver results, plus eight exact elemental channels
with independent rounding values, units and repeated/parallel scratch isolation.
Current metric identities are resolved against the twenty reviewed requested-Metric
bindings; numerical evaluation remains unavailable because the release has no
evaluation bundle. Draft/provenance replay must be deterministic and build coverage
remains incomplete. These are component/import laws, not full-build numerical parity.
The duplicated extension arithmetic/coverage/replay block and the three transferred
numeric helper implementations are removed. Their historical outer canonical-slot,
wire-budget, publication and replay assertions remain until transfer; duplicate
compilation in the two old wrappers is gone. Scratch is reused through the success,
missing-input and changed-input sequence to detect stale cached values.
All **185 Authoring tests pass** (154 library, seven CLI, 24 release), with none
ignored. Final strict all-target Authoring Clippy and the retained CLI target's
all-feature Clippy pass. The historical target passed **two tests**, with its one
existing manual reproducer ignored, before removing the three numeric helpers;
final-source review confirms its other assertions remain and final Clippy covers
their wiring. This was not a second full runtime replay after those deletions.
Logs: `runs/current-release-laws-tests-final.log`,
`runs/current-release-laws-authoring-clippy-final.log`,
`runs/current-release-laws-retained-clippy-final.log` and
`runs/current-release-laws-retained-tests.log`.

**Found execution migration blocker:** the historical Cleric whole-plan test uses
operations-v11. Current operations require checked readiness, and the current
release has no evaluation bundle. Complete owner-qualified program classification
is needed by `OwnedEvaluationStages`; required-input partitions must be complete
only for Skills explicitly listed for readiness. Unlisted Skills retain full gates,
and partial definition coverage remains admissible and evaluates as unavailable.
The public stage-aware plan also needs checked support preparation/input/receiving
packages (outputs optional), plus Metric bindings for a release evaluation bundle.
Source-property programs require explicit early roles, relation bindings and their
participating owner membership; they cannot simply be parked in Execution.
Existing fragments do not establish this for the whole retained corpus. Complete
the stage/readiness declarations under the existing model without claiming
numerical coverage. The separate unused-predicate demand bug below is now repaired;
do not add source-predicate authority that current supports do not require. Preserve
the existing Cleric whole-plan/parallel regression and its optional local reproducer
until authored declarations make the current-release replacement runnable. Do not
lower operation versions, narrow the current package, mark everything Execution,
or drop the plan/parallel assertions to make a test pass. Complete this prerequisite
before deleting their final historical fixture; it is required by the sole native
execution path as well as by test retirement.

Concrete source gaps: no tracked production `SupportInputBindingsInput` exists.
`effective-gem-inputs/authoring.json` supplies real source level/quality channels,
but existing fixture constructors use unused predicate Stats for target context;
those are not production bindings. `sand-preparation/source-properties.json`
declares Partial relation coverage, and its README explicitly says checked
receiving rejects it. Review and close those actual relations plus the real target
type/context bindings; header rebinding or relabeling Partial as Complete is not
completion. Preserve unavailable results for remaining mechanic coverage.

The deeper target-fact audit found no existing production Skill-type or origin
predicates. Definition-owned initial types fit existing rules. More immediately,
all 17 retained preparation rows (13 distinct supports) have `gems_only=false`,
`from_item=false` and `is_trigger=false`, yet computed-plan input gathering demanded
the three corresponding target flags unconditionally before this checkpoint. The
computed drivers now match the lower-level evaluator's conditional policy. Unknown
unused facts, required-fact failures, limits and meaningful failure causes are
preserved. `cannot_be_supported` still needs authoritative producers.
No pinned PoB2 skill definition sets `supportGemsOnly=true`; do not block this
milestone on hypothetical Gem-only support semantics.
The implemented bounded repair is a shared private adapter retaining at most four
existing unavailable-input causes while the existing preparation policy decides
applicability. It maps an actual applicability gap back through its exact origin
to the relevant saved cause, preserving hard errors and budget accounting. The
existing computed-support and direct/inherited-delivery suites cover the
investigation's failure and recovery cases. Replacement tests use two distinct
definitions linked by the existing upgraded-support relationship:
origin 1 replaces origin 0 at retained position 0, and only the replacement's
required flag explains the failure. Restoring that producer recovers preparation
and delivery while the replaced support's flag stays unknown. No public model or
production data changed; this does not complete any original build.
The first replacement fixtures incorrectly gave one effect two conflicting
definitions and were rejected by the existing validator. The corrected fixtures
use distinct effects and `plus_version_of`; production validation is unchanged.
Final Engine validation with `--no-default-features --lib --test owned` passes
**778 tests** (73 library and 705 owned integration), with the 15 existing optional
reference tests ignored. Clippy with `--no-default-features --all-targets -- -D warnings`,
formatting and diff checks pass. Independent review found no
production defect. The existing Windows LNK4098 linker warning remains; it did not
prevent linking or execution. Logs: `runs/support-input-demand-native-tests-final.log`
and `runs/support-input-demand-native-clippy-final.log`. The nine pre-existing
untracked incoming-critical files still match their saved backup hashes and are
excluded from this checkpoint. Release source/input identity and the **0/5**
complete-original count are unchanged.

**Typed Stat declaration checkpoint:** `packet.rs` currently appends programs;
its names must already exist in the schema decoded by `release.rs`. The tracked
bootstrap has only three Boolean Stats targeting Skill (Offering activation,
Frost Bomb requested global effect and participation), with none for initial
types, minion-context presence or support prohibition. Authoring now has maintained
KDL Stat declarations for all four existing value kinds and all seven existing
target kinds, using transactional named allocation before program lowering and
the existing schema/rule validators. This is the planned broader-declaration
work, not a new evaluator ownership model. Do not
predict IDs, hand-edit compressed schema bytes or create another publication tool.
The generic Stat schema remains the sole descriptor model; Boolean is the first
production need. The compiler stages names, schema and symbols immutably, refuses
existing descriptor replacement, checks active base identities and overlapping
alias agreement, and preserves all coverage/authority checks. Eight focused tests
cover syntax, persisted allocation replay, earlier-sorting additions, conflicts,
exact budgets, transactional failure and ordinary Engine execution with true,
false and missing inputs. This is a library seam, not yet release-command wiring.
Rebuild and validate every affected constituent commitment through maintained
Authoring before claiming that production data uses these declarations.
The Data package exposes cached budget validation and a stored-entry growth
preflight. New Stat descriptors and target memberships are charged before
allocation; semantic-link checks retain their independent ceiling. Existing
schema construction and encoded-byte validation remain mandatory. Review caught
the initially late entry check; the Actor-template regression now proves exact
growth capacity, one-over/overflow refusal and an already-invalid baseline.

Final validation passes **250 native Data tests** and **195 Windows Authoring
tests** (162 library, seven CLI and 26 release), with none ignored. Logs:
`runs/stat-declarations-data-tests-final.log` and
`runs/stat-declarations-authoring-tests-final.log`. The first Authoring run
correctly rejected a new fixture whose declared names did not match its DSL
references; that fixture is corrected without changing name resolution. The
final optimized Authoring compilation still took **10m25s**; suite consolidation
has not removed the remaining compilation cost. Production source/input identity,
partial coverage and the **0/5** complete-original count remain unchanged.

That declaration checkpoint identified two publication prerequisites: persisted
`NamedIdRegistry` state and separation from the closed runtime directory, which
cannot accept an extra authoring ledger. The [authoring checkpoint proposal](owned-authoring-checkpoint-proposal.md)
records the accepted offline workflow: an atomic authoring checkpoint with a
separate runtime directory. The owner chose this on October 23 over separate
source-ledger allocation. The later atomic-publication checkpoint below implements
both prerequisites and the allocating CLI; independent test retirement continues.

The final assembler validates existing commitments rather than reconstructing
them. The historical `owned_release_revision::rebind_release_dependencies` assumes
a checked predecessor and rejects evaluation artifacts. Do not call it on raw
decoded source or recreate a second repair list. Shared bootstrap/import semantic
validation against the actual original registry and schema is now implemented
(checkpoint below). Maintained Authoring now constructs final dependencies
after declaration allocation, then validates the complete release. Preserve the
malformed incoming commitment and late-failure checks.
Retire the old transformation caller when its useful tests and binding logic have
moved; none of this may silently grant missing gameplay coverage.
The initial-type producer for Original05 Sniper now follows the pinned
`Data/Skills/act_int.lua:18677` declares twelve own types and five minion types.
Use `support-release/evidence.json` raw Twister/Cleric/Heal memberships and
`djinn-support-preparation/source-vectors.json` initial definitions for independent
breadth controls; final observed types are not initial data.
The five retained preparation sidecars share a 34-type vocabulary, which is not
the game's complete vocabulary. Sniper's CreatesUndeadMinion,
CreatesSkeletonMinion, MultipleReservation, Chaos and Fire are outside that set.
Keep the raw source facts and make any finite projection explicit; author positive
and negative membership only within the declared vocabulary. A labelled synthetic
component control may establish present-empty versus absent semantics if the
pinned natural examples contain no empty context. These remain reusable Skill
Stats and definition-owned DSL producers, with no build-name dispatch.

The implemented first declaration slice uses that existing 34-type projection: 34 own-membership
Stats, 34 minion-membership Stats and one explicit minion-context presence Stat.
All 69 are generic Boolean Skill Stats; seven own and five minion memberships
are positive for this definition. Declaration/name/rule sources append
the producer to `skill.sniper_skill` in group-000 using declaration overlays,
without predicted IDs or Rust mechanics. Raw excluded names remain in evidence and
owner coverage remains Partial. Independent controls distinguish own Attack=false from
minion Attack=true and own Physical=true from minion Physical=false. Exact repeated
Skill recipients and missing-producer plan controls belong with the upcoming actual
execution bundle; the current slice does not construct a parallel fixture bundle.
The validated first source revision and checkpoint digest are now pinned in the
canonical reference, outside the compiled inventory, to avoid self-reference.
Reproduce only that fixed genesis
with the maintained compiler, verify/load it normally, then use it as predecessor.
Before a second allocation generation needs a new predecessor, R3 must distribute
one complete checked snapshot. Do not grow a historical source-replay chain;
local checkpoint outputs are evidence only.

The smallest current producer needs no level, quality, count or support-result
read. Existing group-100/group-101 programs already produce prepared level/quality
channels; group-000 owns physical Sniper raw/final projections. The later bundle
must classify the full actual program/application inventory and define freezing,
readiness, receiving and metrics. Do not copy the old fixture's complete-empty
support census from `sniper-final-inputs/source-properties.json` into Original05;
its unresolved support discovery remains real. Likewise, the Partial census in
`sand-preparation/source-properties.json` cannot be promoted by relabelling it.
Initial-type membership does not establish summoner presence or support
prohibition; those still need their actual occurrence/definition authority.

Keep support prohibition as a separate evidence/ownership task. The current Djinn
snapshots omit nil `cannotBeSupported`; omission does not establish a native false
producer, and occurrence-specific `noSupports` must not prohibit the shared Skill.
Acceptance for the declaration/type slice includes positive and negative membership
within the declared vocabulary, absent versus present-empty minion context,
repeated occurrences and unchanged partial-owner coverage. Source census, stages
and receiving remain separate bundle prerequisites.

**Scalar/attribute harness transfer completed:** the cached Authoring release suite now
checks 21 scalar families and four Actor attribute families, preserving independent
values, exact units/recipients, signed rounding, missing inputs, bounds/overflow,
scratch recovery and parallel workers. Current Actor attributes contribute to
both first- and second-pass Count channels after checked integer quantization;
the older single-Integer receiver expectation no longer describes the maintained
model. The new expected channels agree with `attribute-stages/COUNT-CUTOVER.md`.
The first transfer run exposed that obsolete expectation; the corrected tests
pass without changing production rules or relaxing validation.

Removed `check_scalar_numeric`, `check_components`, their private helpers and
exact callers from the five historical scalar/attribute publication helpers after
the replacements passed. Retained outer publication/import/schema/closure/wire
checks and Cleric whole-plan checks remain explicit retirement work. The four
separate defence-family numeric checks are transferred in the checkpoint below.
The historical chain has not been deleted or
rerun in full at this checkpoint; its affected target is compiled/linted below.

Native-only all-target lint exposed two Data examples that imported legacy APIs
without declaring that feature requirement. `seal_package` and
`profile_allocations` now declare `required-features = ["legacy"]`; native-only
lint passes and both examples still pass strict lint with their default feature.
This corrects the existing feature boundary, not the R4 removal of those tools,
the legacy default or implementation-source fingerprints.
Strict Clippy passes for Authoring all targets, native-only Data all targets,
both retained Data examples and `poe-optimizer-cli --test owned_extension_cli`.
Logs: `runs/stat-declarations-authoring-clippy-final.log`,
`runs/stat-declarations-data-clippy-final.log`,
`runs/stat-declarations-legacy-examples-clippy.log` and
`runs/current-release-scalar-retirement-clippy.log`. Formatting and scoped diff
checks pass. Independent reviews found no remaining blocker in declaration
staging or transferred laws. The nine pre-existing incoming-critical files remain
unchanged and excluded from this checkpoint.

**Original-source dependency and defence checkpoint:** `build_release` now decodes
the complete source input and checks its original registry, schema, rule header,
routing, Import policies and source pins before DSL lowering. Recipe input bounds,
active descriptor membership and Import constituent construction are shared with
the existing runtime path; runtime validation order and final rule/evaluation checks
are preserved. At checkpoint `8fd6ab5c`, source checking returned only a checked
schema; the later content-construction checkpoint below retains its checked Import
owners as well. It never repairs incoming seals. Production declaration integration
and the publication workflow decision remain pending.

Tree already separates authored content from `bind_new`. Mapping, Roles, Rewards,
Items and ItemSource now have their own explicit content types, consuming checked
extraction and binding constructors. They derive dependency seals from checked
arguments and reuse existing validators; source pins, compiler provenance and
coverage are preserved. Current item constructors refuse implicit upgrades from
older contracts. Normalization's nested role/direct-input/item/reward/configuration
commitments now have owner-local content construction, joined by one aggregate.
Authoring installs the resulting raw constituents and uses ordinary assembly;
no nested seal repair is spread through that crate. Old evaluation
seals are deliberately outside the source prefix check: after a new-Stat program
is introduced, an old-schema executable may not exist to authenticate them.
Construct evaluation artifacts from explicitly authored content, or inherit only
from an independently checked release; never treat prefix-check success as
permission to overwrite evaluation commitments. At that checkpoint evaluation
content construction was still pending; the authored-evaluation checkpoint below
now provides that API and source-builder integration.

The maintained normalization source has every top-level optional subpolicy
present; its usage inventory is present while configuration-disposition inventory
is absent. Construction must preserve both present and absent/Unconverted forms.
The dependency order is: schema to quality/gem inputs; schema/roles to source-action
correspondence, Direct/generated inputs and role inventories; rebound scalar
inputs to usage then gem inventory; Direct to support targets and skill inventory;
rewards to configuration reward inventory; those results plus encounter/enemy/
configuration inputs to configuration dispositions; schema/items/item-source to
equipment, modifier and parameter inputs; equipment/mapping/tree **content** to
passive sockets; complete normalization to the Tree package, then final socket
validation against that Tree. The content digest avoids a tree/normalization cycle.
Source-action correspondence inside Direct and Gem disposition rows must be
constructed by `owned_source_actions`, not missed as a nested field. Preserve
historical source pins, catalogs, `mapping_source` and configuration source-census
identities as evidence; they are not current schema seals to refresh.
The source checker retains the actual checked Import owners and consumes them
into authored content. Its connected normalization subgraph is constructed in
the maintained source builder. A raw seal-stripping helper remains unsafe. Its
historical replacement targets are the normalization segment of
`owned_release_revision::rebind_release_dependencies`, eight exported normalization
rebind helpers and nested `owned_source_actions::rebind_input`. These are retirement
targets, not callers to retrofit with the new content API. Maintained Authoring
does not invoke revision/migration; transfer their remaining relevant laws into
the maintained harness, then delete the obsolete callers and helpers. This gate
includes historical modes in `src/owned_release_cli.rs`, root CLI tests and their
succession helpers, not only Import's own tests. A constructor alone cannot claim
the repair chain retired. A leaf-only
SourceAction constructor would leave the same repair authority and a temporary
parallel path. Test unchanged identities, appended Stats, stale nested seals,
absent tree, optional/Unconverted preservation,
pending coverage and bounded transactional failure before removing those helpers.

The checked normalization policy now has a private owning package in the existing
constituent-validation pipeline. Its policy and digest are kept together only after
all existing dependency checks succeed; release readers and staged accessors use
that owner. Validation order, wire formats and coverage are unchanged. No second
standalone validator or public constructor was added. Its consuming extraction now
feeds one normalization-content aggregate, with dependency-bearing leaf types in
their existing modules and unchanged semantic DTOs reused directly. There is no
raw sealed policy remainder or default dependency field in that representation.
Source checking retains its actual checked Import constituents rather than
discarding and revalidating them solely for extraction. Such a source result certifies only its checked Import
dependencies; it must not imply checked rule programs or evaluation authority.

The current integration tests cover unchanged-schema byte identity and an appended
Boolean Stat with a real DSL program, immutable staging, complete assembly and
native Boolean execution. The maintained originals harness remains the all-five
draft/Partial-coverage gate. This library composition precedes production manifest
integration of declarations and the filesystem workflow decision. Next obtain rule
and routing identities through checked storage; do not compile the Engine twice
merely to compute dependency digests. Evaluation-content construction follows
with explicit authored inputs rather than a raw-seal overwrite exception.
Resolve new support-type Stats and frozen-channel references through the staged
`SymbolRegistry`. New owner-qualified programs require explicit stage/readiness
classification; do not predict IDs or infer a Complete partition from added rules.

Aggregate evaluation preflight is now repaired: it charges readiness parameters,
participation and program outputs; receiver applicability/properties; source
relations, generated occurrences, all relation collections, nested support
programs and generated/summoner paths. Hostile Partial gaps count before their
semantic rejection, and both path forms retain the existing receiving-depth
bound. Five regression tests use independently calculated subtree costs against
one baseline budget, exact/one-short bounds, failure before digest work and
separate final Data/assembly rejection. Limits and semantic validators are
unchanged; source-check success remains only structural evidence.

Six new regression tests cover corrupt original seals/source pins, retired schema
addresses, exact and exhausted budgets, rejection before malformed DSL, late DSL
failure, nested gem-policy binding and mandatory final evaluation validation.
The same cached release harness now checks the four remaining defence scalar
families with independent decimal expectations, both signs, exact units/recipients,
live versus dormant missing inputs, scratch recovery and parallel workers.
After that replacement passed, the old `check_legacy_numeric` helper and its
private numeric/Boolean helpers and caller were deleted from
`tests/support/owned_item_defence_inputs.rs`. Its outer import/layout/coverage/
publication checks remain pending transfer; Cleric whole-plan checks still need
the maintained execution declarations. No gameplay coverage is promoted.

Checkpoint `8fd6ab5c` passed **202 Authoring tests** (162 library, seven CLI and 33 release), with none
ignored. This includes maintained source/input identity, historical non-program
preservation and all-five original import/action provenance checks. Optimized
compilation took 8m40s; release tests took 13.64s. Log:
`runs/source-dependencies-authoring-tests.log`. Strict Authoring all-target Clippy
also passes (`runs/source-dependencies-authoring-clippy.log`). All **1,107 selected
Import tests pass** (246 library and 861 owned integration), with none ignored,
including checked release/recipe roundtrips, malformed bindings and budgets.
That is **1,309 passing checks** for this checkpoint. Import compilation took
11m30s and each test binary completed in under 40s; compilation cost remains R0
work. Log: `runs/source-dependencies-import-tests.log`. Strict all-target Import
Clippy and retained `owned_extension_cli` lint pass; logs:
`runs/source-dependencies-import-clippy.log` and
`runs/source-dependencies-retirement-clippy.log`. Scoped formatting and diff checks
pass. Independent review found no issue in shared validation or the transferred
laws. The nine protected incoming-critical files remain unchanged. This checkpoint
does not change current release identity, coverage or the **0/5** original count.

**Content binding and metadata checkpoint (`22899910`):** the five component constructors
reproduce unchanged current artifact identities and revalidate after an immutable,
symbolically allocated Boolean Stat append. Tests preserve source provenance,
compiler receipts, pending evidence and semantic content; corrupt members,
tighter limits and raw sealed inputs remain rejected. These component tests do
not claim that normalization/tree/evaluation have been rebound or that a complete
release can yet publish declarations. The normal release compiler is unchanged.
Three maintained-release metadata tests cover inert headers, duplicate IDs and
brace text, independently expected range value 12, tagged-line refusal, malformed
or misplaced headers and unsupported selectors/markup. All **208 Authoring tests
pass** (162 library, seven CLI, 39 release), with none ignored, including all five
original import/action-provenance checks and the unchanged release identity.
Log: `runs/content-binding-authoring-tests.log`. Strict all-target Authoring and
Import Clippy also passes (`runs/content-binding-preflight-clippy.log`). After the
replacement passed, the superseded `check_preamble_cases`, its caller and private
attribution helpers were removed from `owned_item_metadata.rs` (204 lines).
`metadata_only` remains used by `check_originals`; original-build and outer
publication checks are retained. All **1,118 selected Import tests pass** (246
library and 872 owned integration), with none ignored. This includes all five
aggregate-preflight regressions and six Mapping/Role content-construction tests.
Import compilation took 9m40s; the library and integration binaries took 36.50s
and 49.32s. Log: `runs/content-binding-import-tests.log`. Together this is **1,326
passing checks**. Strict retained `owned_extension_cli` lint passes
(`runs/content-binding-retirement-clippy.log`), as do scoped formatting and diff
checks. Independent reviews found no issue in the constructors, aggregate bounds
or transferred metadata laws. The nine protected incoming-critical files still
match their recorded hashes. Release identity, coverage and the **0/5** original
count were unchanged. Subsequent checkpoints below implement normalization
construction and the accepted filesystem declaration-publication workflow.

**Checked normalization ownership checkpoint (`8e93cd7f`):** the existing constituent pipeline
now constructs `CheckedNormalizationPolicy` after passive-socket placement checks.
The runtime reader moves this owner into `CheckedOwnedRelease`; canonical hashing,
receipts, authored-input reconstruction and staged accessors read the same stored
policy. The runtime path retains its existing single policy clone. Source-only
validation adds one clone after its existing aggregate and policy-byte bounds,
and at that checkpoint still returned only a schema; it did not gain evaluation authority. Independent
review found no validation-order, identity or coverage change. Strict all-target
Authoring/Import Clippy passes (`runs/normalization-owner-clippy.log`). All **19
focused Import release tests pass**, with none ignored; compilation took 8m07s
and execution 1.61s (`runs/normalization-owner-import-tests.log`). All **six focused
Authoring source-dependency tests pass**, with none ignored; compilation took
7m15s and execution 4.97s (`runs/normalization-owner-authoring-tests.log`). The
follow-up therefore passes **25 focused regression checks** plus strict lint.
Scoped formatting, diff checks and the nine protected-file hashes also pass.
No release identity, serialized data or original-build coverage changed. This is
the storage prerequisite for content extraction, not completion of that migration.

**Normalization content and maintained-builder checkpoint:** source checking now
retains the actual checked registry, schema and Import constituents. Consuming
extraction moves their authored content; it cannot strip another input's seals.
One normalization aggregate reconstructs all connected current commitments through
owner-local content types. The existing scalar, usage and configuration digest
helpers share borrowed prerequisite views, preserving their domains and serialized
field order. Tree-content commitments precede full Tree-package construction.
Historical source/catalog/census evidence, policy variants and absent values remain
unchanged. Four fields retain their explicit-null rejection. Authored content is
byte-bounded before conversion allocates replacement vectors. Review found and
fixed extra null serialization on seventeen absent fields: sparse content must
fit the raw policy's existing exact byte budget. A regression reconstructs and
validates the unchanged sparse fixture at that limit and rejects one byte less.

The maintained release builder uses this path on every build. Its constructed
Import inputs are explicitly raw; ordinary complete assembly still validates
normalization semantics, membership, source union, budgets, rule programs and
evaluation artifacts. Evaluation inputs are never part of source extraction or
rewritten by these constructors. No new runtime format or game rule is introduced.
The local Authoring numerical fixture now authors the current empty ItemSource
contract, with assertions that its source/layout tables are empty; no implicit
production upgrade or shared historical-fixture change was added.

Six new maintained-release tests cover exact identity/content reconstruction,
registry-allocated Boolean Stat plus real DSL execution with unchanged coverage,
absent/Unconverted policies and untouched evaluation, invalid provenance/nested
actions/membership, separate construction/final-validation limits, and rejection
of sealed content or explicit null. Existing all-five original tests remain the
corpus gate. Peer reviews found no remaining actionable issue after
the explicit-null control was restored. The full Authoring suite passed **214
tests** (162 library, seven CLI, 45 release), none ignored
(`runs/normalization-content-authoring-tests.log`). Strict Authoring/Import
all-target Clippy passed (`runs/normalization-content-clippy.log`). All **six
focused tests** pass after the final sparse-byte adjustment (7m37s compilation,
13.09s execution; `runs/normalization-content-boundary-tests.log`). All **1,118
selected Import tests** pass (246 library, 872 owned integration), none ignored:
9m47s compilation, 39.59s and 38.80s execution
(`runs/normalization-content-import-tests.log`). This is **1,332 passing tests**
across the full Authoring and selected Import suites, plus the six final focused
reruns. Final strict all-target Authoring/Import Clippy passes
(`runs/normalization-content-clippy-final.log`).
Scoped formatting and diff checks pass. The nine protected files retain their
saved hashes; release identity, coverage and the **0/5** complete-original count
remain unchanged. The next implementation step is production declaration
integration, with evaluation construction as the separately tracked dependency.

**Production in-memory declaration checkpoint:** the existing `build_release`
now takes optional explicit declaration context: persisted names, expected name
digest, typed declarations and their baseline symbol scope. The checked source
registry must accept the persisted registry as a successor before staging.
Packet aliases remain local; only current declarations are overlaid, and exact
ledger collisions, namespace separation and unique symbol budgets share the same
checks used by declaration staging. Reserved names without current descriptors
remain reserved and are not projected into executable scope.

The original source checker now retains actual checked routing plus the bounded
rule payload whose original header was authenticated. `ReleaseSourceParts`
replaces the tuple directly. Rule content reconstructs raw bindings; routing
content preserves its existing semantic version and calls the sole validator.
The compiler binds both to the staged schema before lowering and reconstructs
Import content through the same path. Evaluation commitments remain untouched.
Only complete assembly and the expected-input check can produce a `BuiltRelease`
with its staged name allocation. No persisted state changes, compatibility entry
point, alternate compiler, or allocating CLI input was added.

New tests cover native Boolean Stat execution, serialized-ledger replay,
earlier-sorting additions, stale predecessors, local/equal/conflicting aliases,
reserved names, symbol/name/source budgets, late DSL/evaluation/identity failures,
routing semantic versions and partial coverage, stale routing inputs, and raw rule
content that remains subject to final validation. Strict all-target Clippy passes
for Authoring, Import and Data (`runs/release-declarations-clippy.log`). All **223
Authoring tests pass** (166 library, seven CLI, 50 release), none ignored; the
build took 11m11s and the suites 18.17s, 0.31s and 17.65s
(`runs/release-declarations-authoring-tests.log`). Scoped formatting passes.
All **18 focused routing tests pass** (1m36s compilation, under 0.01s execution;
`runs/release-declarations-routing-tests.log`). Both new Import rule-content tests
pass (3m53s compilation, under 0.01s execution;
`runs/release-declarations-rule-content-tests.log`). This checkpoint passes **243
tests**, with none ignored in the selected suites. Peer integration review found
no material issue; diff checks and the nine protected-file hashes pass.
Current production source identity, coverage and the **0/5**
complete-original count remain unchanged.

**Authored evaluation construction checkpoint:** Metrics, Stages, Preparation,
Inputs, Receiving and optional Outputs now expose content without runtime seals.
Construction follows the existing dependency graph and delegates to the same
semantic validators. Inherited content can be extracted only by consuming the
actual checked evaluation owner. Stage/receiving semantic contracts, partial
coverage, optional support/outputs and explicit-null restrictions are preserved.
No runtime package reader repairs bindings or calls the source compiler.

The maintained release manifest uses these content payloads under the same six
component labels, with no compatibility fallback for sealed source JSON. It
constructs them after lowering against the final checked recipe and still runs
ordinary full release assembly. One borrowed semantic view shares nested work
and depth accounting with runtime inputs. The compiler reserves that work from
the source-release allowance before source constituent copies; byte, component
and final combined limits remain enforced. The old test helper that repaired
evaluation dependency hashes is removed. Runtime stale-binding refusal tests
remain required.

Strict all-target Clippy passes for Authoring, Data and Import
(`runs/evaluation-content-clippy-final.log`). All **225 Authoring tests pass**
(166 library, seven CLI and 52 release), none ignored. Compilation took 11m18s;
the three suites took 6.97s, 0.29s and 17.47s
(`runs/evaluation-content-authoring-tests.log`). All **269 Data owned tests pass**,
none ignored (1m37s compilation, 0.16s execution;
`runs/evaluation-content-data-tests.log`). All **1,127 selected Import tests pass**
(248 library and 879 owned integration), none ignored: 11m51s compilation,
39.22s and 38.02s execution (`runs/evaluation-content-import-tests.log`). This
checkpoint passes **1,621 tests**. Scoped formatting, diff checks and all nine
protected-file hashes pass. Peer review found no remaining issue in constructors,
shared accounting or compiler integration. The first lint attempt found one
test initializer style violation; it was fixed before the successful full lint
and test runs, with no suppressed warning or changed validation behavior.

The maintained release still has no evaluation bundle; this adds construction
capability, not Sniper content, complete coverage or a successful original.
The following checkpoint implements the publisher and allocating CLI approved on
October 23; next populate the real execution declarations and transfer useful fixture
laws before retiring the obsolete publication closure. No separate ledger-allocation
path is to be added. During publication-core extraction, reuse final
checked recipe handles for content construction and assembly where practical:
the current offline builder checks the final recipe for evaluation construction
and ordinary assembly checks it again. This is offline duplicate validation,
not a second validator or per-candidate runtime work; preserve validation and
budget ordering when removing it.

**Atomic authoring checkpoint:** the maintained manifest accepts one optional
`stat-declarations "definitions.kdl" names="declaration-names.kdl"` source pair.
The same decoder, typed declaration compiler, packet scopes and final release
checker consume it. Explicit bootstrap or predecessor selection replaces the old
optional caller-assembled context; even empty declarations preserve the full
predecessor registry and reserved names. Declaration errors include source and
packet paths. All CLI builds publish `checkpoint.json`, `names.json` and a closed
`runtime/` directory together. The receipt binds exact encoded sources, predecessor,
ledger and runtime; the checked loader verifies canonical bytes and the complete
runtime before granting allocation authority. Runtime registry identity must
equal the name ledger's registry identity. No history replay, compatibility layout
or separate ledger-allocation command is added.

The Host bundle primitive supports bounded root files and immediate child
directories of files, checks portable names and case-insensitive collisions
before staging, and shares the existing no-replace rename transaction. A failure
publishes neither ledger nor runtime. This is atomic visibility, not a power-loss
durability guarantee. Selecting an immutable predecessor is not mutable-head CAS;
that deferred work remains in the follow-up register.

Review caught and fixed build/load limit inconsistencies: one declaration-derived
allocation limit set now governs both paths; the loader checks authoring registry
bounds before copying; staged runtime bytes must fit the runtime loader's aggregate
input/output limits, including receipts. The outer checkpoint limits delegate to
the existing source-limit validator. All **243 tests pass**, none ignored: 166
Authoring library, eight CLI, 58 release and eleven Host tests
(`runs/authoring-checkpoint-verified-tests.log`). Strict all-target Clippy passes
(`runs/authoring-checkpoint-clippy.log`), as do scoped formatting and diff checks.
The first compile caught a temporary-path lifetime error in a new test; the next
run caught the real `--bootstrap` plus `--expected-predecessor` flag-conflict
omission. Both were corrected before the successful complete run, without
weakening the refusals. The stable combined Authoring/Host selection reused
dependencies: the release-test rebuild took 2m47s; the final binary-only fix compiled
in 6.25s, followed by 7.19s library, 0.37s CLI, 19.10s release and 0.02s Host execution.
Cargo timing reports remain under `target/cargo-timings`; these are local iteration
measurements, not CI speed claims or runtime evaluator benchmarks.

The production CLI built all tracked sources into
`runs/authoring-checkpoint-current-01`, checkpoint identity
`51f906f77bedc5fbe017122e368f015a638f8d3fc8f8baba9845cc6728248218`, then
resumed from that checked checkpoint into `runs/authoring-checkpoint-current-02`,
identity `46d39ada31d5a5d49998a2af69aa7d01d6d911caeb4ff88d4005a440ea8f20c5`.
The ledger and all eighteen runtime artifacts are byte-identical; the second
authoring identity differs because it records its predecessor. The runtime input
identity remains `d5f7086228a1fe2bfbc7fcdde033fd62741e719aa3af188fc5f3686b33b02406`.
Both local outputs are verification evidence, not required production inputs or
a new canonical distribution mechanism. Current build completeness stays **0/5**.

**Initial-type declaration checkpoint:** the maintained source now declares 69
Boolean Skill Stats and one definition-owned Sniper program, with 34 own-type
memberships, 34 minion-type memberships and explicit minion-context presence.
Sniper emits seven own positives, five minion positives and present=true; all
other projected memberships are explicitly false. Five excluded raw own types
remain in reviewed evidence. The finite preparation vocabulary is not the game's
complete type universe. There is no new Rust mechanic or dispatch by skill name.

The first non-empty persisted ledger belongs to this declaration source, not the earlier
empty-ledger publication. The full CLI build produced genesis identity
`e89ae92ac3895a3e9f38591452d340b8c640d4af737a544cc18759de4d98b109`, name identity
`dabf9a92dffb7e9ec36ee587e1dd377bc9aaffa096373c32062ac4428dadabd2`, and runtime
input `802bf2eccbdf83814dbb34f0745a2006c75ec4fdcd80e4ee8fc80e053852cdcb`.
Replaying through the ordinary checked predecessor loader produced checkpoint
`d15cabc5774b85149fa98d7672257e8b9188fa586c9fee8c598cdf732d8c4622`; the ledger
and all eighteen runtime files are byte-identical. Source text has LF bytes on
all 2,771 inventoried KDL/rule paths, including the manifest. Git attributes retain
those bytes across platforms. The following reference checkpoint pins source
revision `caca40a55885e5c8612e7d9b47c1691f8ea321a9`, avoiding a self-referential
source commitment. A fresh `git archive` of that revision reproduced the exact
genesis digest with the maintained CLI. Its normal checked-predecessor replay
preserved the ledger and all eighteen runtime artifacts byte for byte. See
`runs/initial-types-archived-{genesis,successor}.{stdout,stderr}.log` for local
evidence; reproduction requires no pre-existing local checkpoint. This reference
update changes documentation only, so the 245-test validation below still applies.

Validation covers real declaration types/ownership and Sniper facts, reused rule
scratch, six independently specified projection controls, and absent versus
present-empty minion contexts.
Absent context emits no invented minion-membership values. Historical gates admit
only the exact 69 schema additions and one program, preserve every original
registry entry, owner/program position, graph and metadata field, and reconstruct
the archived receipt with maintained content constructors. The production owner
remains Partial; the release has no execution bundle and remains **0/5** complete.

Final validation: **245 tests pass**, none ignored (166 Authoring library, eight
CLI, 60 release and 11 Host tests), including the unchanged five-original gate.
The last optimized rebuild took 2m36s; the release tests took 18.07 seconds.
Strict all-target Authoring/Host Clippy passes. Logs:
`runs/initial-skill-types-tests-verified.log` and
`runs/initial-skill-types-clippy.log`. Scoped formatting, diff checks and all nine
protected incoming-critical hashes are checked before publication. Early failures
were confined to the new fixture: type mismatches, omitted required empty
inventories and the invalid occurrence setup described below; no production
validator or coverage requirement was weakened.

An attempted isolated occurrence test exposed its invalid Direct selection of a
non-directly-selectable physical skill and missing readiness. The public effect
plan alone cannot accept current readiness stages; the preparation/support plan
owns that contract. Keep this slice at the public compiled-rule boundary instead
of inventing another support bundle or downgrading the operations contract. Add
exact occurrence, missing-producer and worker-reuse controls when the maintained
execution bundle is authored. Historical whole-plan fixtures retain those laws
until then.

**Composed support census checkpoint:** the structural traversal
already collected potential ActionOutput requirement owners, but support-domain
checks occurred only when an Action was requested or otherwise registered. Those
checks now run on the existing bounded structural traversal, with exact provider
paths and existing loadout eligibility. Disabled sources and descendants remain
covered; purely off-loadout providers remain excluded. Partial output inventories
retain ordinary coverage gaps as well as requirement gaps.
This repairs the accepted composed-build contract without a new model or another
discovery pass. Tests cover queried/unqueried outputs, known/missing/Unmapped
domains, complete-empty versus incomplete inventories, exact disabled descendants,
loadout switching and unchanged Partial numerical programs.

The maintained Authoring release target now runs the seven Gem support-domain
proof checks. It compares every one of the 966 current declarations against pinned
source evidence, preserves the 937 Known / 29 Unmapped split, and validates the
111 stat-set distinctions plus all eight missing-effect classifications and their
nine export occurrences. Source-proof tests read the pinned submodule; normal
release construction does not. The replaced ignored publication command and
helper are removed, including their local `runs/` and JIT report prerequisites.
The same maintained target now resolves each original's actual saved item/skill/
configuration/tree selectors through typed source links and performs checked
selected finalization. It must retain **123/137/127/144/5** input obligations;
selection may not mutate the draft, erase provenance or manufacture a ready build.
Original05's five selected obligations are skill-preset usage preferences,
configuration-role accounting, external assumptions, Scenario usage and Scenario
usage intents. Their exact diagnostic codes and owner paths are pinned in the
same test. Support-source coverage and remaining numerical/legality work are
additional gates, not represented by that count.

Validation: **1,579 distinct tests pass** across Authoring, Host and Engine; the
15 existing optional oracle cases remain ignored. The final affected-target run
passes 1,151 tests (166 Authoring library, 67 release, 197 Engine library, 710
native integration and 11 Host); the unchanged CLI/allocation/reference targets
passed another 428 in the preceding full run. Logs:
`runs/support-census-tests-final.log` and `runs/support-census-tests.log`.
The earlier full run exposed three fixtures that accidentally coupled source
domains to numerical program membership. They now declare synthetic output
source domains explicitly and retain missing/Partial numerical-owner assertions;
no production validator or requirement assertion was weakened. The final
optimized rebuild took 6m25s; the initial broader build took 13m37s, including
retained legacy targets. Do not repeat unaffected legacy suites for fixture-only
edits. Strict all-target Clippy passes for the selected packages, and a separate
Engine library check with legacy genuinely disabled passes
(`runs/support-census-clippy-final.log`, `runs/support-census-native-clippy.log`).
The combined package command still enables legacy transitively through Import;
it is not evidence that R1 dependency extraction is finished. Scoped formatting,
diff checks and all nine protected incoming-critical file hashes pass. Runtime
data, source-release identity, canonical genesis and **0/5** completion are
unchanged.

Found during peer review: the existing Core resolver returns DisabledProvider
before checking the source's own or ancestral loadout scope, including a disabled
support's target. This repair preserves existing eligibility; it fixes output
coverage without claiming that combined disabled/off-loadout case is correct.
Before loadout optimization, derive scope eligibility independently of enabled
state within the existing authored dependency traversal. Add the full
enabled/disabled × active/off-loadout matrix, disabled supports targeting authored
and generated skills, descendants and contained equipment, with missing/Unmapped
domains. Preserve authored required-input checks and structural requirement/usage
census even for inactive sources. Reordering only the two local conditions is
insufficient; do not add a second Engine traversal or a new game model.

The next blocker is the composed support-source census and execution input
authority, not more type facts. Current metadata covers 966 Gem, 19 Reward, five
UsagePolicy, three root/Ascendancy and eight intrinsic Skill/Actor/slot domains; all 23 selected
Original05 physical Gems are covered. Gem
coverage and an empty Sniper assignment order do not certify the composed build.
Audit the unchanged normalized draft's Class/Ascendancy/implicit roots, selected
equipment and exact modifiers, skills, passives, rewards, granted descendants,
Actor/slot owners, shared Actor rules, Encounter, UsagePolicy and ActionOutput
owners through the existing occurrence traversal. Author existing
`SupportDiscoveryInput` rows only where the entire admitted source domain proves
`AuthoredAssignmentsOnly`; numerical owners remain Partial. Preserve unknown and
positive extra/linked support factories as Unmapped. A selected positive factory
would require an authority/model discussion, not a false absence declaration.
No new public model is needed for the reviewed-absence tranche. Transfer the
disabled-source, duplicate-equipment, loadout and determinism laws, then retire
the replaced historical discovery publisher while retaining source evidence.

**Intrinsic support-domain checkpoint:** the release now covers eight existing
subjects:
Sniper Skill `0012`, its ActorSlot `001f` and ActionOutput `0015`; supplied Actor
`3091`; BasicAttack Skill `0021` / output `0022`; and GasArrow Skill `0024` /
output `0025` (all existing `def.000000000000....` identities in
`owned-mechanics-v1`, with exact declaration owners retained). Proofs cover full
constructors rather than the selected build's empty support list:
`act_int.lua:18677–18799`, `Minions.lua:83–121`, `minion.lua:116–264`,
`CalcSetup.lua:1898–1975,2053–2103` and
`CalcActiveSkill.lua:144–234,1116–1165` at the pinned source. Child construction
inherits the parent's existing support list; no additional-origin authority
follows from inheritance. Preserve nonempty inherited-list controls and explicit
positive ExtraSupport/LinkedSupport contrasts (`ModParser.lua:2220–2253,3650–3658`,
`ModCache.lua:9410`). Foreign modifiers/descendants remain independently checked,
and the missing Player Command definition stays unresolved. No new ID allocation
is needed for these existing owners.

These eight rows are authored in the existing
`migration-source/bootstrap/rules.json.gz` support-discovery component; the
current manifest already loads it. At that checkpoint totals were 994 domains (965 Known,
29 Unmapped), with the existing 986 rows untouched. Tests use the existing pinned
complete Lua archives only as offline proof: `self-flat-minion-store/{Minions,
act_int}.lua.gz`, `self-flat-action-factories/minion.lua.gz`,
`configuration-distance-inputs/CalcActiveSkill.lua.gz` and
`mana-supplier-sources/CalcSetup.lua.gz`. The proof belongs to the maintained
Authoring release target, without another parser or historical publisher.
Source review also follows `Data.lua`'s lazy `SkillStatMap` fallback: the shared
map archive and an explicit census of every stat used by these definitions,
including quality and all stat sets, cover the intrinsic mapping boundary.
Literal absence of a factory name in a skill table alone is insufficient.
Historical restoration authenticates and removes only those eight exact rows
before retaining every original metadata/artifact equality check. The changed
release uses the fixed checked genesis predecessor and pins the new input identity;
schema and name ledger remain byte-identical. The canonical genesis is unchanged.
The test helper bootstraps its source fixture for content checks, without calling
changed current content the fixed first genesis. This tranche retires no selected
input obligation and admits no complete build.

The pinned-source successor is
`3198e95e3c147e8fd8b723ad52a4635f77b300d9f8020f5040263b3f0296f9a4`.
Its source-bound checkpoint differs from the discovery build without
`expected_input`; both have the same runtime input. No evolving source is
published as a new genesis, and no compatibility path is added.

Validation: **256 distinct tests pass** (166 Authoring library, eight CLI,
71 release and 11 Host), with no ignored cases in these targets. The full run
passed 254 checks and exposed two new restoration assertions that incorrectly
assumed appended source domains remain a suffix after compilation. The existing
Engine canonically sorts this inventory by typed subject. Both gates now require
the exact eight-row count, no historical owner overlap, one occurrence per exact
typed owner and exact domain equality before removal; all remaining rows,
whole-package fields, historical receipts and artifact hashes must still match.
The subsequent 13 tracked-source checks pass, including both repaired gates.
No production behavior or validator changed during that repair. Logs:
`runs/sniper-support-domains-tests.log` and
`runs/sniper-support-domains-restoration-tests.log`; optimized rebuilds took
5m36s and 4m14s respectively. Strict all-target Clippy and scoped formatting pass
(`runs/sniper-support-domains-clippy-final.log`).

Four new source-proof tests authenticate eight existing archives and 26 reviewed
spans, including all 33 stat keys (12 mapped, 21 absent), exact owned bindings,
nonempty inheritance and external-origin boundaries. All four pass again after
normalizing the evidence JSON to repository LF endings
(`runs/sniper-support-domains-proof-final.log`). They preserve the missing
active Command and seven Partial numerical owners; the already complete-empty
summoner output remains unchanged. Independent review found and closed the lazy
map evidence omission before tests. Reloading the actual new checkpoint through
the CLI's checked predecessor loader reproduces the name ledger and all eighteen
runtime files byte for byte (`runs/sniper-support-domains-reload.log`). All nine
protected incoming-critical files retain their hashes. Selected input counts stay
**123/137/127/144/5** and complete-build admission stays **0/5**.

**Root and UsagePolicy source-domain checkpoint:** seven existing owners now
declare `AuthoredAssignmentsOnly`: Ascendancy `0a36`, class root `1790`,
ascendancy root `1b5d`, and UsagePolicies `3259`, `3267`, `326a`, `332b`.
All prior 994 rows remain unchanged; the current total is **1,001** domains
(972 Known, 29 Unmapped). Policy `338e` was already covered. No schema, name
ledger, program or numerical-coverage declaration changes in this tranche.

The maintained release harness authenticates full pinned source bytes and the
constructor/consumer evidence in `class-start-root`, `ascendancy-start-root`
and `ascendancy-owner-closure`. It preserves exact topology, class/root relations,
default modifier identity and external transformation ownership. Nine ordinary
laws and all 34 corruption controls move with the evidence; the two replaced
root targets and their three helpers are deleted, including their ignored
publishers and local-report dependencies. Dev source-authentication tests require
the pinned PoB submodule; the normal release builder still does not.
Archived seventeen-owner preservation checks retain their historical state;
current checks do not regress independently completed Passive `1b48` to Partial.

The four policies are checked as complete parameter-to-derive programs, using
a conservative test-only structural predicate independent of names. Tests retain
required-input failures, Boolean false, Count zero, participation truth tables,
exact parameter ownership and absent scenario expansion authority. Changed
effect/read kinds, unknown coverage, foreign parameters and additional target
authority fail the review. This is evidence for these exact declarations, not a
runtime inference or general permission to certify empty program lists.

Historical restoration removes exactly fifteen independently typed declarations
after checking their count, unique owners and complete row equality. Every other
metadata field, historical receipt and artifact hash must still match. Publication
uses the fixed checked genesis predecessor; the pinned-source successor is
`dc5d6c1997f188d169e2ec62b83d14e0f0ed8ce13b111c3cb874e1c17c4387eb`.
All **269 affected tests pass** (166 Authoring library, eight CLI, 84 release
and eleven Host), with no ignored cases. The optimized build took 4m11s;
the release target took 18.25s. Strict all-target Clippy, scoped formatting and
diff checks pass. Logs: `runs/root-usage-domains-tests.log` and
`runs/root-usage-domains-clippy.log`. These are the affected Authoring/Host
profiles, not a claim of runtime native-only dependency isolation.
Loading that actual checkpoint as predecessor and rebuilding reproduces all
eighteen runtime files and the name ledger byte for byte
(`runs/root-usage-domains-reload.log`). The replay checkpoint is
`11c3e8ac7f1bf345c645b591046912c2f0236aef86a1452d1b56f881eb446542`.
The nine protected incoming-critical files retain their hashes. All five selected
input counts remain **123/137/127/144/5**, with **0/5** complete builds and no
evaluation bundle. No input or numerical blocker is falsely retired by these
source-domain proofs.

Next prioritize the unchanged Sniper active spec, not the largest global family.
Its 57 mapped passives comprise 55 explicit nodes and two roots. Both roots are
covered; 43 Complete numerical owners and 12 Partial owners still lack intrinsic
source domains. Reuse the proof families that actually intersect those 43 before
unrelated breadth. Jewel Socket source `7960` / Passive `1b48` is now independently Complete;
the old thirteen-Partial selected census is historical.

The immediate family is **attribute choices: 22 selected owners, 293 catalog
members**. Prove all three attribute options and their constructor dispatch,
complete exact choice ownership, the current Count-cutover six channels and
missing-choice refusal. The saved Strength/Intelligence selections alone do not
prove the whole definition's source domain. Use the authenticated source policy
and current typed bodies; historical Complete coverage alone is insufficient.
Start from `attributes/policy.json`, `tree/tree-catalog.json` and
`attribute-stages/{count-bindings,count-programs,count-authoring}.json` under the
pinned data root. Retain the Import `owned_attribute_recipe` laws and the
`owned_attribute_count_cutover`, `owned_attribute_count_native`,
`owned_attribute_base_source_audit` and `owned_attribute_base_membership` helpers'
useful controls. The PoB `owned_attribute_pipeline` witness and full archives in
`passive-structural-inventories/sources` / `empty-passive-socket-intrinsics`
supply existing raw/constructor evidence; no new parser or runtime dispatch is
needed merely to declare a reviewed domain.
The remaining selected Complete families are: pure Minion Damage (2), combined
Damage/Life (4), Command Cooldown (6, preserving four paired Damage programs),
pure Minion Life (2), Command Damage (3), Growing Swarm (1), Gigantic (1),
class-dependent passive view (1), and empty Jewel Socket (1). These counts plus
the 22 choices partition all 43 selected Complete owners without duplication.
Their proof packets are `plain-minion-owner-closure`,
`plain-minion-life-passives`, `command-cooldown`, `minion-life-passives`,
`command-damage`, `growing-swarm`, `gigantic-following` / `gigantic-flags`,
`passive-views` and `empty-passive-socket-intrinsics`, respectively. Reuse the
matching Rust evidence helpers, then delete replaced publication paths only
after preserving their useful laws and independent current-body checks.

The 251 defence/recovery passive definitions are a coherent breadth/retirement
batch but intersect **zero** active-spec nodes (12 in saved spec 5 and 17 in spec
6). The 58 plain attribute definitions also intersect zero active nodes. Defer
both batches until the selected families. Reuse full-catalog policies, complete
raw-field inventories and shared constructor evidence; compare current
descriptors and numeric programs, preserving the later Count cutover rather than
restoring historical effect counts. Retire replaced publisher helpers when their
useful laws have moved to the maintained harness.
Reward-domain proof/replay transfer is another R4 candidate; retain its generic
complete-numeric admission and missing/restored/Unmapped receiving controls.

Class definitions `0a1e`–`0a25`, shared Player Actor `332a` and Encounter `31d1`
need separate full-constructor proofs; their current Partial numerical programs
cannot prove source absence. Class evidence must include base attributes and
unarmed defaults, and shared Actor evidence must distinguish intrinsic bases
from foreign modifier merges. Encounter certification must account for custom
modifier text: `ConfigTab.lua:1201–1241` and `CalcSetup.lua:908` can introduce
parsed ExtraSupport/LinkedSupport sources. Do not certify all configuration from
a default selector or an empty selected list. Keep foreign custom sources
independently blocking until accounted for. The unchanged Original05 uses item
set 2, skill set 4
and passive spec 3; the audit found no selected positive extra/linked support
factory, which is only a finite witness. Two selected passive grants remain
unparsed: Kelari's Deception (`ModCache.lua:7962`) and Navira's Fracturing
(`ModCache.lua:8105`). Their missing mechanics/authority remain tracked blockers;
do not infer support absence from the parser returning no modifiers.

Inherited summoner context already uses `ReceivingSkill { summoner_path }` and
the exact parent's prepared types; do not add another structural read for it.
Assigned targets still need explicit summoner presence. Keep shared support
prohibition separate from saved occurrence `noSupports`: review the exporter’s
Boolean source field, winning definition and later factories before authoring
false. Then construct the actual program/stage/readiness/receiving/metric
inventory. Do not copy the old complete-empty fixture census. R1 publication-core
extraction, R3 compact packaging and R4 legacy removal remain gates before the
first unchanged build is complete.

Historical CI performance observation, 05:07 UTC on October 23: four CLI
two-feature-profile test steps and Windows PoB tests had run for about 163 minutes.
Those jobs subsequently passed as part of the completed twenty-job run. Each CLI step compiles and
tests both profiles serially; public step metadata does not identify its current
subphase or establish ongoing log progress. Windows CLI-3 completed at 05:04:43 UTC,
so the snapshot does not indicate a workflow-wide hang. Keep the existing R0 work
on shared fixtures, test-binary reduction, CLI/PoB consolidation and partitioning;
no workflow or concurrency change is justified solely by that snapshot.

The [support target facts investigation](owned-support-target-facts-proposal.md)
also records a deferred recommendation for narrow structural source reads, which
needs approval only when an actual mechanic requires it. The earlier review prompt
predates the unused-input finding and no longer blocks current work. Manual Djinn
`gemData` does not prove physical Gem ownership or a native eligibility rule.
Do not promote fixture flags or `unwrap_or(false)` into production bindings.

Reuse the reviewed fragments as evidence, not as a ready-made evaluation bundle:
`djinn-support-preparation/preparation.json` supplies typed preparation definitions;
`sniper-final-inputs/{readiness,source-properties}.json` and
`offering-final-inputs/{readiness,source-properties,preparation}.json` describe
specific census/assembly relationships; `ordinary-item-routing/readiness.json`,
`amulet-bonus-snapshot/readiness.json` and `skill-participation/readiness.json`
add program roles; support-delivery `receiving.json` files identify exact receivers.
Inventory their owner-qualified identities against the current package before
authoring the missing declarations. They are historical/reference fragments with
partial inventories and old commitments. Fixture-only closed domains or synthetic
predicate Stats do not establish production completeness. If actual source
inventories cannot be completed and execution instead requires admitting those
same partial source domains, bring that contract change back for review.

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
| Disabled sources on unselected loadouts | Existing resolver precedence can wrongly require their support domains. Fix scope eligibility across authored dependencies before loadout optimization (or earlier if it blocks R5); retain disabled selected-source coverage, required-input checks and structural requirements. Cover the enabled/scope matrix and support-target/container ancestry without a second Engine traversal |
| Windows test runtime-library linkage | The support-census validation emitted LNK4098 from the Lua-linked Engine `legacy` and `owned` test binaries. Resolve the linkage for oracle binaries retained through R4; do not suppress all linker diagnostics. No warning was emitted by strict Clippy, and native production gained no Lua dependency |
| Resource obligations | Accepted Skill-owned payer/resource model remains required; amount-channel signature proposal still needs a concrete decision if R5 requires it |
| Separate socket configurations | Accepted but incomplete; finish when needed for the first build or R6 equipment search; keep rolls/configurations/physical copies separate |
| Scoped numerical coverage / computed source selection | Pending, not implied by DSL approval; unknown reach stays blocking. Ask only when a concrete required case needs a public contract change |
| Lua cleanup and resistance terminology | R4 audit classifies game law vs external syntax vs obsolete application code; ordinary resistance is a game channel, legacy is not |
| Numeric P3 suggestions | After semantics-preserving migration, add targeted cases for exact integer folds, failure order/All/Any, equality, rounding and decimal scaling; discuss output-changing policy before adoption |
| Curve/segment storage | [S1-S4 investigation](owned-scaling-data-investigation.md) in R6 unless an earlier real mechanic requires it; lossless data/domain/rounding equivalence, no fitted extrapolation |
| Seeded jewels / legion-rune opportunity | [J1-J5](seeded-jewel-feasibility.md) in R6: establish game/pin applicability, provider parity, bounded seed/socket/path search, reproducible reporting and independent cost/quality checks; opt-in |
| UI autocomplete | Bounded query/index over loaded named definitions; no database/ORM milestone |
| PoB updates | One pin manifest, offline extraction and selective differential tests, stable semantic anchors and a pin-bump runbook; no automatic network/source work in cargo build |
| Repository/release hygiene | R0-R4 dependency boundaries, deduplicated tests/artifacts/docs, one MSRV policy and locked workspace dependencies; batch Windows formatting by package while workspace-wide `cargo fmt --all` exceeds the process command-line limit (OS error 206); R6 release packaging, dependency audit and measured release-profile tuning |
| Repeated offline recipe validation | During R1 publication-core extraction, reuse the final checked recipe between authored evaluation construction and full assembly while retaining all shared aggregate and dependency checks; measure before broader performance work. No native candidate path is affected |
| Mutable authoring-head promotion | Deferred until a shared mutable head is introduced; the first approved checkpoint publisher uses an explicit immutable predecessor. Implement actual stored-predecessor CAS/concurrency control and interruption tests before exposing head promotion; read-compare-rename alone is insufficient |
| Canonical allocation-history distribution | R2 first-generation bridge implemented: canonical reference pins source `caca40a5` + checkpoint digest outside the compiled inventory; fresh source-archive reproduction and normal checked loading pass. Before a second allocation generation needs a new predecessor, R3 must archive/distribute one full checked snapshot. No historical source-replay chain, silent rebootstrap, machine-local dependency or second ledger importer |

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

At every checkpoint record the current MVP blocker, the smallest change that
removes it, and any safely deferred work with its later gate. Reassess the closest
build from actual input, execution and legality evidence rather than counting
implemented features. Prefer removing a replaced path and shortening iteration
over adding optional infrastructure; do not weaken model correctness or coverage.
Use targeted PoE2/PoE1 wiki or other public mechanic references when they can
resolve a specific ambiguity or reveal a simpler general model. Record the game
and version, distinguish explanation from observed behavior, and validate the
chosen semantics with independent cases and the applicable reference. This is
targeted research, not PoEDB scraping or bulk extraction; it does not authorize
copying source-language accidents into the native model.

1. Read this plan and Git status. The computed-support unused-input repair,
   immutable typed Stat declaration compiler and original bootstrap dependency
   checks, aggregate evaluation budgets and complete Import content construction
   are implemented in the maintained builder. Immutable Stat declarations now
   participate in that in-memory compiler, and authored evaluation construction
   now follows the same final dependency graph. Atomic checkpoint publication and
   the allocating CLI are implemented. Next **author the current release's
   real execution declarations**, following the R2 prerequisites above. Keep these
   as separately validated compiler phases; source extraction has no authority
   over evaluation inputs.
   Declaration integration takes an explicit persisted name ledger, validates it
   as a successor of the bootstrap registry, and stages before projecting newly
   allocated names against the schema. Never reconstruct an empty ledger on replay.
   Preserve packet-local aliases; overlay only current declarations with exact
   typed-address agreement, separate definition/slot namespaces and bounded unique
   growth. Construct rule-shell/routing bindings from their authenticated source
   inputs before packet application. Return successor names and the predecessor
   identity only after final assembly and `expected_input` succeed. Cover replay,
   earlier-sorting additions, predecessor mismatch, alias agreement/conflict,
   symbol limits and late DSL/evaluation failures without predecessor mutation.
   The CLI declaration path now preserves source/packet paths alongside typed
   conflict and limit causes.
   Evaluation construction now follows the existing graph in
   `owned_release_evaluation.rs`: checked recipe to Metrics, Stages and Preparation;
   Stages/Preparation to Inputs; those packages to Receiving; all prerequisites to
   optional Outputs. It reuses row DTOs, Readiness and SourceProperties, and one
   shared bounded traversal over semantic payloads before hashing/copying.
   Extract inherited content only from independently checked evaluation owners;
   the source-prefix checker cannot authenticate that graph. Author historical
   fragment semantics explicitly or first validate their entire original graph.
   Preserve optional support/outputs, coverage and predicate/path limits. Existing
   gates cover unchanged identity, changed dependencies, stale originals, missing
   program classification, malformed closures and exact/one-short budgets.
   Construct against the final checked recipe and retain ordinary full release
   assembly. Reuse checked recipe handles when extracting the publication core
   to avoid the current offline duplicate recipe validation; do not bypass the
   shared pre-copy budget or full graph checks.
   Preserve the checkpoint laws: exact immutable predecessor, reserved-name
   retention even with zero declarations, runtime/ledger cross-binding, bounded
   closed inventories and atomic no-clobber publication. Mutable-head promotion
   remains deferred; do not add another publication layout or allocation command.
   Sniper's real initial-type facts are now authored from pinned source evidence.
   Canonical first-genesis reproduction is verified and its reference published.
   Next complete the composed Original05 support-source audit above, followed by
   execution inputs: explicit shared
   support-prohibition authority and assigned-target summoner presence, then
   program classification, freezing, readiness, receiving and metrics. Inherited
   summoner context already uses the exact-parent receiving relation. A missing
   source value is not an inferred false.
   In parallel, continue independent behavioral transfers.
   Complete the actual execution declarations needed to replace historical
   whole-plan fixtures, **retire obsolete
   transformation paths and move the maintained publication core into Authoring**.
   Do not blindly relocate
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

   Existing Authoring gates preserve exact graphs/owner placement, non-program
   constituents, historical receipt restoration, publication atomicity and allocation
   stability. Direct arithmetic, DSL coefficient change, five-original query/action
   provenance now share its cached tracked release.
   Continue in this same `release` target; do not create another builder harness.

   Finish the remaining transfers in three coherent groups:
   - Numeric components: scalar families, Actor attributes, local scaling and
     elemental-channel laws now use the cached release harness; their replaced
     numeric helpers/calls are deleted, including the four separate defence-family
     numeric cases. Transfer remaining outer canonical property-slot, import and
     wire-budget checks before deleting their publication helpers. Preserve
     units, exact channels, lazy branches, negative rounding, overflow, missing
     facts and independent/reused scratch. Discard old program/owner/target-growth
     counts.
   - Item attribution: grammar/source cases in `owned_canonical_admission`,
     `owned_item_layouts`, `owned_item_metadata`, `owned_source_conditions`,
     `owned_source_role_migration` and `owned_item_quality_inputs`. Use current
     checked item/source policies; preserve signed/ranged parsing, overlays,
     applicability, provenance and malformed-input rejection. Current support
     determines whether Corrupted headers and other inputs are admitted; retire
     historical dialect/version tests and intermediate admission-gain totals.
     Metadata preamble transfer is complete using cached-release items/source
     policies and the existing attribution/conversion APIs. Its independent value
     12 uses the admitted untagged range; the tagged line remains a refusal case.
     The old preamble helper and v4 compatibility reconstruction are deleted.
     Remaining: original-build metadata and outer policy/publication assertions.
     Keep `metadata_only` until its `check_originals` caller also moves.
   - Gem ownership: source-value checks in `owned_support_gem_inputs`,
     `owned_gem_numeric_aliases`, `owned_multieffect_gem_inputs` and
     `owned_active_gem_inputs`. Reuse current normalized originals and independently
     join raw corruption/quality/level, nil/missing handling, physical/generated
     identity, multi-effect membership and unresolved fields to exact origin links.
     Drop predecessor/rebind comparisons and intermediate closure expectations.

   The reachability audit found no uncalled nested `check_*` helper: Actor/Cleric
   chains continue through selected Actions. Do not delete them merely because
   they are not top-level tests. Current-release whole-plan tests additionally need
   authored evaluation/readiness declarations (see the R2 blocker above); retain
   Cleric's existing whole-plan/parallel regression until that replacement passes.
   Also relocate `owned_item_observations::check_export`
   (the `pob` feature) to maintained offline export tests: it checks extractor
   output, authenticated catalog/evidence and no-clobber behavior. Default-profile
   transfer alone would lose this evidence. Then delete predecessor/stage/republish
   helpers, obsolete transition fingerprints and the closed historical pipeline.

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
   remain an R1/R4 deletion gate. Mutable-head filesystem CAS, broader schema/
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
