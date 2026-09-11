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
whole bodies through the same syntax implementation. The domain extractor must separately
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
that a constructor ran. `invoke_method` and the shared dynamic-call instruction resolve the
receiver once and look up the target before argument evaluation. Argument side effects
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
metatables. Class projections need a separate domain adapter; the plain observer does not
certify class readiness or numerical effects.

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
