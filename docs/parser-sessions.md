# Parser sessions and source-visible cache state

Design direction: retain the original public parser wrapper and private mutable state in
shared source programs. This follows the accepted typed-language and full-parity direction.
The service API and complete public parser are not implemented; current gates and the resume
point live in [implementation](implementation.md).

## Why the parser is a session

PoB's public `modLib.parseMod` owns a cache, scalar capture cells and mutable dictionaries.
The cache is also exposed as `modLib.parseModCache`; startup can preload it. Exact input
text selects a cache entry, including case. No-match results can be cached. A DOUBLED modifier
mutates shared name data and changes later uncached parses, while an older cached result
retains its earlier meaning. These effects are already demonstrated by the
[session audit](../crates/poe-optimizer-pob/tests/mod_parser_session_audit.rs) and
[public result audit](../crates/poe-optimizer-pob/tests/mod_parser_public_parity.rs).

The current `CompiledModifierParser::parse` is a bounded stateless interface. It retries,
copies results and owns new output/program accounting for each call. It remains useful for
its admitted legacy consumers, but it cannot replace the actual public source callable in
real-build preparation merely because it parses the next quest line successfully.

The end-state boundary preserves the complete public wrapper, cache/dictionary history,
result cardinality, identities, source failures and cumulative resource use. Unknown parsing
stays distinguishable from a legitimate no-match result.

## Source programs, kernels and ownership

Shared definitions own authenticated code, source data and immutable initial descriptors.
A preparation session owns cache tables, mutable dictionaries, scalar cells and function
instances. Every alias to one mutable source table resolves to that same private identity,
including aliases exposed outside the wrapper. This state belongs to the preparation's
explicit lifecycle and baseline; it must not leak between unrelated search candidates
through a shared Rayon worker. Sessions may share compiled libraries and immutable data.

The original wrapper remains responsible for cache lookup/write, retry, copying and return
packs. A native inner parsing kernel is viable only behind an exact source-bound contract
that consumes the same live state and accounting. The existing public `parse` interface
would repeat retry/copy work and hide state, so it is not that inner kernel. Keep reusable
pattern matching, scans and numeric operations beneath the source-level state machine.

Any service binding retains both its source owner and parser owner, the authenticated
callable/prototype and the applicable data/program admission manifest. Names, matching
source spans or equal numeric IDs cannot establish identity. Binding happens when constructing
a validated compiled owner/session, never through a mutable global callback-name registry.
Normal callee lookup and argument-effect ordering remain part of the shared call protocol.

Prefer observing interdependent programs and state into one coherent owner. If a kernel
retains another owner, use explicit authenticated identity mapping or owner-tagged handles.
Never reinterpret parser-local callback/table IDs as receiving-session IDs. A returned
function that captures mutable parser data must retain the same session cells. Unsupported
function-bearing results must remain visible until that identity path is implemented.

## Return graphs and failures

The wrapper returns `unpack(copyTable(cache[line]))`, not an unconditional two-value tuple.
Nil/Nil, a single result table, a Nil first result with extra text, and an empty extra string
have different raw packs or truthiness. Cache lookup uses exact source keys; `isComb` does
not change the cache key. A truthy cached no-match result avoids the inner parser.

Original `copyTable` recursively copies value tables without memoizing repeated references.
It can split aliases between repeated nested occurrences while preserving function identity.
Preserve this source-required copy first, then preserve the resulting graph through writable
import, `modLib.setSource` and ordered `AddMod`. Do not merge copied aliases or make returned
tables read-only. Repeated public calls must yield independent writable results; modifying
one result must not change the cache or future results. Existing
[program/public parity tests](../crates/poe-optimizer-pob/tests/parser_program_public_parity.rs)
cover these distinctions.

Source errors preserve earlier writes. Failure before cache assignment differs from failure
after assignment or a dictionary mutation. Missing producers, unsupported language/data,
ambiguous scans, budget exhaustion, source errors and valid no-match outcomes remain distinct.
A service must never turn unsupported behavior into an empty modifier list.

## One cumulative preparation budget

Borrow one preparation-owned accounting context across source calls, parser scans, typed
callbacks, cache allocations, copies and result import. It covers instruction/depth work,
pattern work and value/table/byte allocation, including failed calls. Enforce limits before
allocation/publication; adding deltas only after success loses failures and permits transient
overshoot. Private parser and session counters must not independently grant the whole limit.

Factor the existing request accounting and allocation facilities so legacy `parse` can own
a short-lived request while source-session execution borrows its caller's context. Audit
intermediate strings, tables and copies as well as final outputs. Importing a produced graph
costs additional allocation unless ownership can transfer without copying. Cache hits charge
lookup and copy work; misses and retries charge the inner operations they actually execute.

## Alternatives and consequences

A handwritten native public adapter could reduce interpreter overhead, but matching the
source requires the same cache/dictionary state, exposed aliases, startup history, error
prefixes and return rules. It would duplicate a public lifecycle already expressed in the
chosen rule language. Retaining the source wrapper makes update differences and missing
operations explicit. Pure inner kernels remain available for measured optimization.

This choice requires more general mutable table, closure and accounting facilities before
quest activation can be admitted. That work also serves equipment, provider and later
calculation preparation, avoiding an isolated quest-only parser bridge. It does not imply
source interpretation belongs in the per-candidate calculation hot path; prepared native
plans retain their separate performance contract.

## Table evidence and the constructor boundary

Observed mutable tables carry private raw traversal/length facts through the shared session
input, separately from unordered entries. Replacing an existing non-nil value with another non-nil value can preserve that
layout; structural writes invalidate it until a native transition model proves the new state.
Original source `copyTable` must read live values in that observed order and construct fresh
writable tables through normal source operations. The input's length is not the copy's length.

This distinction matters for `{nil, extra}`: a two-element source constructor can reserve
array slots absent from its raw inventory, while copying starts from an empty table and
inserts the present entries. Track native constructor allocation/layout and insertion history
generically before admitting default `unpack` for sparse results. An explicit numeric range
can proceed without default-length evidence, but changing the original wrapper to supply one
would change its semantics. Do not special-case cache rows or substitute maximum integer key.

Preserve the pinned runtime's numeric-conversion profile as an explicit requirement when
extending `unpack` beyond currently admitted signed-32-bit indices. Host-dependent casts must
not silently change native/WebAssembly behavior. Source-authenticated observations, raw data
transport, native layout simulation and source-domain admission remain separate concerns.

## Integration gates

1. Establish mutable traversal/length behavior for actual wrapper and `copyTable` consumers,
   including holes, writes, deletion and observed order where it affects output or failure
   prefixes. Immutable traversal evidence cannot be attached to a writable table unchanged.
2. Execute source `unpack` and complete recursive copying with exact result packs, fresh
   identity, function preservation and cumulative budgets.
3. Bind actual wrapper capture cells, exposed cache aliases, mutable dictionaries and startup
   preloads/history. Pair cache hits/misses, cached no-match results, retry, eviction, DOUBLED mutation
   and failures with the existing source audits. Model optional logging from its observed
   environment; do not infer absence from commented source declarations.
4. Bind the complete inner parser through source programs or a validated state-aware kernel.
   Preserve scan ties/order, produced closures and every reached branch. Executing only the
   short wrapper with an observed inner-call producer remains a component gate.
5. Continue original multiline quest/custom callbacks with writable source-tagged results,
   then full initial/saved activation on Twister and Skeletal Sniper together. Retain all five
   originals and mapping variants. A cached success or a larger callback prefix alone does
   not admit complete native build evaluation.

The pinned behavior is in `ModParser.lua`'s public wrapper and DOUBLED branch, `Common.lua`'s
`copyTable`, `Main.lua`'s cache preload, and `ConfigOptions.lua`'s quest consumer. Exact source
references and dependency findings are recorded in `runs/r2l-parser-service-readiness.md`;
future admissions must bind the actual source/data revision rather than these prose names.
