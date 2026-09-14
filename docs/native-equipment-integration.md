# Native equipment and item-set integration

Status: R2ao adds native injected loadout display/lookup and directed original activation
observations; R2ap shares immutable parser compilation across native preparations. R2aq
integrates source-derived loadout metadata into the injected package; acquisition,
lookup, package, native-only CLI, lint and portable checks pass. R2ar adds bounded live
item-set reads and exact creation lineage; component, source, CLI, lint and portable
validation passes. Loader
base names remain preserved in owned item assembly. Public native preparation reaches `AwaitingSyncLoadouts` on original builds 2, 4 and 5; builds 1 and 3 still stop at
captured jewel dependencies. The [implementation record](implementation.md) owns current
validation and resume status. Full native build parity remains **0/5**. No alternative
execution model is selected; earlier checkpoints below retain their historical scope.

## Current ownership and continuation boundary

Slot validity borrows the registered `AssembledItem`, including its canonical `baseName`.
Hydration copies this dedicated loader field after replaying scalar projections on every
pass. Reparsing can replace it; a failed base lookup preserves the loader's previous base
name while clearing `base`. The adapter must not substitute the display name or read a
separate diagnostic state to fill a missing runtime field. Whole-item parity explicitly
observes this consumer field, and activation regressions require the owned production
inventory to reach the pre-Sync comparison on the three unblocked originals.

`AwaitingSyncLoadouts` is still a continuation, not completed equipment or build evaluation.
Synchronization joins tree, item, skill and configuration sets, resolves linked loadouts,
and can reenter selection callbacks. Its ordered effects, trailing loading flags and final
undo initialization must be accounted for before completing the Items lifecycle. The
[consumer audit](#consumer-boundary-for-the-execution-model-investigation) distinguishes
those domain effects from presentation-only storage without choosing a replacement model.

## Live owner integration after policy acquisition

The R2aq package supplies the lookup program with injected, source-derived metadata. Its
policy can be held directly in the program's immutable Arc; no new global default or
compiled owner is required for those operands. Complete production lookup still needs the
live domain readers. ItemSetState exposes bounded borrowed reads over its published
order/map roots, including the state retained after partial publication failures. Borrowed
rows prohibit mutation while being read. Separate opaque identities survive movement and
allowed restarts, so native preparation can bind each publication to its creation-time
SetOrigin. Numeric lookup winners do not identify duplicate saved occurrences or a detached
previous-active row. Reserve lineage capacity before source mutation and capture any
successful publication even when the enclosing source operation later fails. Tokens stay
private to runtime ownership and are never reconstructed from serialized diagnostics.

The shared Build preparation context must retain those private domain owners and its
source-order cursor across incomplete results. Current independent preparation reports
finish at different points; Skills does not retain its live set map, NativeTree represents
one selected allocation, and Config still has pending effects. The source constructs all
owners before loading non-tree XML sections and delays saved-tree loading. Reconstructing
a context from final projections would substitute future or unreached state. Build-owned
special-link maps must also come from synchronization, not diagnostic graphs. These are
implementation gates within the accepted shared instance/resolution/plan design.

## Synchronization source contract and next integration gate

R2an now has executable reference observations across all five originals. The optional
observer preserves the old Items.Load protocol and adds complete import-time Sync/lookup
roots plus separate direct post-import calls. Each observed first Items.Load still reaches
one returning SyncLoadouts with export refresh and no SetActiveLoadout in that scope. The
wider import history does reenter Sync through SetActiveLoadout; the scopes must remain
distinct. Final imported selections cannot stand in for the live pending-call owners:

| Original | Pre-Sync tree / item / skill / config | After-import selections | Existing dropdown index |
| --- | --- | --- | --- |
| 1, 3, 4 | 1 / 1 / 1 / 1 | 1 / 1 / 1 / 1 | 2: Default |
| 2 | 1 / 6 / 6 / 1 | 6 / 6 / 6 / 1 | 1: header |
| 5 | 1 / 2 / 4 / 1 | 3 / 2 / 4 / 1 | 1: header |

All five have activeLoadout=1 at this boundary, despite selecting different kinds of
dropdown row. Build 2 also has different item and skill IDs for matching titles. Build 5's
export item selection resets to the first saved ID, 2, independently of its skill selection.
These are observed source facts, not additional native coverage.

Keep imported occurrence identity, caller selection intent and live effective selections
separate. The shared preparation boundary must advance actual domain owners in source order;
combining independently completed diagnostic reports does not reconstruct that state.
An explicit selected instance must not silently retarget to another instance with the same
title. Resolve the application of caller overrides against executable duplicate/link cases
before choosing the continuation API. Missing reached domain operations remain dependencies.

The validated direct corpus contains 91 cases per control lane: 20 Sync calls, five
GetSpecList calls and 66 lookups across five originals. Three fresh hosts per original
produce 273 direct invocations: 258 returned packs and 15 source errors. Exact declared
pre/post graphs, return arity and aliases agree between control and observed hosts. Import
intermediate debug-history equality remains diagnostic, with per-host call/return ancestry
checks. Broader item/UI graph equality differs in 7/10 comparisons and is also diagnostic;
only the declared selection projection is required there. Real-source successful arities
are 1 or 4, so zero-return and one-nil outcomes remain unexercised. These observations are
not complete cross-domain state or native parity.

Observed consumer distinctions:

- All five imports initially reenter Sync through a Default SetActiveLoadout. Build 2 also
  reenters for Budget Endgame, ending at spec/item/skill 6 and dropdown 7; build 5 ends at
  spec 3/item 2/skill 4 with dropdown 1. No observed activation changes a domain through its
  SetActive* callback, so that transition family still needs directed cases.
- Direct post-import Sync(true), repeated true, nil and false do not reenter activation in
  these builds. Nil/false refresh exports, invoke all three export callbacks and ResetUndo;
  unchanged scalar selection IDs do not mean the call has no effects.
- Of 66 control-lane lookups, 61 return tables and five missing-link lookups raise the source
  error at Build.lua:925. Missing plain names retain singleton-derived IDs; no nil-result
  path is exercised. Build 5 has a partial result with only spec/config IDs, and build 2's
  Act 3 name resolves spec 3/item 4/skill 2/config 1. Do not zip domains by position.

Initialization also exposes retained old specs: TreeTab replaces specList and build.spec
before Sync clears loadoutsList. The reference observer now projects the old reached
PassiveSpec objects with their exact class/owner and shared alias memo, separately from
current equal-valued objects. This is a reference snapshot requirement, not a prescription
to retain every obsolete GUI object in production. A native omission needs its consumer
argument and evidence.

The next directed source gate must exercise changed-domain activation, caller overrides,
authored links and collisions, duplicate titles, sparse orders, stale link maps and version
changes. Preserve ordered failures and actual callback reentry. The source can resolve a link
before a later exact title; a normalized name join cannot be assumed equivalent. Current
source successes do not close those cases or the pending native continuation.

After that gate, implement the required ordered domain transitions behind the accepted
source/instance/definition/plan interfaces. Tree, item, skill and configuration activation
can change effective data and invoke further synchronization. Export refresh and the pending
Items.Load tail must also reach an accounted-for outcome. A proposed private continuation
must preserve these effects without imposing the reference observer's entire GUI graph on
the production API. No continuation representation or alternative execution model is selected.

For [A1-A4](rule-execution-model-investigation.md), distinguish actual game definitions from
PoB import grammar and presentation protocol. Link patterns, fallback titles and menu/version
labels are source-adapter operands; their downstream selection effects matter, but that does
not automatically require a shipping field for every literal. The current package lacks the
complete tree-version display map. Extracting that projection would not imply support for
older passive trees. Compare adapter cost and the consumer contract before extending schemas.

Review and command ledgers are under `runs/r2an-loadout-sync-01/`; final source reports are
in `runs/r2an-isolated-loadouts-05/` and the unchanged lifecycle regression reports in
`runs/r2an-isolated-lifecycle-05/`. The [implementation record](implementation.md) owns current
validation and integration status. These tests run only in the optional reference harness;
production definitions, native calculations and the package schema are unchanged.

## Native lookup component and directed activation checkpoint

R2ao supplies a native read-only component for GetSpecList and GetLoadoutByName. Its
standalone DATA policy injects fallback titles, latest/version-display data, decoration and
single-link syntax. IMPORT reads a borrowed live context with independent source singleton
proofs, one-based order reads, numeric winners and stale link maps. It never reconstructs
live owners from the final SelectedView or diagnostic snapshots. Returned IDs remain numeric
requests bound to the exact context/program; the later activation must resolve them through
the existing instance and owner rules. No second authored-instance identity system is added.

The source evaluation order is explicit: Skills/Items/Config singleton probes, complete
spec formatting, then Tree/Items/Skills/Config resolution. Each failed exact row comparison
tries link fallback immediately, even before a later exact name. Singleton fallbacks bypass
row/link access; absent results, partial tables and reached failures stay distinct. Shared
bounded Rust byte patterns provide lazy source-pattern behavior. This adds no generic
interpreter feature and no PoB/Lua dependency to the native component.

Four policy and 13 native tests pass; the complete unchanged original method bodies match
29 supplied-state cases, including three injected-version-global cases. These are component
differentials, not original import-closure or native full-build parity. The new modules pass
strict Clippy and WASM library checks. The standalone policy has not yet entered the bundled
package or native build coordinator. Source-backed acquisition must reconcile its latest
version with the existing authenticated radius/version data, and add the missing display map.

Directed post-import SetActiveLoadout evidence now covers 26 cases across the five originals
per control lane (78 calls across 15 hosts). All return zero values. Seven changes, in builds
2/5, execute Tree -> Items -> Skills with deferred synchronization, then Sync; dropdown
selection, lookup and nested SetActiveLoadout cause a second Sync without further domain
changes. Tree and Items both populate slots. Nil and no-spec partial requests call no domain
activation or Sync; repeated requests call Sync but no domain activation. Declared argument,
pre/post and final activation graphs match all ten control/observed comparisons. Broader
import exact graph equality differs in 8/10 and remains diagnostic.

This evidence closes the previously missing ordinary tree/item/skill transition examples.
It does not close changed configuration activation (the corpus has only singleton config
sets), partial requests retaining a spec ID, authored link/collision histories, caller
instance overrides, source-time state production or the complete native Sync/export/Load
tail. Those remain required before marking the pending Items lifecycle complete. Preserve
these effects in the shared Build coordinator; simply updating the four selected IDs is
insufficient. Validation and integration status belong to the living implementation record.

## R2ai preceding implementation boundary

Scalar doubled modifiers now use injected operands and fresh native result graphs, retaining
shared-table mutation as an explicit dependency. The four real rune strings have full-public-
parser graph comparisons, including the original raised-shield line's unparsed remainder.
Parser success here does not certify that the resulting effect can be calculated numerically.

All five public native-only inspections retain 116 records and 96 registrations (13, 34, 0, 21, 28).
Builds 1 and 3 stop at captured jewel factories. Builds 2, 4 and 5 advance to `rune_initial_order` with
special callback 1291: Legacy of Deidbell, helmet, ordinary row 1,
`Warcries Explode Corpses dealing 10% of their Life as Physical Damage`.
The report now records the actual provider request using escaped, bounded context, including
explicit truncation flags. Extra retained context is charged; diagnostic exhaustion preserves
the original dependency kind and emits an omission marker. No shared parser API was expanded.

These results do not complete rune preparation, public activation or a build evaluation.
Full native originals remain **0/5**. [Command receipts and frontiers](../runs/r2ai-doubled-forms-01/public-frontiers-02.json)
separate this current result from the R2ah component evidence below. The
[implementation record](implementation.md) owns validation and continuation status.

## R2ah preceding implementation boundary

The public `runtime-02` check passed 46 test executions: 19 native, 4 default-feature and
23 native-only. All five native-only CLI inspections succeeded with incomplete preparation
reports, retaining 116 records and 96 registrations across the original inputs
(13, 34, 0, 21, 28). Builds 1 and 3 still stop at captured jewel factories. Builds 2, 4 and 5
now reach `rune_initial_order` with
`ModParser pending shared dictionary mutation in doubled form: None`. The earlier callback333
dependency is resolved; these reports do not establish successful public activation.
The diagnostic comes from an unconditional guard for the `DOUBLED` form, so it does not
establish shared mutation on the failing invocation. The reports do not retain its actual
parser input. A [file-only catalog audit](../runs/r2ah-equipment-activation-01/frontier-analysis-by-engine.md)
finds four ordinary rune lines with scalar names; the source scalar-name path allocates
fresh tables. Admitting that path is a potential bounded follow-up requiring parity evidence,
not an execution-model decision.

The [activation transition](../crates/poe-optimizer-import/src/item_sets/activation.rs)
preserves the previous active-set object, copies live slot fields in source operation order,
resolves rune selections and populates slots only after a bounded dependency proof. Reached
dependencies remain explicit; successful population returns `AwaitingSyncLoadouts`.
Cross-domain loadout callbacks, trailing Load flags, ResetUndo and actor-effective equipment
are still separate work. Changed cluster selections and observable traversal dependencies
cannot be bypassed by choosing a native iteration order.

The [native context](../crates/poe-optimizer-native/src/items/activation.rs) retains the exact
compiled-data owner and uses an injected read projection of every node in the authenticated
startup tree, including the source-derived charm-socket flag. Dynamic effective-node changes
remain outside this startup context. The item inventory report uses schema 5 and retains
activation progress or the reached failure; diagnostic snapshots cannot construct live state.

[Rune-choice preparation](../crates/poe-optimizer-import/src/item_sets/rune_choices.rs)
parses ordinary rune lines through the supplied finite parser and applies source attribution
before slot filtering. Bonded lines contribute display text and grouping. Private handles
retain the selected record and its modifier values; unknown names preserve the previous
handle. A [bounded proof of the pinned sort](rune-headless-selection.md) certifies completion
and the unique first record without requiring a global strict weak order. Remaining native storage
order is private; ambiguous first records, duplicate eligible names and unproved comparison
safety remain explicit frontiers. Failed preparation is retained across resumes. Finite metadata
ingress does not reconstruct arbitrary source aliases or admit callbacks as effects. The R2ah
modifier-parser schema 8 added closed scalar `Gsub` expressions with literal patterns and
text or original `string.upper` replacements; this alone does not establish complete rune preparation.

The source comparison is deliberately scoped: exact source arrays remain evidence, while
the headless comparison uses retained set aliases, selected item IDs, choice-label
multiplicities, notes and child inactivity. Rune comparison covers selected names; it does not
establish source effect-record identity or array order. A substituted original parser used
after import is component evidence, not proof of original initialization/cache history or a
production dependency. The startup witness retains the module-local `runeModLines` table
and requires continuation of the exact original module after sorting; a C-return hook is only
optional diagnostic evidence.

The final activation target passes 13 tests after removing two duplicate helper test
registrations; `activation-05` retains the earlier 15-test execution. The final
[source-output-06 receipts](../runs/r2ah-equipment-activation-01/source-output-06/summary.json)
retain unchanged comparison outcomes. The ten component cases retain all five original
inputs plus five derived cases: the five originals and three derived cases reach
`AwaitingSyncLoadouts` with matching declared headless graphs and node writes. The other two
derived cases retain explicit population-order dependencies; they are not graph matches.
The component uses source-fed context and an explicitly substituted original parser, with
629 ordinary rune parser calls per preparation. It supplies no source traversal to native code
and establishes neither public inventory completion nor the complete Load/Sync/actor lifecycle.

The ten `activation-05` startup observations retain 595 rows and 5,377–6,117 actual comparator calls.
Completion is observed at the exact original module continuation at ItemsTab.lua:2286;
the optional C-return diagnostic is false. The component fixture alone permits 50,000,000
steps: build02 completes at 5,646,764 reported steps, above the previous 5,000,000-step
component limit. Production defaults remain unchanged. Reconciliation is retained in
`runs/r2ah-equipment-activation-01/activation-source-review-05.json`.

Construction and query charges are cumulative logical bytes/work, including failed
preparation and selection queries. They are not allocator/RSS or execution-time measurements;
parser internals retain their separate limits. Include policy, acquisition, adapters and
reference tooling in the existing [A1–A4 investigation](rule-execution-model-investigation.md).

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
projection is not a list inferred from the five observed builds. At the R2ae checkpoint,
the package was schema 34 with item-assembly policy schema 6:
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
