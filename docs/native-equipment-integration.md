# Native equipment and item-set integration

Status: design for the existing injected-data/native-preparation seams. R2ac validates the
complete slot-validity component with source-fed contexts; ordered set activation and full build integration remain
open. No execution-model alternative is selected by this work. The
[implementation record](implementation.md) owns progress and evidence.

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
