# Native structural modifier parser

The native parser turns modifier text into structural modifier records using injected
source dictionaries. It is a preparation component shared by item, passive and skill
pipelines; parsing text is not intended to run inside each numerical evaluation.
Recognizing a modifier does not certify that its mechanic can be evaluated natively.

The end state is complete native parsing and calculation, with PoB loaded explicitly
for update checks and differential tests. The current parser implements ordinary forms,
static special rules, pure Special callback factories and their modifier/tag/wrapper
construction. Unrepresented callback shapes, other callback call sites and the stateful
`DOUBLED` operation remain explicit pending operations. The
existing Spark/Mace evaluation pipelines and build-admission boundaries are unchanged.
See [the implementation log](implementation.md) for current validation and remaining work.

## Injected definitions

Schema 16 adds `modifier_parser` to the portable package. `ModifierParserCatalog` retains
all 28 constructed dictionaries, including historical special rules, generated skill
names, jewel definitions and helper lookup tables. The current source has 10,027 rows.
No production rule set is selected from the example builds or a fixed list of stats.

`ParserValue` preserves nil, booleans, finite numbers, text, non-finite sentinels, table
references and callback references. One-based graph IDs retain shared tables and closure
identity. Tables keep string and integer keys separately, including sparse numeric keys.
Lua callbacks include authenticated source spans and ordered captured upvalues; source
spans alone do not identify a closure. Named C builtins are opaque runtime primitives,
not serialized copies of their internal C closures. Global-environment descriptors are
source evidence, not permission to execute a callback or load external code.

The extractor runs the complete authenticated source construction. It records 4,390
lexical declarations, final winner records and complete generator dependencies. The
original raw gem assignment loop has three ambiguous base-name mappings: Lightning Bolt,
Mace Strike and Spear Stab. Every possible assignment candidate is retained; deterministic
mappings and unresolved alternatives form an exact partition. Later display-name changes
do not rewrite this lookup. A future helper executor must resolve an ambiguous selected
lookup explicitly rather than taking a sorted or incidental winner.

The package is limited to 32 MiB and two million JSON values; the supervised extraction
envelope allows one additional MiB for evidence. Graph, text, reference, source-span and
policy validation remain bounded. These are implementation resource limits, not game
rules. All 25 preceding package sections retain their values and digests.

## Pure factory definitions

Source-derived pure factory recipes extend the same immutable catalog. Each original
closure retains its source span, captured graph and an explicit lowering disposition.
An admitted recipe records fixed argument slots, scalar literals and captures, policy
field references, negation, ordered table fields and constructor calls. All game values,
modifier names and field keys are selected data. Byte offsets and a function digest
identify the complete function within its authenticated source span; constructor
provenance binds the observed original helper. Modified external packages have their
own identities and do not inherit a claim of parity with the original source.

Constructor operation parity is separate from opaque function-value graph parity. The
preserved catalog descriptor was extracted from the unchanged standalone constructor
body; the full original module captures its local `select` and `type` functions instead.
Their operation identity is verified, but the two lexical closure graphs differ. A
caller-injected recipe returning the constructor as a value remains an opaque callback,
and both source and native results defer at the finite item-metadata adapter. Supporting
function-valued modifiers requires a separate environment/identity migration; this phase
does not normalize away that difference or claim complete closure-graph parity.

A strict source verifier consumes the entire function and proves its referenced
bindings. Unrepresented functions remain opaque. This boundary does not imply a general
Lua runtime: branches, mutation, arbitrary helpers and other callback call sites require
separate source semantics and parity work. Structurally invalid recipes are package
errors; unsupported source forms are per-callback dispositions. Optional captured values
are inspected only when selected execution reaches them.

Special invocation preserves the original converted first argument followed by all raw
captures. Arguments and sparse list positions retain explicit nil values until table
construction. A nil factory result differs from an empty modifier table. The general
constructor preserves name/type/value types and independently classifies its positional
source, flags and keyword flags. Factories use per-request scratch budgets and the
existing final public copy boundary; their constants and definitions can be shared
across threads. Current delivery and validation status belongs in the
[implementation checkpoint](implementation.md#pure-special-callback-factories-checkpoint).

## Execution boundary

`CompiledModifierParser::new(&ModifierParserCatalog)` compiles immutable pattern tables.
It can be shared through `Arc` across preparation workers. `parse(&[u8], &mut MatchBudget)`
returns structural records and optional remaining bytes. Source nil, an empty modifier
table and an extra string remain distinct. Outputs preserve scalar types, exact finite
number bits and non-finite values; opaque callbacks retain catalog identity.

The shared byte-pattern engine follows LuaJIT 5.1 semantics: string and position captures,
classes, balanced/frontier matching, backreferences, quantifiers, anchors, plain search and
lazy syntax errors. Matching uses ASCII byte case conversion. It never applies Unicode
case folding or assumes that a capture is valid UTF-8. Numeric parsing and the source's
signed-low-word `OR64` operation are separate portable primitives.

The scanner selects earliest start, greatest end and greatest pattern byte length. It
exposes exact ties in caller-supplied iteration order. The parser accepts a tie only when
its effective captures and payload contents agree. Content comparison uses exact number
bits and bounded memoized graph work; a shared DAG cannot trigger exponential comparison.
False payloads suppress the source return only after selection. Later pattern errors are
still observable. Removal uses the original-case bytes, while captures use lowercase bytes.

Ordinary parsing preserves source scan order, conditional order-two retry, sparse
contribution positions, tag ordering, wrapper precedence and positional `createMod`
arguments. All raw modifier rows are constructed before wrappers are applied. Internal
table views can share references; source `copyTable` operations make fresh nested copies.
The public parser return also recursively copies results, matching PoB's cache boundary.

`DOUBLED` requires additional session state: the original parser mutates the shared name
dictionary for composite names. Cached earlier text keeps its old result while uncached
text observes the mutation. The current native parser stops on that operation. The next
implementation must add an explicit bounded session overlay/cache while keeping the
injected catalog immutable and avoiding shared mutable state between workers.

The session/cache contract must also retain function identity: original public table copies
leave closures shared, and returned jewel callbacks can reach the same mutable name
dictionary through their captured `parseMod`. A cache entry is not necessarily an isolated
copy of every reachable environment. An injected host error after the original mutation
leaves the dictionary changed without caching the failed text; this probe does not claim
that the valid input naturally causes a source error. Cache eviction after a mutation can therefore
change the result of previously cached text; any capacity policy must preserve semantics
or report a resource boundary. A fresh session is an explicit reset, not an interchangeable
continuation of an existing session.

Source errors, resource exhaustion, selected pending operations and ambiguous rule
selection are separate outcomes. Pending callbacks never fall through to a lower-priority
rule. There is no Lua fallback, subprocess, host I/O or runtime game-data lookup in this
engine. Compilation and execution enforce aggregate pattern, compiled-memory, work,
output-value, depth and byte limits before potentially expanding output.

## Item and CLI integration

`NativeModifierParserProvider` converts supported structural results into the current
finite item metadata model. It reports callback payloads, non-finite values and non-UTF-8
output explicitly as pending conversion/assembly dependencies, rather than discarding or
coercing them. Dense nonempty numeric-only nested tables become metadata arrays; empty,
sparse or mixed tables retain their table representation. Root modifiers remain metadata
tables. `from_compiled(Arc<CompiledModifierParser>)` lets hosts share compiled rules.

`BuiltinItemLoadProvider` combines native formatting with native parsing. Known lines can
advance to the separate assembly dependency; unresolved callbacks still stop at parsing.
The formatter's internal precision requests use the same explicit parser and retain their
ordered trace. `inspect-build --with-definitions` uses this provider without evaluating a
build or granting numerical support. Implementation fingerprints include every parser,
matcher, numeric helper and conversion source file.

The prior narrow profile grammars remain in use by their admitted numerical pipelines.
Their migration requires separate differential tests; changing parser availability does
not silently broaden those pipelines.

## Validation requirements

Validation uses unchanged original LuaJIT functions and the original public ModParser,
complete source dictionaries, adversarial byte inputs and every parser request observed
in the five caller builds. It distinguishes successful structural comparisons, explicit
native deferrals and observer limitations. Callback descriptor comparisons do not prove
native callback execution. Cold and warmed source behavior, errors, alias/copy behavior,
source mutation sequences and resource rejections are checked separately.

Caller-controlled catalog tests cover conflicting captures, signed zero/NaN, shared DAGs,
output expansion, source error ordering, compilation bounds and concurrent requests.
Package reproduction, source preservation, native-only dependency checks, WASM compilation
and complete-build reference measurements remain separate gates. Current results and exact
commands belong in [the living implementation document](implementation.md).
