# Native condition producers

The native query boundary resolves conditions from explicit tables and ordered FLAG
modifiers. It is shared by numeric aggregation and the supported actor preparation path.
It consumes caller-supplied data and runtime values; it neither loads a build nor resolves
its actor/action graph. The [general build proposal](general-build-input-proposal.md)
remains a separate design discussion. Delivery and validation status belong in the
[implementation log](implementation.md).

## Inputs and ownership

`ConditionProgram` compiles explicit stores, their parent links, actor references and
ordered FLAG records. Every record retains its value kind, masks, source and represented
tags. Store-local numeric and condition values remain runtime inputs, so changing a
candidate's values does not require recompiling unchanged producer definitions. Actor-role
links and a queried skill store are distinct from the actor's own base modifier store.

Compiled per-store indexes retain original record positions and group both complete FLAG
names and `Condition:` suffixes. Lookup visits matching producers in source order; it does
not scan every unrelated modifier or construct condition-name strings for each query.
Programs are immutable and shareable. Runtime bindings borrow the selected query context,
scalar tables and resolved stat tables; they do not own Lua state or spawn processes.

A binding validates store counts, represented values, masks and stat context. Numeric
queries must use the same flags, keyword flags and source filter as their condition
binding. Matching layer counts alone does not mean two independently supplied programs
represent the same build; the higher-level native preparation must bind their provenance.

## Source semantics

Raw condition values and truthiness are separate. Lua treats numeric zero and empty text
as true; only nil and false are false. `GetCondition` can return a raw scalar, while FLAG
returns true or nil. Explicit false overrides suppress inherited values. False local
condition-table entries do not suppress true parent entries. `noMod` suppresses producer
lookup while retaining explicit table inheritance and overrides. An absent condition is
false with `noMod`; ordinary producer-enabled lookup returns nil when absent.

Parent FLAG records evaluate in the queried child's context. Condition tags can fall back
to skill-local values; ActorCondition tags use their target store and preserve the source
actor-role fallback. Negated weapon conditions retain the distinction between an absent
`Added` field and a present false field. FLAG source filtering uses the first nonempty
colon-delimited component and honors `ignoreSourceInCheckConditions`. This is the ModDB
filtering contract. ModList differs for that bypass option; captured ModList records are
replayed only in the common unfiltered contexts until store-kind-specific semantics exist.

The represented producer predicates include Condition, ActorCondition, reviewed global
metadata and ordinary StatThreshold checks. Threshold comparisons retain source operation
order, summed stat order, equality, percentage arithmetic and nonfinite behavior. Resolved
stats are explicit inputs, not proof that their producing calculations exist natively.
Multiplier-derived threshold percentages, unrepresented tags and opaque value kinds remain
explicit unsupported dependencies. They cannot be dropped, even behind an inactive gate.

## Recursion and limits

Condition producers can call other conditions. The query keeps a bounded local stack and
reports reached recursive producer dependencies or depth exhaustion. It does not invent a
fixed point. An override or earlier source-order success can legitimately avoid a cycle;
those short circuits remain observable. Parent-store cycles and invalid actor references
are rejected when compiling the program.

Resource limits are implementation bounds, not game rules: 256 stores/actors, 65,536 FLAG
records, 64 producer frames, bounded compiled-input strings, and at most eight names in one
FLAG query. Borrowed query names and source filters are not subject to the compiled string limit.
Unsupported or exhausted evaluation is an error, never a silently false condition.

## Actor integration and performance

Supported actor preparation supplies its normalized per-layer FLAG records to this
program. Numeric and FLAG queries share the same dynamic attribute-condition tables.
Failures propagate through actor preparation. The existing compiled actor path retains
its specialized boolean predicates; its numeric storage for booleans does not turn a
raw Lua numeric-zero FLAG into false. Generic scalar kinds remain distinct at their input
boundary. Existing stage restrictions keep movement-condition feedback acyclic.

Binding, FLAG, GetCondition and producer-aware SUM can run without successful-query heap
allocations. The existing compiled actor/candidate loops retain their allocation-free
contract. This does not establish allocation-free behavior for every generic modifier
operation: the ordinary MORE implementation still uses a per-layer vector. Whole-build
preparation, general dependency scheduling and search performance require separate
measurement on broadly admitted builds.

## Validation scope

Independent tests execute authenticated original ModStore/ModDB code and helpers. Warm
claims require traces containing the original query functions. Paired tests cover raw
returns, masks, source filters, parent/actor context, ordered predicates, thresholds and
errors. Original corpus records ground selected query replays in the wider build set;
frozen captured context is labeled separately from a complete native build calculation.

Preserve whole-build differential fixtures, original exports, reviewed data and source
identity. A passing query replay neither admits unrelated producers in that actor store
nor proves a complete build's legality or numerical parity. Unknown dependencies and
unavailable captured context remain visible in the breadth inventory.
