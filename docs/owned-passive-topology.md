# Owned passive topology and allocation legality

This contract refines the [domain architecture](domain-architecture.md). It describes
semantic ownership independently of a PoB tree, UI, save format or traversal lifecycle.
The [implementation log](implementation.md) records which parts are delivered.

## Identity and ownership

A passive definition is a physical position in the game's allocation graph. An
allocation is an authored occurrence selecting that position, a point pool, loadout
scope, access source and choices. An external node token is an import key; it is not
necessarily an allocation. Images and presentation-only records are not paid nodes.

Classes and ascendancies declare `implicit_passives` as explicit, closed or partial sets.
The selected character exposes those passive definitions under its Character provider.
They consume no AllocationId or paid point pool. A physical root shared by two owners
contributes once. Changing class/ascendancy changes root membership without rewriting
all physical node identities. Unknown root membership or unknown pool exclusion stays
unresolved; known root effects may be retained as evidence, but cannot close incoming
contributions. A known nonempty paid pool contradicts an implicit-root declaration.

Choices belong to their real owner. A multiple-choice tree branch is one parent
allocation plus an option on its declared choice slot, with the option's effects
included. It is not two paid allocations. Attribute selections likewise keep one
physical node and a typed choice. Choices declared by an implicit root use Character
scope and the exact PassiveNode declaration. Class-dependent stat views are definition
rules reading the selected class/ascendancy; they do not replace physical node identity
with a source option ID. The importer must not use a UI allocation-time default as
proof that an omitted saved choice was authored.

Passive adjacency describes semantic graph connections, including connections that a
renderer omits. Reachability, affordability and activation are independent constraints.
No renderer, source tree object or cached UI total belongs in native evaluation input.

## Scope, cost and budgets

Point-pool scope declares which authored scopes are allowed: Shared, PerLoadout or
Either. It does not define affordability. One ordinary pool may admit shared and scoped
allocations while several constraints account for those same allocations differently.

The legality package should inject:

- Node/pool costs, including explicit zero costs where game rules require them.
- Budget constraints with named aggregation semantics and a capacity stat.
- Access prerequisites and exceptions, including scope-aware root reachability.
- Completeness for costs, capacity contributors, alternative membership and access.

For the reviewed game rules, ordinary usage is shared cost plus the **maximum** scoped
cost across weapon alternatives. A second constraint bounds **each** alternative's
scoped cost by its weapon-point allowance. These operate on the same allocations;
splitting them into unrelated point pools loses their coupling. An inactive alternative
still affects authored feasibility. Partial alternative membership cannot prove a
maximum or an each-alternative budget.

A prospective representation is `AllocationCost { node, pool, points }` and
`AllocationBudget { id, pools, usage, capacity_stat }`, with usage operations such as
`SharedPlusMaximumScoped` and `EachScope`. The capacity is a nonnegative Integer
PlayerActorStat produced by the owned effect graph from level, explicit rewards and
other known contributors. Saved source totals, estimated quest progress and a build's
observed spent-point count cannot supply that capacity. These types and the legality
executor remain planned; scope eligibility alone makes no legality claim.

Allocation access rules should express conjunctions of prerequisites and explicit
provider-granted exceptions. Shared paths may use shared nodes; a scoped branch may
use shared nodes and its own scope. An ordinary graph-disconnected node can be legal
through an item-granted radius, alternate starting point or another declared access
rule. Conversely, adjacency alone does not prove an ascendancy or unlock prerequisite.
Radius eligibility should be compiled from owned geometry or exact finite membership
with source evidence, then injected. A jewel does not automatically allocate every
eligible node or waive their costs. Static implicit roots are not a workaround for
unimplemented dynamic grant activation.

## Offline conversion and saved-build import

The offline compiler reads pinned finite source data, classifies semantic rows and
appends owned identities through the single registry ledger. It emits definition
schemas, known typed rules, legality data, and separate source mappings/provenance.
It never emits Lua source ASTs, callbacks or UI state to be replayed by native code.
Schema completeness and numerical rule completeness remain separate.

Catalog publication uses the same validated successor finalizer as carried package
publication. It can add new selectors and conflict-checked source pins while preserving
prior descriptors and registry history. Refining an existing Unmapped definition needs
an explicit refinement policy; appending a second identity is not a refresh strategy.
Native loaders do not fix stale bindings. Source mapping/role policies are offline
artifacts, not required native package dependencies.
A typed tree-import artifact binds to the final schema/mapping/registry and the rebound
base-normalization digest. The base policy does not refer back to the tree artifact.
Validate the tree content before one manifest-covered publication; subsequent transitions
must carry previously declared artifacts. Do not replace prior-bound policies with new
successor IDs before their validation or permit arbitrary unvalidated extra files.

Saved-build conversion maps each source token to a physical allocation, an implicit
root marker, an attached option, or an unresolved fact. Root markers verify character
selection. Attached options verify their parent and exact choice slot. Attribute
lists, weapon overlays, class identifiers and tree version must be structurally and
semantically consistent. Conflicting, duplicate, unknown and namespace-mismatched
input remains diagnosable; substring recovery and last-write-wins are not normalization.
Historical presets stay independent from the selected preset.

The five protected originals provide 16 saved specifications and 613 selected node
tokens, including implicit roots and attached choices. All 141 selected attribute nodes
have explicit choices. Two selected trees have disjoint 24+24 weapon overlays already
in the main node list. These exercise shared identity and scoped membership rather than
justify a new skill-specific path. Original01 additionally requires item-bound radius
access; original03 exercises a conjunction of unlock prerequisites; original05 exercises
a class-dependent stat view and a saved level independent of its smaller selected tree.

The first production conversion implements that structural split. The finite exporter
produces a bounded catalog; the Rust compiler appends to the existing ledger and the
shared finalizer publishes one bound `tree-normalization.json`. Subsequent CLI reruns
validate the declared prior artifact and compare canonical content before publishing.
The importer records implicit roots as origin links, places parented choices on their
physical allocation, and preserves real loadout scopes across all saved specifications.
Its sidecar version 9 includes the exact tree-policy digest. The absent-policy route
remains explicitly unresolved; it does not invent topology. All actual allocation access
is still Pending until injected legality data and the executor establish it.

## Acceptance and retirement

Prove root deduplication, changed class selection, partial membership, paid-root rejection,
all attribute options, parent/option isolation, malformed overlays, class-view selection,
coupled budgets and explicit access exceptions. Use synthetic perturbations alongside
all five fixed original query sets; keep all 110 reference rows. No structural conversion
or test count certifies a complete build.

The runtime consumes only the owned build and compiled package. The optional PoB adapter
compares independently realized results and reports unsupported mechanics. CLI, future
GUI and search use the same domain services. Retire the legacy tree/profile consumer and
its source/UI dependencies when its real replacement and numerical invariants pass;
do not retain a second catalog or a general source interpreter as a compatibility layer.

## Injected allocation package checkpoint

Core now defines version-1 `AllocationRulesInput`, including node/pool integer costs,
budget membership/closure, `Total`, `SharedPlusMaximumScoped` and `EachScope` usage, and
explicit final-stat capacity bindings. Data validates bounded canonical packages against
the exact owned schema. It neither executes the usage formulas nor certifies capacity or
access. Missing cost rows are unknown; explicit zero is a supplied game rule.

A cost row asserts an invariant cost across the admitted route, choices and scope.
Conditional cost modifiers require a modeled rule and corresponding coverage; a static
row must not silently suppress their effect. Granted access does not waive a node's cost.
A uniform capacity must agree across all authored weapon alternatives; per-scope capacity
uses that alternative's final Integer PlayerActor stat. Capacity cannot be reconstructed
from spent points, configured level or a source UI's endgame maximum.

The [offline component](../data/owned/poe2/3887ae68/allocations/README.md) emits 4,503 exact
costs and three budget declarations using the current owned IDs. Its acquired capacities
are Unmapped and budget membership is Partial. Source policy/provenance remain separate
from `rules.json`; the native loader needs only that artifact and its owned schema. The
native usage/capacity executor and access resolution remain next. This does not change
the complete-request or contributor-closure gates, finalize any Pending allocation,
or establish original-build legality.

## Ordinary attribute effects

The offline Rust `compile-owned-attributes` command converts an injected finite catalog
and reviewed lane policy into ordinary Choice/Compare/Contribute programs. It preserves
the physical passive, exact allocation-owned choice slot, selected option, pool and
adjacency. The [production authoring inputs](../data/owned/poe2/3887ae68/attributes/README.md)
cover 293 ordinary attribute nodes. Names, values and target stat IDs are supplied data;
the runtime uses existing typed operations without attribute-specific dispatch.

Publishing this conversion uses a narrow `PassiveDeclarationRefinement`: exact before
and after schema identities plus the affected node IDs. The finalizer permits only
Partial-to-Complete port closure with identical members; every other old declaration and
registry entry remains exact. Unknown nodes, duplicate entries, unused assertions,
changed topology or changed slots reject. Rule membership is validated separately and
is never inferred from port closure. A prior nonempty conflicting program cannot be
replaced by invoking the attribute converter again.

The shared CLI bundle loader retains manifest hashes, endpoint checks and all carried
import policies. Unchanged reruns preserve semantic artifact identities and allocate
nothing. Allocation access remains Pending in imported builds; this conversion does not
prove affordability, grant activation, class-view semantics or complete attributes.
The original query set and whole-request/contributor closure gates are unchanged.
