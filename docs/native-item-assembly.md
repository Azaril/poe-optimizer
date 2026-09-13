# Native item assembly

The target is a complete native item-preparation pipeline driven by injected game data.
It produces immutable modifier structures that the native build compiler can reuse
across parallel candidate evaluations. PoB remains an optional, pinned differential
reference. Assembly has no Lua runtime, subprocess, network or global mutable state.
See the [implementation record](implementation.md) for the current capability boundary.

## Ownership and data

`poe-optimizer-data` owns immutable source definitions, `poe-optimizer-engine` owns
portable value/list mechanisms, and `poe-optimizer-import` owns ordered item loading and
assembly. Engine code must not depend on import-owned `ItemState`.

An additive `ItemAssemblyData` section and `ItemAssemblyCatalog` describe closed,
source-authenticated policy groups: row collection, range rewriting, local queries,
rune effects, grants, named compatibility, attribute requirements and slot selection.
Game names, patterns, labels, modifier selectors, masks and numeric defaults are data.
Source field structure, arithmetic order, copy semantics and native operation kinds
are code. The catalog is not a general instruction language or a build whitelist.
Its current section schema1 declares `PolicyOnly`; loading and binding it supplies
validated definitions, never an admitted executable item. Complete-method provenance
also does not imply lowering of every specialized slot branch.

Reuse the existing complete base records, rune definitions, scalability policy,
precision map and keyword definitions. Verify shared source facts during extraction.
In particular, a live keyword flag and a mask captured when a helper was constructed
are separate dependencies; changing one must not silently regenerate the other.
Keep requirement operand order and the order of independent compatibility rules.

Definitions validate required roles, references, provenance and resource bounds.
Pattern and replacement text can include NUL within their bounded UTF-8 data domain;
matching and output operate on bytes. Unused malformed patterns remain lazy source
errors. Catalog availability describes definitions, not completed item assembly or
numerical support for their effects.

## Mutable preparation, immutable results

Preparation needs object identity. A duplicate variant can append the same modifier
several times; subsequent source writes must reach every alias. A source copy later
creates independent objects, including independent copies of previously shared children.
Cloning every append, copy-on-write mutation, and graph-preserving deep copies each
change this behavior.

Use a bounded item-local arena with stable handles for tables, modifiers and rows.
Ordered lists retain handles until the original copy operation is reached. Source copy
operations recursively copy values without an identity memo; table keys and external
class identities require their own explicit representation. Charge nodes, references,
bytes, copies and work before allocation. Unsupported cycles or values remain explicit.
Freeze completed reachable state into immutable storage for worker sharing. Mutable
scratch is local to a preparation task and needs no shared lock.

Input adapter limits remain distinct from runtime values: current finite UTF-8 metadata
admission is not permission to discard non-finite arithmetic, opaque functions, sparse
positions or unexpected intermediate types. Preserve representable raw values and exact
number bits; otherwise stop at the reached representation boundary.

## Progress and provider boundary

Assembly progress carries the reachable item state, ordered dependency events and one
of Complete, Pending, SourceError or ResourceError. State includes base/single/slot/buff
modifier lists, row-owned Bonded lists, range-row references, grants, requirements,
sockets, restrictions and cache fields. Absent, empty and unchanged are different states.
ModStore named fields are retained until the source converts a list to a plain array.

Earlier mutations survive a later failure. A slot result becomes an assigned item field
only after that call returns, while item mutations made inside the failed slot remain.
Retain sticky fields and opposite slot lists across re-entry exactly as the source does;
clear the active Bonded cache only after all slots succeed. Returning progress does not
promise arbitrary instruction-level resumption: retry must explicitly re-enter the
source operation or start from a retained input snapshot.

Parser and formatter events carry assembly invocation, phase, row/category identity,
optional authored index, rune provenance, slot and ordinal. Record calls before invoking
the provider, including failures and nested formatter precision parses. Preserve actual
argument count and nil positions. Do not invent authored line indices for generated rows.
Explicit caller providers remain supported; legacy atomic complete results retain their
restricted representability. No hidden fallback or restart with a different backend.

## Source-ordered operations

Collection handles disabled/extra/Bonded rows, duplicate variants, source assignment and
range callbacks in the original order. A range parse occurs before the zero-line check;
an empty replacement is different from leaving the old payload unchanged. Reuse existing
variant counting, with an explicit boundary if injected alternate-count policy exceeds
its represented state capacity.

Local queries consume matching modifiers in order. Earlier removals survive an error
on a later value; the failing value is not removed. Lua truthiness, numeric coercion,
first-tag selection, AND64 and MatchKeywordFlags govern selection. Nil-config queries
must filter before requesting EvalMod, so an unrelated tagged modifier does not block
assembly. Reached unsupported evaluation remains a specific dependency.

Slot selection uses the source's base fields and separate primary-name/dispatch rules.
Rune caster tags are not interchangeable predicates. Substitute tag strings with the
shared LuaJIT-compatible replacement primitive, including replacement percent semantics.
Preserve source naming quirks, every reached slot and each copy/filter boundary. Rune
scaling, requirements, sockets, crafted quality, Spirit and charm limits complete in
source order or report their exact frontier. Assembled grants are metadata until a
separate actor evaluator supports their effects.

## Completion and proof

Complete means every reached operation, every slot call and final cache mutation has
finished. A collected modifier prefix is not a completed item. Selecting a scope from
source predicates is valid; selecting production support from current item/build IDs
is not. Any representative fixture set is a validation set only.

Differential tests call unchanged complete original methods with an uninstrumented
control. Compare the whole reachable graph, exact numbers/bytes, aliases, source copy
boundaries, row references, requirements, lifecycle state, callback order and failure
prefixes. Include changed injected names/policies, empty and missing values, all common
branches and adversarial resource limits. Warm execution claims require completed live
traces of the actual original methods; repeated calls alone are insufficient evidence.

General actor/action modeling and conditional-parser operations remain separate design
decisions. This item-preparation contract does not approve them or claim full native
build parity. The delivery sequence and measured breadth belong in the
[living implementation record](implementation.md).

## R2y implementation checkpoint

The producer and ordered inventory prefix use the existing injected catalog. Common assembly
and accessory slots are implemented; specialized local data and reached unsupported callback/tag
evaluation remain explicit. Owned assembly graphs, opaque item/catalog/attempt bindings,
preparation report schema 4 and CLI exposure are implemented. All supplied native builds remain
**0/5** complete.

`prepare_authored_items` retains authored occurrences and the source-order registration prefix.
Duplicate numeric IDs update only the lookup winner while retaining both instances and insertion
order. The stage stops before unimplemented item-set/slot/trade instructions or repeated root
sections. Equipment activation, actor effects and completed root loading remain open. Diagnostic
loader completion cannot authorize production registration. All five original public CLI runs
retain their 116 records but stop before the first registration: two at armour, two at
jewel/radius and one at charm dependencies. Accessory component matches below therefore do
not represent completed original inventory prefixes.

Source validation retains all 116 original records: 32 eligible accessories match using an
explicit original-parser dependency and 31 also match through the built-in native parser. The
remaining native case reaches unsupported parser callback 168. The other 84 records are excluded
by the local-family predicate and retained in the denominator; this lane does not execute them. Comparisons cover the declared assembly-field/row-field graph,
including aliases within it; they do not close the whole-object contract above. Five separate
control imports preserve selected views, output scalar bits, inventory order and 114 available
item graphs; two callback-bearing graphs remain unavailable. Directed coverage adds 30 variant
vectors, 15 range/reparse/reassembly steps and five malformed-requirement failure prefixes.

Binding/isolation, graph bounds and retained failure state, all-five inventory denominators,
native-only CLI and five portable WASM libraries pass. The source target's eight tests pass in
41.36 seconds on this Windows release run, including ten reference hosts. This is validation
cost, not hot candidate throughput. Final receipts are in
`runs/r2y-item-assembly-01/source-parity-04/`; the living implementation record lists other checks.

The metadata-tree adapter cannot recover original aliases or executable callbacks. Reference
parser conversion refuses these rather than stripping them. The full dependency event contract
above remains a completion gate: current sequence metadata is local to an assembly attempt after
its retained loader prefix, not a globally monotonic history or proof of actual argument arity.
Diagnostic projection refusal retains the owned graph without final exposure and stops the
machine before stale projected state can be reused; a fresh coherent load is required.

Count the producer, graph/adapters and differential tooling in the
[execution-model investigation](rule-execution-model-investigation.md). It selects no winning
architecture. Specialized locals, item-set activation, equipment/actor integration, lossless
acquisition and dependency events are the next integration gates; A3 remains the discussion
before a significant model migration.
