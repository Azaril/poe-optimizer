# Native equipment and item-set integration

Status: R2ae wires constructor and ordered inventory/set loading through the existing
injected-data/native-preparation seams, up to an explicit activation continuation. Scoped
source validation, affected regressions, CLI checks, strict lint and portable compilation pass.
R2ac validates the complete slot-validity component with source-fed contexts; ordered set
activation and full build integration remain open.
No execution-model alternative is selected by this work. The
[implementation record](implementation.md) owns progress and evidence.

## R2ae implementation boundary and scoped source validation

The [native item coordinator](../crates/poe-optimizer-native/src/items.rs) and its
[set adapter](../crates/poe-optimizer-native/src/items/sets.rs) now consume the first Items
container in source order. Item text and ModRange operations use the existing owned item
loader; legacy Slot rows, ItemSet bodies, RuneSlot rows, SocketIdURL rows and
TradeSearchWeights use the new [item-set state](../crates/poe-optimizer-import/src/item_sets.rs).
A reached item failure or unavailable dependency stops later records; a valid later set cannot
bypass that producer. Unknown namespaces and repeated Items containers remain explicit
frontiers.

The constructor consumes injected base/swap/embedded slot relationships, rune-slot identities,
defaults and the complete passive-socket predicate projection. The latter is acquired from
the authenticated full tree, before the partial bundled class-tree projection, and its tree
version and full snapshot digest must match the package tree. Its requested startup version
must also match radius preparation; older resolved radius fallback data remains allowed. The
projection is not a list inferred from the five observed builds. The package is now schema 34 with item-assembly policy schema 6:
26,314,825 bytes, SHA-256
`2a6b63b64c237a0f31dcc65f7a301b7e18bbbd78cd09518a07a8bd12418384a6`.
Only the manifest and item-assembly section changed; the other 28 sections are unchanged.

The owned graph keeps the previous active-set object when Load replaces its lookup maps.
Missing set IDs allocate the next unused numeric key; duplicate numeric IDs replace lookup
winners while preserving distinct occurrences and order entries. The legacy fallback assigns
the new default as the active object before the pending switch. Numeric socket keys remain
distinct from string fields, and updates retain internal aliases. Trade weights use ordered,
first-match power-stat definitions, including absent keys and shared or distinct transform
identities. Owner-bound transform descriptors are retained without executing them; the graph's
transform-presence marker is only diagnostic. Rune names are stored without claiming dropdown
selection, choice ordering or rune-effect execution.

Finishing the represented prefix produces `AwaitingActivation` with the requested set and
deferred trailing flags. It does not execute SetActiveItemSet, PopulateSlots, SyncLoadouts or
ResetUndo, and it does not establish effective equipment. Diagnostic graph snapshots cannot
be imported as producer state. Item diagnostics, owned assembly and set construction share the
enclosing byte allowance; set graph operations also have a separate cumulative logical step
budget alongside the coordinator's instruction limit. These are work/construction charges,
not allocator or RSS measurements, and diagnostic snapshot copying is outside producer charges.

The [set-materialization target](../crates/poe-optimizer-pob/tests/item_set_materialization_parity.rs)
passed all eight tests. Across 16 fresh source hosts (five originals and 11 derived cases),
all 16 constructor comparisons and all 16 declared materialization comparisons matched.
Six cases reached the expected Source failure and matched the declared failure prefix.
The repeated-Items case records two successful original Load calls, but native comparison
ends at the first activation entry; the second successful Load is source-only evidence.

These are source-fed component comparisons, not native inventory-production or complete Load
parity. The declared graph excludes dropdown arrays/indices/effect data, slot parent/number/
weapon fields omitted by the observer, and exact trade-transform Function identity; rune
comparison covers selected names. Constructor comparison omits uninitialized source trade
storage, and error snapshots omit the previous-set root unavailable to that source witness.
Activation, PopulateSlots, SyncLoadouts, trailing flags and ResetUndo remain outside native
comparison. All 133 native regression tests and 59 affected data tests pass. Final binding/
item tests cover the last startup-tree guard; native-only CLI and WASM checks pass. The five
original public preparation reports retain all 116 items and 68 successful registrations;
three stop in item production and two await activation. No report claims complete evaluation.

Full native build parity remains 0/5. Its policy, acquisition, runtime and validation costs belong
in the existing [A1–A4 investigation](rule-execution-model-investigation.md); a future alternative
may use different representations while preserving injected definitions and required consumer
behavior. No architecture decision follows from wiring this prefix.

## Required behavior

Prepared item records, saved item-set assignments, live slot selections, and actor-effective
equipment are distinct states. They must remain distinct in the API and diagnostics. In
particular, a saved item ID may be absent, replaced by a later equal numeric ID, rejected by
slot rules, or inactive in the chosen weapon set. A successfully assembled item alone does
not establish participation in an actor's calculations.

The complete source loading boundary is `ItemsTab:Load` (1193–1320 at the current pin), including
its calls to `CreateItemSet`, `SetActiveItemSet`, `PopulateSlots`, `SyncLoadouts` and `ResetUndo`.
The native coordinator must preserve required outputs and failure prefixes across this boundary;
an observation before `SetActiveItemSet` proves only the earlier inventory prefix.

## Definitions and reusable validity

Inject slot identities and relationships, item types, tag names, rarity exclusions, patterns,
keystone flag names, node predicates and size thresholds through versioned definitions. Native
algorithms consume those definitions and caller build state. They contain no production list of
accepted build IDs, item IDs, or special answers for the supplied corpus.

Implement the complete `IsItemValidForSlot` method before using its result to clear or admit
selections. It includes passive-tree sockets and node overrides, flask routing, special arm/leg
slots, same-type slots, embedded jewel restrictions, and both weapon sets. Off-hand legality
depends on the selected main-hand item and explicit keystone state; startup defaults and later
calculated flags are different inputs. The original method's reached context determines which
one is appropriate. A final calculated flag snapshot is not evidence of earlier import state.

The kernel reads borrowed immutable item definitions/graphs and explicit context; it does not
run Lua, spawn processes, calculate a hidden actor, or mutate equipment. Preserve lazy lookups,
source errors, finite operand-valued returns and zero-return versus one-nil arity in differential
evidence. Downstream equipment consumers use a truthiness projection. Unknown dependencies and
invalid requests must not silently become an ordinary illegal-slot result.

Share definitions across Rayon workers; keep changing set, actor and tree inputs private or
immutably owned. Cache only when every relevant owner/revision is represented, including main-hand
selection, parent jewel host, tree/node overrides and keystone state. The calculation/backend seam
remains independent of the reference host and usable from a portable Rust/WASM caller.

## Ordered set and control state

Source `Load` resets set maps, set order and trade weights, while retaining the item map, item
order, live slot state and the previous active-set reference. `SetActiveItemSet` writes live
fields back into that previous set before reading the incoming set. Missing authored set IDs
allocate the next unused numeric key; duplicate numeric IDs replace lookup winners while source
occurrences and order remain represented.

The legacy no-ItemSet path assigns the new default as active before switching to it. Previous
and current set therefore alias intentionally, preserving legacy live-slot values. A stateless
projection of saved Slot rows cannot reproduce this behavior. Repeated Items sections and reused
hosts require the same explicit history. Keep source occurrence identity separate from numeric
lookup keys and the private constructed default-set identity.

Use the existing SelectedView selection identity and explicit override contract; do not create
an unrelated second selection policy. Reconcile independently resolved intent with actual loaded
state, retaining source failures and shadowed identities. An override cannot skip failed item
loading or other required producers merely because its requested set is syntactically present.

Rune selection uses `SelByValue`, which changes the dropdown selection without invoking its
selection callback. Unknown rune names retain the prior selection. Slot population validates
items and can replace selected IDs with zero, update child-jewel inactivity and save notes.
These changes affect later consumers and must be accounted for before claiming activation.
Loadout synchronization can invoke SetSel callbacks and reenter set activation. UI list ordering,
loadout synchronization and undo effects require a consumer audit: preserve
all behavior observed by build/export consumers, and document any presentation-only boundary
with source evidence rather than assuming UI methods have no semantic effects.

## Consumer boundary for the execution-model investigation

An original method observation and a shipping native API have different purposes. Observe
`ItemsTab:Load` through its actual return, including the final `ResetUndo`, to establish source
outcomes and failure ordering. The native calculation/export contract need not expose every
object that the observer retains. The current source audit identifies these distinctions;
they are proof obligations for the next implementation, not completed native integration.

| Source mechanism | Required consumer behavior | Boundary to investigate |
| --- | --- | --- |
| `SyncLoadouts` and dropdown `SetSel` callback | Preserve selected tree, item, skill and configuration sets, linked-set resolution, re-entry and reached failures. | Model the ordered domain transitions explicitly; do not classify the whole callback as presentation. Initial selection depends on previously loaded sections and dropdown state, even before saved Tree data loads. |
| `PopulateSlots` and item choices | Preserve selection clearing, child-jewel inactivity, notes and any later validity affected by earlier mutations. | Compare exact traversal separately from selected equipment results. Display order is not automatically a valid replacement for the source's `pairs` traversal; demonstrate independence or represent the dependency. |
| `RefreshBuildPlannerSets` | Preserve the exported spec/skill/item selection behavior when the caller uses these controls. | Synchronization resets the three export dropdown selections to their first entries. An explicit native export selection API can replace the controls only with a documented mapping; rendering does not justify dropping the selection effects. |
| `ResetUndo` / `CreateUndoState` | Preserve successful source completion and any relevant load failure. | The audited calculation and export consumers do not read undo buffers. A headless evaluator may omit GUI history storage once the admitted input domain proves these copies have no further effects; this does not promise arbitrary mutated Lua-object compatibility or native Undo/Redo. |
| Constructor slot and rune controls | Preserve slot relationships, selected rune identity, unknown-name retention and any ordering used by later selection. | Inject semantic definitions and ordered choices without constructing drawing, tooltip or layout objects. Reuse the existing rune catalog; derive passive sockets from the injected latest-tree definitions, never a fixed observed count. |

The source anchors are [ItemsTab](../vendor/path-of-building-poe2/src/Classes/ItemsTab.lua)
(`CreateUndoState`, 4477–4491; `RestoreUndoState`, 4494–4513),
[UndoHandler](../vendor/path-of-building-poe2/src/Classes/UndoHandler.lua) (`ResetUndo`, 23–27),
and [Build](../vendor/path-of-building-poe2/src/Modules/Build.lua) (`SyncLoadouts`, 637 onward;
`SetActiveLoadout`, 956–980) at the pinned reference revision. Restore/Undo/Redo are the identified
consumers of the history snapshot. [ImportTab](../vendor/path-of-building-poe2/src/Classes/ImportTab.lua)
(`RefreshBuildPlannerSets`, 494–531) supplies the export-control reset.
[BuildExportPoE2](../vendor/path-of-building-poe2/src/Modules/BuildExportPoE2.lua) (`GetLoadouts`, 264 onward)
suppresses that refresh when calling `SyncLoadouts(true)`, but still permits activation callbacks.
This audit does not establish that all UI-originated state
is irrelevant, nor does a source-only trace establish native equivalence.

Use this slice in A1/A2 to distinguish game-domain complexity, source-runtime compatibility and
reference-harness complexity. Count adapters and observation code in the total cost, while
keeping them outside production dependencies. No source-fed context or observed traversal may
become a required PoB runtime service for native candidate evaluation.

Rune choice ordering also needs a measured contract. The first fresh-host comparison retained
a swap between `Legacy of Wings of Caelyn` and `Legacy of Horns of Bynden`. Their source rune
sort keys tie (order 6868, requirement 65, group 3), with no name tie-break in `ItemsTab` 2240–2247.
This is consistent with the observed permutation, not proof of its cause or interchangeable
behavior: the two definitions have different modifiers. Preserve exact arrays as evidence and
compare selected names and duplicate-name counts separately; that narrower comparison cannot
establish effect equivalence or universal observation noninterference.

## Validation and completion gates

1. Authenticate and extract the complete validity policy; prove missing, malformed and changed
   definitions fail explicitly and caller-injected values drive behavior.
2. Compare the unchanged original validity method with native execution across all saved items,
   actual slot names and retained item sets, plus directed branch, lazy-error, tree, flag and
   main-hand/parent-jewel cases. Distinguish source-fed component inputs from native-produced
   preparation and record exact return arity/value and input ownership.
3. Integrate constructor definitions, ordered container/set loading and activation. Compare
   complete original method boundaries, current/previous set aliasing, legacy, repeated, reordered,
   failure and explicit-override histories. Use independent native inventory production; reference
   snapshots are validation inputs only.
4. Connect effective equipment and passives to actor/action preparation, re-evaluate keystone-
   dependent validity at the original lifecycle points, and validate changing optimizer candidates,
   worker isolation, fresh exports and complete original build outputs.

The five originals contain 116 Item nodes, 15 ItemSets, 330 saved Slot rows, 36 SocketIdURL rows,
five RuneSlot rows and four TradeSearchWeights containers. Preserve every record and the original
set order. These counts establish corpus scope, not successful native loading or full parity.
Use additional generated interaction/history cases where this corpus does not cover a branch.

Include policy, native kernel, context adapters and parity tooling in the
[A1–A4 execution-model comparison](rule-execution-model-investigation.md). Their necessary consumer
behavior does not prescribe the current schema, table graph or implementation for future models.
