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

## Configuration integration

The first source-method gate is the complete original `ModList.AddMod`: implicit `self`,
its captured original `table.insert`, explicit plain-table receiver arguments, ordered
writes, duplicate table references, shared
nested tags, nil insertion and failure behavior. A full PoB runtime supplies the independent
reference. This gate does not establish `NewMod`, replacement methods or effective settings.

Method calls require more than prepending `self`. Bind each supported method to its source
closure and verify the actual receiver lookup before argument evaluation. Preserve
inheritance/parent behavior and reject unrepresented overrides. `Common.new` also creates
raw parent-proxy records containing callbacks and class references; this is not solely a
metatable concern. Preserve their receiver and initialization semantics before claiming
constructed-instance readiness. Add `NewMod`, `AddMod`,
`ReplaceMod` and `ReplaceModInternal` through that shared dispatch, not hard-coded
configuration keys. Control methods and parser-service calls also need explicit effect
permissions and source-bound ownership.

Then execute initial defaults and saved activation in the original lifecycle order, using
Twister and Skeletal Sniper together and all five supplied builds for structural coverage.
Retain separate UI/control notifications and configuration placeholder writes, captured
quest closures, injected monster/boss definitions and precise unsupported frontiers.
The [configuration lifecycle](configuration-preparation.md) remains the integration contract;
complete build numerical parity is a later, unchanged requirement.
