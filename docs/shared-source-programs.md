# Shared source programs and per-build sessions

The typed-program engine serves modifier parsing, configuration preparation and future
source algorithm consumers. Each domain owns its input and effect policy; there is one
verifier, compiler, interpreter and table heap. The [implementation record](implementation.md)
tracks admission and parity separately from this architecture.

## Immutable definitions and compiled code

`SourceProgramOwner` retains a validated graph of tables, closures, captured values, named
roots and source provenance. Roots and callbacks belong to that exact immutable owner.
Table aliases and cycles remain graph references. The current immutable graph represents
string and exact integer keys; it does not claim arbitrary Lua class-table coverage. The
session transport supports broader key types under its separate validation. Distinct
closures may have the same source
body while retaining different captures. Names are lookup keys, not proof of identity.
Bound root handles retain their owner and reject resolution against another snapshot.

The parser adapter retains the original `ModifierParserCatalog` allocation. It does not
copy the graph or construct parser-shaped placeholder definitions for other domains.
`ParserProgramCatalog` and `CompiledParserPrograms` remain compatibility facades over
`SourceProgramCatalog` and `CompiledSourcePrograms`. The neutral modules re-export the
same existing graph/IR types, so parser names and serialized forms remain compatible.
Legacy parser roots resolve through the parser adapter; standalone programs use their
own named roots. A root absent from the owner is an invalid binding.

Compilation is structural validation, not source authentication or numerical admission.
The optional PoB lowerer authenticates complete file and function-span hashes and lowers
whole bodies through the same syntax implementation. Standalone table-inline closures use
balanced token extents within the authenticated line span; byte offsets and the complete
body remain verified, and same-line ambiguous functions are rejected. Ignoring the
surrounding table syntax does not permit skipping an unsupported interior statement.
The domain extractor must separately
prove actual closure captures, original environments and runtime primitive identities.
A label claiming `table.insert` cannot by itself prove which function a source closure
captured. Parser extraction keeps its stronger full reconstruction/comparison check.

Unsupported bodies remain diagnostics. Missing captured callees invalidate their callers
transitively; an unreachable unsupported branch still prevents whole-function admission.
Generated metadata or a successful compiler run must not silently enable a game mechanic.

## Mutable state belongs to one build

A `ProgramSession` retains one compiled library, one heap and cumulative execution budgets.
An explicitly supplied initial state graph is writable. Later borrowed argument graphs and
all definition tables are read-only. Separate sessions may share compiled definitions while
running concurrently with private state; there is no global evaluator lock or Lua runtime.

`SessionValue` is an opaque handle tied to one session. Cloning it preserves aliases.
A handle from another session is rejected even if its numeric table index would collide.
Sequential callback calls use the same heap. Exporting a snapshot neither consumes the
session nor creates a writable handle into it; output retains the real definition owner.
The parser's existing invocation API continues to treat argument tables as borrowed.

Step, pattern, value, byte, table, call-depth and result-pack limits apply to the preparation
request. Imports, failed operations and snapshots retain their charges. Counters describe
bounded cumulative work/allocation, not measured peak resident memory. Source failures may
leave earlier writes visible, as in Lua; callers must not infer rollback from an error.

This is preparation machinery. Resolved native calculation plans still use immutable typed
inputs and private scratch without XML parsing, file/database access or VM setup per search
candidate. Any move of source-program execution into a repeated calculation path requires
its own throughput/allocation measurements and explicit planning.

## Source class and method protocol

A standalone owner can retain a separate `SourceClassDefinitions` catalog. Class IDs,
actual class-table identities, ordered parents and observed superclass iteration belong
to that owner. Bound class handles reject foreign owners. Class names, initialization
fields and proxy fields come from injected descriptors; game class names do not select
native behavior.

A class-table projection is explicitly incomplete. Every omitted field remains an
unsupported dependency when reached, including through raw proxy lookup. The typed
superclass descriptor preserves the class-key set and its observed iteration order without
claiming that the immutable string/integer-key graph represents arbitrary class tables.
Constructor metadata retains the actual published wrapper, original callback and complete
captured class/name/helper bindings. Structural validation checks consistency and primitive
bindings; the extracting domain must authenticate the original source and observations.

`ProgramSession::allocate_instance` creates private instance/proxy state without implying
that a constructor ran. `invoke_method` and dynamic method instructions resolve the
receiver once and look up the target before argument evaluation. General function-value
calls retain their evaluated callee and do not prepend a receiver; `invoke_callable` uses
the same session and ownership checks. Argument side effects
precede a non-callable-target source error. Raw overrides, nil restoration, inherited
callbacks and callable parent proxies retain their different lookup rules. In particular,
a parent proxy checks raw object fields before its parent class; it does not use the
child's prototype as a fallback. Its missing writes forward to the object, while raw
`table.insert`/`ipairs` operations bypass proxy lookup and forwarding.

The bounded Common construction protocol checks source-bound prototype/cache state,
parent constructor calls and initialization markers. Method and original constructor bodies
run through the same compiled source-program engine. The Common allocation, wrapper and
proxy protocol itself is modeled by generic Rust operations with explicit source policy;
this is not a claim that complete Common functions have been lowered. Allocation currently
requires the already captured constructor wrapper and class cache. First-use cache writes,
alternate prototypes and instance mix-in calls remain explicit unsupported boundaries;
shared definitions must not be mutated to simulate them. Changed helper
captures, unrepresented metamethods or unsupported construction state must fail explicitly.
Source failures preserve writes already performed and consume the same cumulative budget.

The plain `ProgramValueGraph` transport does not encode class/metatable behavior. Export
therefore rejects reachable behavior-bearing instances, proxies or partial class definitions
instead of silently flattening them. Consumers can retain opaque session handles or export
explicit plain result projections. Repeated external arguments retain cross-call aliases
by reusing imported handles; independent graph imports cannot infer shared identity from
matching numeric IDs.

The optional `SourceClosureObserver` captures primitive identities before source loading,
then reads actual Lua closure upvalues and plain captured graphs with bounds and source
spans. It rejects unknown native callbacks and unrepresented metatables. Captured primitive
identities and the original string-method environment remain authenticated; ordinary globals
use the separately observed environment values. Its constructed-class API takes explicit requested classes/methods, complete
named definition roots, original allocation helper and exact source aliases. It authenticates
the closed Common protocol, records omitted class fields and captures original callback
identities without invoking constructors. Callers still establish the source-loading order
and construction/cache state; observation does not certify numerical effects. Generic
partial global/build projections use the explicit coverage boundary below.

## Configuration integration

Admit complete modifier-store consumers (`NewMod`, `AddMod`, `ReplaceMod` and
`ReplaceModInternal`) together with their original constructor and `createMod` dependencies.
The latter can use generic type inspection, vararg selection and comparisons rather than a
configuration-specific helper. Compatibility parser extraction retains its existing
capability policy; standalone language growth must not silently change packaged parser
programs or their admissions.

Use a complete original PoB runtime as the independent reference. Compare explicit class
identity/initialization observations, ordered modifier state, aliases within and across calls,
parent-list replacement, raw overrides and failure prefixes. A successful method gate does
not establish all class behaviors, effective settings or whole-build calculations.

Then execute initial defaults and saved activation in the original lifecycle order, using
Twister and Skeletal Sniper together and all five supplied builds for structural coverage.
Retain separate UI/control notifications and configuration placeholder writes, captured
quest closures, injected monster/boss definitions and precise unsupported frontiers.
Control methods and parser-service calls need their own source-bound effects and permissions.
The [configuration lifecycle](configuration-preparation.md) remains the integration contract;
complete build numerical parity is a later, unchanged requirement.

## Configuration dependency boundaries

Environment/table coverage is implemented as a generic source-program facility. Live
Session-owned capture instances use the contract below; the source-authenticated parser-service bridge remains planned.
Structural compilation and successful component execution do not admit a whole configuration
activation or numerical build. Retain existing parser artifacts and admissions throughout.

### Environment and table coverage

Partial `data`, `modLib`, build and control graphs must distinguish a present value, a
proven absent key and an unavailable value. Only proven absence becomes Lua `nil`.
Unavailable reads stop with an explicit dependency diagnostic; they cannot participate in
truthiness, `or` fallback, type inspection or argument coercion as a synthetic Lua value.
Enforce this at the shared raw-read boundary so ordinary indexing and proxy helpers agree.

`SourceProgramContext` is an immutable schema-1 sidecar attached to `SourceProgramOwner`,
separate from `SourceProgramDefinitions` and the legacy parser graph. `new_with_context`
creates a fresh owner identity; a context cannot be swapped under existing bound handles.
`SourceTableCoverage` records the complete/selective inventory, known-absent and unavailable
text/exact-integer keys, and independently unavailable `__index`/`__call` behavior. Bounded
validation rejects overlap, dangling references and duplicate serialized keys. Generic
coverage cannot overlap an existing constructed-class descriptor.

Metadata remains attached to table identity. A complete observed key inventory can prove that an unlisted
key is absent; a selective projection must treat an unlisted key as unavailable unless
absence was independently established. Known omitted values remain unavailable even when
the key inventory is complete. Unsupported key types and unknown metatable behavior must
not silently become absent keys or plain-table behavior. Absence evidence belongs to the
actual lifecycle checkpoint: an unprepared component is not necessarily absent in PoB.

Iteration needs its own coverage evidence. `pairs`, `ipairs` and length must not expose a
truncated projection as a complete collection or terminate at an unavailable entry.
Preserve observed iteration order where ordered effects depend on it; otherwise retain an
explicit unsupported frontier. Successful writes and deletions update coverage in private
session state. Unknown prior presence blocks effects whose metatable routing depends on
that presence. Failed reads or writes retain earlier source writes and cumulative charges.
A partial table snapshot must retain coverage metadata or reject export through the plain
graph transport, which has no representation for unavailable values.

`SourceCaptureContext` selects actual table identities/raw fields and optionally an actual
global environment. The observer registers all projections before recursion, preserving
aliases and cycles; it enumerates the complete raw inventory and marks omitted values
unavailable. Unsupported key domains are rejected. Index and call fallback require separate
explicit opt-ins and remain unavailable; other unrepresented metamethods are rejected.
This permits plain raw-field calculations on actual PoB class objects without pretending
to implement their inherited methods or mix-ins.

The context designates an explicit owner-bound root for the actual original global
environment. A table root can expose scalar, callback and table values without changing
legacy root kinds. Global reads can then use the existing indexed-read operations while
preserving lexical shadowing, source evaluation order and owner identity. A root name alone
is not evidence that its contents were the original environment. The observer must prove
environment, table and function identities, including original primitive library fields
and string-method lookup. Bind `new`, `round`, `StripEscapes` and parser helpers through
observed source callbacks or separately admitted source protocols; their names cannot
select substitutes. Ordinary globals may have been rebound at observation time; their actual
captured values determine execution. Captured primitive function identity and the original
string metatable/lookup are verified separately. Constructed-class capture still verifies
the primitive globals required by its closed Common host protocol. Runtime global writes
and dynamic environment iterator invocation remain explicit frontiers.

`session_with_coverage` imports writable state, `borrow_with_coverage` imports immutable
arguments, and `import_with_coverage` admits fresh writable producer results. Transport table
IDs are local to each import; reuse opaque handles to retain cross-call identity. Private
writes resolve unavailable values and deletions establish raw absence without mutating the
shared owner. Missing ordinary reads with unknown `__index` fail; raw absence still yields
nil. Unknown `__call` fails at invocation after argument effects. Safe snapshots require
complete reachable coverage and no unavailable index/call behavior. Resource limits include
coverage storage and failed imports.

### Live controls and captures

Control notifications need closures whose mutable captures refer to the same live build
session as their caller. ConfigTab's numeric `changeFunc` captures the ConfigTab object and
its option descriptor; `SetPlaceholder` writes a string on the control, then optionally
invokes that closure to write a numeric configuration placeholder. Capturing the ConfigTab
as an immutable definition table or copying it per call would lose writes and aliases.

Keep shared code and immutable definitions in `SourceProgramOwner`; carry live values in
a separate, coherent `SourceSessionInput`. Its owner-bound closure prototypes have ordered
capture layouts whose slots are explicit `LiveCapture` markers, including slots that will
hold scalars or references to immutable definitions. The markers cannot appear in ordinary
captured tables or legacy parser callbacks. `SourceClosurePrototypes` is a separate schema-1
sidecar; `new_with_closures` creates a fresh owner rather than rebinding existing handles.

`SourceSessionInput` carries a graph and its coverage, a cell arena, and distinct closure
instances referencing prototype handles and ordered cell IDs. `DefinitionTable` references
retain read-only owner data; state table IDs retain private writable state. Two closures
may share a cell, and one cell may contain a table or another closure. Artifact-local
table, closure and cell IDs are one-based. Named `ObservedSourceSession` root indices are
zero-based positions in `state.values` and the returned session-root vector. Equal values
never imply shared cells. The domain-neutral graph types live in the data crate; existing engine
`ProgramValue`/graph/table names remain aliases. Plain graph imports reject artifact-only
closure and definition references instead of silently inventing an owner.

`session_from_input` creates a private session; `import_session_input` admits a new coherent
artifact into an existing session. Validate owner identity, all references, layout lengths
and resource bounds before publishing imported state. Failed imports retain their resource
charges. Opaque values retain owner/session identity across calls. Bare prototype callbacks
cannot substitute for closure instances, including zero-capture functions. Function keys,
equality, dynamic calls and captured assignments operate on the live instances and cells.
`CaptureSet` evaluates the whole right-hand value list before storing its first value (or
nil), preserving prior effects on failure. Snapshots reject closures whose identity/cells
cannot be represented by the plain result graph.

Preserve the build/ConfigTab cycle, selected-set input and placeholder aliases, control
identity and repeated callback identity across calls. A handle from another owner or session
must fail before its state can be used. The source bodies of control methods and notifications
need their own admission; no-op controls or per-setting Rust handlers cannot establish it.

The first live-capture gate can execute already-constructed original functions without
first lowering their enclosing constructors. `EditControl.lua:110..115` defines
`SetPlaceholder(self, text, notify)`: write the string placeholder, then conditionally call
`self.changeFunc(self.placeholder, true)` with two arguments and no implicit receiver.
The numeric closure at `ConfigTab.lua:315..324` captures the ConfigTab instance and the
current option descriptor. Both complete bodies are individually lowerable; general native
construction still needs nested-function creation and the preceding constructor lifecycle.

Represent a session closure as a distinct identity referencing shared compiled code and
ordered capture-cell references. Authenticate its complete source occurrence, actual function
and environment identities, slot order and cell/table identities from one construction/state
observation. Observe cell sharing; equal capture values do not prove shared cells. Do not
expose an unchecked replacement of immutable upvalues by name. The optional PoB
`observe_session` boundary receives actual callback/state roots and explicitly classified
immutable definitions, discovers the live graph and capture cells first, and then captures
definitions while forbidding live tables, functions and shared cells from leaking into them.
This includes an immutable helper that shares a scalar cell with a live closure. Its initial
scalar value cannot be treated as a constant. Actual upvalue-cell identity is observed using
a small pinned-LuaJIT shim inside the optional PoB adapter; native libraries contain no Lua
runtime or pointers. Runtime transport supports table/function keys, but source observation
currently captures the bounded text/integer projection key domain; broader raw source keys
remain an observation frontier. The present observer creates a fresh owner for each observation and
interns prototypes only within that observation. Native sessions can share an existing
compiled owner, but reconciling a new source observation against that owner still needs an
explicit authenticated binding protocol; it is not implied by matching names or source text.
Preserve the captured ConfigTab object while resolving the currently active set at invocation
time. Inherited calls use the class/state binding below; copying inherited methods into raw
fields would change source semantics.

Acceptance should include multiple controls sharing ConfigTab state, repeated notifications
after a set switch, false/nil notification avoiding an unavailable callback, Lua numeric
conversion edge cases, foreign-session handles, foreign prototype owners, invalid artifact-local
cell references and staged failure effects. Failed
string conversion precedes the placeholder write; callback failure follows it. The original
non-placeholder branch also writes input before `AddUndoState`/`BuildModList` and only sets
`buildFlag` after those calls. Preserve the whole branch even while its later consumer is
unsupported.

### Existing class instances and shared methods

A coherent session input can associate an actual writable state table with an owner-bound
`SourceClassHandle`. `SourceSessionClassBindings` and `ClassResolved` index coverage must
match one-to-one. Plain graph imports and immutable table contexts cannot carry this marker.
The native importer checks owner identity, local table bounds, the captured self-indexing
class protocol and independent call behavior before publishing any state. It attaches lookup
behavior to the captured table; it does not allocate a replacement instance, execute a
constructor, or synthesize `Object`, `_parentInit` or parent proxies.

Raw lookup still comes first. An omitted present raw method is unavailable, a known-absent
method can resolve through the actual captured class table, and deleting a raw override
reveals the inherited method. Common copies inherited fields into class tables during
construction; native lookup uses that observed result rather than inventing another parent
search. Preserve lookup-before-argument ordering and keep `__call` independent: Common's
unmodeled mix-in call remains unavailable after argument effects. Other unmodeled
metamethods reject observation/import. Iteration sees raw entries, and plain snapshots cannot
silently flatten away class behavior.

The optional `observe_session_with_classes` entry point composes class definitions and live
state into one owner. Instance membership comes from the actual metatable identity and the
selected registry class, not a class name supplied for the instance. Selected shared methods
and their source helper functions retain one owner callback identity when referenced through
class lookup, state or captures. An explicit session-closure request for that same function
is contradictory and rejects. Constructors may be retained with unsupported bodies when
they are unentered; observing an existing instance does not prove constructor execution.

Shared class-method table captures require positive immutable ownership through explicitly
declared definition roots/projections or authenticated class tables. A hidden per-instance
table does not become a constant merely because no live root exposes it. Declared immutable
table edges can extend that graph; function captures cannot bootstrap their own state into
it. Known live tables, functions and shared capture cells reject across this boundary.
Session prototypes, including zero-capture prototypes, cannot be called as shared class
methods or their transitive helpers. Class members with live captures need an explicit future
per-session member binding; the current observer reports that frontier rather than freezing
those captures. Compiled owners remain reusable by private native sessions.

### Parser results and explicit effects

The parser service requires an explicit source-bound bridge into the same preparation
request and resource budget. Keep dispatch permission separate from structural compilation.
A missing implementation must report an unavailable dependency, not imitate the parser's
legitimate no-match result. The caller branches on that distinction.

Returned modifier graphs need the ownership that their next source consumer requires.
`modLib.setSource` mutates both a modifier and its nested modifier, and returns the same
object. Plan fresh writable session results with preserved aliases, rather than importing
those results through the existing read-only argument path. Immutable package definitions
and caller-borrowed input remain shared and protected. Prove result identity, mutation and
failure behavior before admitting custom modifiers and generated quest callbacks.

### Resume and acceptance cases

The first consumer gate now compares complete original `ConfigTab.UpdateLevel` using an
authenticated environment projection and actual live entry-state graphs. All five originals
exercise direct and boss-callback calls in both initial and saved passes (20 paired actual
calls), followed by continuing writes/source failures and branch-dependent unavailable state.
The test producer is bounded and explicit; it is not production configuration preparation.
The bounded live-notification gate also compares complete original `SetPlaceholder` and
numeric `changeFunc` bodies, including reused controls, unavailable short-circuits and
failure-prefix effects. The inherited-method gate adds continuing actual callback sequences
through source-authenticated class bindings. Native closure construction and the complete
ordered defaults/saved-activation lifecycle remain outstanding. Keep the five-build
structural matrix and the Twister/Skeletal Sniper pairing as coverage, not runtime presets.
The complete ConfigOptions callback inventory should determine later dependency work.

The next gates must establish these source-driven cases:

- `UpdateLevel` reads injected `data.misc.MaxEnemyLevel`, chooses explicit input,
  placeholder or character level in source order, and does not demand unavailable values
  from an unexecuted branch.
- `BuildModList` distinguishes a genuinely absent `varData.apply` from an omitted real
  callback. Unavailable callee lookup stops before argument effects; a known non-callable
  result fails only after those effects, matching the existing call-order contract.
- `SetPlaceholder(value, false)` changes only control state; notifying calls also execute
  the actual closure and update the active set. Boss callbacks and `enemySizePreset`
  exercise this difference before later options consume the resulting placeholders.
- Active-set changes preserve `self.input` and `self.placeholder` aliases. Boss preset
  lookups preserve optional-field absence, dynamic keys, selected control values and
  iteration coverage; unavailable boss data must never remove an effect silently.
- Partial collections do not report fabricated lengths or complete iteration. Writes,
  deletions, failure prefixes, repeated aliases and foreign-owner/session rejection retain
  their semantics under projection coverage.
- Parsed custom and quest modifiers preserve writable nested aliases through `setSource`
  and ordered insertion. Unsupported parsing remains visible, and shared definitions are
  unchanged after execution.

These cases derive from `ConfigTab.UpdateControls`, `UpdateLevel`, `BuildModList` and
`SetActiveConfigSet`; ConfigOptions' generated quest, enemy-size and boss-preset callbacks;
`EditControl.SetPlaceholder`; and `modLib.setSource`. Their source versions and observations
must enter the evidence for each gate. Passing one gate does not admit later lifecycle
stages or complete build evaluation.
