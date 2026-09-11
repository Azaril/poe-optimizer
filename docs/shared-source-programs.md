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
spans. It rejects unknown native callbacks, changed primitive environments and unrepresented
metatables. Its constructed-class API takes explicit requested classes/methods, complete
named definition roots, original allocation helper and exact source aliases. It authenticates
the closed Common protocol, records omitted class fields and captures original callback
identities without invoking constructors. Callers still establish the source-loading order
and construction/cache state; observation does not certify numerical effects. Generic
partial global/build projections are a separate planned boundary below.

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

## Next configuration dependency gates (planned)

The following contracts are planned dependencies of effective configuration preparation.
They do not claim that general table coverage, live capture APIs or the corresponding
numerical effects are implemented or admitted. Existing class-projection checks establish
only their documented class boundary. Settle concrete API names and serialized forms after
the source acceptance cases below; retain existing parser artifacts and admissions
throughout.

### Environment and table coverage

Partial `data`, `modLib`, build and control graphs must distinguish a present value, a
proven absent key and an unavailable value. Only proven absence becomes Lua `nil`.
Unavailable reads stop with an explicit dependency diagnostic; they cannot participate in
truthiness, `or` fallback, type inspection or argument coercion as a synthetic Lua value.
Enforce this at the shared raw-read boundary so ordinary indexing and proxy helpers agree.

Plan bounded coverage metadata attached to table identity, separately from the existing
parser graph representation. A complete observed key inventory can prove that an unlisted
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

Designate an explicit owner-bound root for a projection of the actual original global
environment. A table root can expose scalar, callback and table values without changing
legacy root kinds. Global reads can then use the existing indexed-read operations while
preserving lexical shadowing, source evaluation order and owner identity. A root name alone
is not evidence that its contents were the original environment. The observer must prove
environment, table and function identities, including original primitive library fields
and string-method lookup. Bind `new`, `round`, `StripEscapes` and parser helpers through
observed source callbacks or separately admitted source protocols; their names cannot
select substitutes. Unrepresented global writes or rebinding remain explicit frontiers.

### Live controls and captures

Control notifications need closures whose mutable captures refer to the same live build
session as their caller. ConfigTab's numeric `changeFunc` captures the ConfigTab object and
its option descriptor; `SetPlaceholder` writes a string on the control, then optionally
invokes that closure to write a numeric configuration placeholder. Capturing the ConfigTab
as an immutable definition table or copying it per call would lose writes and aliases.

Plan source-declared session capture slots or closure instances, bound to authenticated
construction occurrences. Retain immutable captured definitions in the owner and mutable
object references in the session, including shared capture cells where source requires
them. This is not permission to replace arbitrary authenticated upvalues. Preserve the
build/ConfigTab cycle, selected-set input and placeholder aliases, control identity and
repeated callback identity across calls. A handle from another owner or session must fail
before its state can be used. The source bodies of control methods and notifications need
their own admission; no-op controls or per-setting Rust handlers cannot establish it.

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

Start with the complete original `ConfigTab.UpdateLevel` over an authenticated environment
projection and one session state graph. Then establish live notification captures before
running the complete ordered defaults/saved-activation lifecycle. Keep the five-build
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
