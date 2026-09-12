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

## Numerical helpers and injected flag logic

Keep language-level numeric primitives separate from game-level flag helpers. The native
engine can implement the admitted source VM's modulo and scalar bit operations, while
`OR64`, `AND64`, `XOR64`, `NOT64`, their capture cells and mask values remain in the
source/data catalog. Combining signed low words and high masks must follow the complete
source helper, including its arity, coercion and early-return behavior; a conventional
unsigned integer operation is not automatically equivalent.

Numerical admission includes operation order and conversion rules. The pinned modulo
path uses separate division, floor, multiplication and subtraction, with observable
negative, signed-zero and exceptional-value behavior. Bit conversion uses the VM's
rounding rule, independently from integer-index conversion. Evaluate ordinary argument
expressions before primitive conversion, preserve ignored-argument effects, and charge
all work and results through the session budget. Source operand timing and constant-fold
admission remain distinct from runtime arithmetic.

Parity inputs are values admitted to the selected source numeric profile. A transport
adapter must reproduce any source API NaN canonicalization when comparing foreign raw
numbers; the neutral graph does not silently rewrite supplied bits. Tests compare finite
bit-operation outputs exactly and distinguish this from the existing class-only treatment
of floating NaN payload/sign. Keep complete original-helper comparisons on real initialized
build graphs alongside scalar cold/warmed tests and portable-library checks.

## Observing complete initialized state

The reference adapter must capture the complete admitted initialization graph, including
large caches, without one host-language Lua handle per retained value. Use a private
runtime-owned arena or equivalent bounded retention mechanism; retrieve only transient
handles during discovery and conversion. Preserve original table/function identities,
shared capture cells, traversal observations and source numeric/string values. The arena
must not mutate the source tables or global environment, and both successful and failed
capture must release its roots. Existing value, text, table and callback limits remain
explicit; an incidental bridge-library reference-stack limit must not define build breadth.

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

## Retained builtin iterator identities

Source code may capture `ipairs`, store the function/state/control tuple it returns,
pass its auxiliary to another function, or call that auxiliary directly. The native
contract preserves those functions as owner-bound identities. A factory call returns
the same retained auxiliary, the original table and control zero; it does not allocate
a cursor or infer a builtin from the current global name. Rebinding a global changes
later global lookups without changing an earlier captured function.

The reference adapter captures the original factory before source loading and verifies
its private C capture to establish the auxiliary relation. That auxiliary is not a Lua
global or a fabricated Lua upvalue. The shared catalog carries the authenticated relation;
the native engine uses it without a Lua runtime. Old catalogs without the optional relation
retain their existing admission limits. Structural deserialization alone cannot authenticate
claims from an untrusted catalog.

The auxiliary consumes its explicit control on each call, increments within the admitted
source integer profile and performs a raw lookup against the current table. Nil ends the
protocol with zero return values; false is a real value. Metatable fallback, physical
traversal order and a saved snapshot are irrelevant to this raw lookup. Stored or interleaved
iterators therefore observe intervening writes. Generic loops and explicit calls share the
same function/state/control protocol and cumulative resource accounting. Numeric inputs
outside the supported portable conversion domain remain explicit dependencies.

Acceptance compares complete result packs, identity, rebinding, mutations, independent
sessions and argument effects before failure. Warmed evidence must identify the exact
original wrapper in a completed live trace. The pinned LuaJIT cannot trace every legal
interpreter path: a dynamic negative hash lookup on a table with both array and hash storage
can stop recording with `NYITMIX`. Such cases retain interpreter parity and an explicit
trace limitation; a separate pure-hash case can establish the negative-control trace path.
Neither case establishes general warmed-table layout semantics.

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

## Source constructor allocation

Constructor allocation belongs to the immutable program catalog, separate from the
input graph and mutable table observations. Each descriptor identifies its callback,
complete program provenance, exact expression range, original instruction location and
bytecode digest. The optional constructor metadata includes a required typed runtime
profile recording the source VM revision, numeric mode, architecture/endianness, GC/frame
convention and relevant build flags.
These describe the source being emulated; they do not require Rust to run on that same
architecture. A WebAssembly evaluator can execute an admitted source profile.

The PoB adapter observes the actual function identities before lowering. Its opaque
observation retains the original owner binding; matching names, source lines or newly
compiled snippets cannot replace it. Source lowering reconciles observations with whole
programs and constructs a fresh catalog. Ordinary lowering has no constructor evidence.
Serialized descriptors remain structural claims whose importing domain must authenticate;
valid JSON does not prove a source runtime observation. Compile-only flags such as
LuaJIT's table-allocation bump option require explicit trusted build attestation.

The native session owns simulated allocation state only for constructors whose semantics
are represented. All writes, including nil-to-absent stores and table.insert, update or
invalidate that state through one mutation boundary and the same cumulative budgets.
An explicit-key write outside the modeled layout remains a legal value update, but later
operations requiring unknown layout must report unsupported. Append must first establish
its index from a supported length; an ambiguous length stops it before any write.
Raw snapshots preserve values and aliases without
reconstructing allocation history. Imported observations cannot become simulated layouts
merely because their raw entries equal a freshly constructed table's entries.

The end state must represent new-table allocation, constant-template duplication, reserved
nil slots, numeric growth, mixed-key hash layout and deleted-slot history. Sparse raw
length, the length operator, and warmed JIT length hints require separate source evidence;
a match against the interpreter's raw-length helper alone does not establish all three.
Keep exact-function interpreter and warmed result/failure comparisons in the acceptance
suite, including equal raw maps reached through different construction and mutation paths.
The current implementation/admission limits are tracked in [implementation](implementation.md).

## Template slots and fresh map traversal

A constant template can reserve a string-key slot even when the initial value is nil.
A source-bound template descriptor must distinguish that slot from an absent key and
retain its observed order separately from the raw value map. When the source duplication
algorithm preserves the slots, native allocation can preserve their order through writes,
deletion and reinsertion into those same slots. New keys and layout-changing numeric
packs require separate transition evidence. This proof belongs to constructor metadata;
it must not weaken the narrower contract for imported snapshots of live keys.

A copied table has its own allocation and insertion history. Matching its input's raw
entries does not establish matching traversal order. Likewise, sorted Rust keys or source
literal order cannot replace observed source order. General hash transitions must account
for the source's string and object identities: the pinned runtime uses string IDs affected
by interning history to place string keys. The eventual injected representation needs
sufficient identity and layout facts, or a separately validated proof that order cannot
affect the entire operation, its failure prefixes or downstream consumers.

The bounded template-slot phase and its remaining acceptance gates live in
[implementation](implementation.md). It must first identify the actual constructor and
fresh table reached by the original parser; a matching fixture alone is insufficient.

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

## Source-created closures

Observed closure instances establish initial state. Native execution must also create
closures at the original lexical function occurrence. A factory site binds its parent
callback and exact expression range to an immutable child prototype, with ordered capture
origins: a visible parent-local cell or an existing parent-upvalue cell. Source metadata
must establish the child beneath that actual parent; neither matching capture names nor
executing a replacement factory authenticates the relationship. Construction creates a
fresh closure identity on each execution while reusing the compiled child program.

Captured locals require shared cells with lexical activation lifetimes. Reads and writes
through the enclosing frame and every capturing closure observe the same cell. Leaving a
scope, returning, or failing closes the frame's attachment without losing an escaped
closure's value or identity. Re-entered declarations and loop iterations must follow the
source's cell-opening/closing boundaries, rather than accidentally reusing a previous
iteration's cell. Recursive local functions allocate the cell before binding the new
closure into it. Nested captures retain their original cell through intermediate functions.

Factory metadata and compiled prototypes remain immutable, injectable data. Worker
sessions own created closures, cells and mutable tables, and charge their creation against
the same cumulative preparation budget. Raw snapshots must preserve aliases without
turning a produced closure into a static callback or granting another owner's prototype.
An unsupported factory or source layout is an explicit dependency, not an empty result.

Acceptance includes sibling closures sharing a mutable local, equal-code closures with
distinct identities, recursive capture, nested capture forwarding, closures escaping loop
iterations and error exits, and no cell sharing across independent sessions. Original
parser cases must exercise `getEffectFromStatus` and returned jewel functions against
PoB, including subsequent dictionary mutations; simple factory fixtures supplement these
real consumers. Mixed assignments must preserve source-local operand timing after locals
are promoted to cells, so adding factories cannot change earlier assignment semantics.

Creation-site evidence must include capture origins, not just ordered names. The PoB
adapter can inspect real child prototypes without executing their factory. Its reflection
and bounded bytecode-dump metadata must agree on each parent/child creation edge, local
register or inherited upvalue, and lexical declaration lifetime. Handle the source VM's
warmed-instruction normalization and recursive-local debug-range convention explicitly;
missing or ambiguous metadata remains unsupported. Dump limits also need to cover the
VM's temporary prototype buffer, rather than only the host writer's output.

Adding mutable active-frame captures also requires an operand-timing audit beyond indexed
reads and assignments. Arithmetic, comparisons, concatenation, calls and computed
constructor fields must preserve when the original compiler retains a local register or
copies its value. Do not admit factory-created closures on the assumption that earlier
expression evaluation was always eager. Differential factory tests must cover these
interactions before whole-parser admission.

The typed creation facet must match every admitted creation expression in both directions.
The same child prototype cannot change its ordered capture layout between sites, and
simultaneously visible lexical declarations cannot claim the same source register. Keep
structural package validation separate from authentication against the actual source VM.
If a child body cannot be represented completely, report its dependency for each enclosing
factory; do not remove its branch or publish a parent that references missing code.

Native frames may represent an uncaptured local directly and promote it to a session-owned
cell when a factory first captures it. Declaration and visible-loop binding re-entry create
new generations; assignment changes the existing generation. This permits immutable
compiled code to be shared across workers while each session retains private mutable state.
Reserve identities, memory and work before publishing a new closure or promoting any local.
An allocation failure may consume budget but must not expose half-created closure state.

Operand timing is part of the source program contract. Arithmetic and comparison can
retain an active local as a live left operand until the right expression completes, whereas
computed operands and inherited captures are read earlier. Concatenation, call targets,
receivers and preceding arguments/results retain their established timing. Represent these
choices explicitly in the lowered program rather than rediscovering compiler behavior in
the evaluator. Computed constructor-key timing is a separate admission requirement.

The optional oracle observer needs cumulative bounds as well as a per-dump cap. Charge
retained metadata and preflight temporary source-VM/dump/decoder allocations against the
remaining observation budget. Reflection, prototype dumps and lexical source mapping must
agree; a copied or serialized metadata record alone is not proof of its source identity.


## List allocation and source execution history

An allocation facet must bind every admitted list occurrence to its actual prototype,
bytecode position, instruction and exact expression range. Multiple sites in one body
and sites within child factories retain separate inventories. Keyed/template allocations
must not shift the correspondence for otherwise supported list sites. Parenthesized
expressions preserve both scalar-result adjustment and the inner allocation range.
Allocation metadata is injected with the source package; the native engine must not
infer VM storage from a particular skill, cache key or build.

The source allocation hint describes initial physical capacity. Decode reserved hints
according to the identified runtime profile and charge native reserved storage before
publishing the table. Evaluate list fields in source order, preserve nil slots and expand
only the final unparenthesized call/varargs result pack. Runtime expansion must preserve
side effects and failure prefixes, with bounded storage allocation.

Initial capacity is not proof of every later table history. The pinned interpreter can
resize a bulk final-result store differently from a compiled trace that emits individual
stores. Equal entries can consequently have different lengths, unpack results and valid
iteration controls. Retain original cold/warm counterexamples. A path that exceeds its
established storage history must invalidate layout evidence until the implementation
can model the possible histories and prove the requested observable independent of them.
Do not choose the interpreter layout silently or grant history by raw-value equality.
This uncertainty is a remaining dependency for full parity, not a reduced end-state goal.

List traversal, boundary selection and ordinary later writes require separate validation.
A retained initial capacity can justify numeric traversal while a holey default-unpack
boundary remains ambiguous. Original cache hits, misses, copies, errors and subsequent
mutations must exercise these distinctions through the complete public parser.
