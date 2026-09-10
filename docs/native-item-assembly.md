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
