# Native configuration preparation and lifecycle

Status: R2b authored loader prefix implemented and paired with the original runtime;
activation callbacks and effective configuration are not complete. The
[implementation record](implementation.md) tracks validation and the resume point.

## Ownership and execution boundary

Configuration is per-build executable state. Versioned definitions describe available
settings, defaults, compatibility rewrites and source programs. Imported occurrences
preserve what the caller authored. A selected view identifies the requested set. Native
preparation combines these through a source/view/data-owned token; a serialized report
cannot manufacture that token or an effective evaluation plan.

The immutable definition snapshot is shared by workers. Each preparation owns its input,
placeholder, control and modifier state, including table aliases and callback writes.
A prepared calculation consumes bound, typed state without opening files, parsing XML,
calling Lua or querying a database in the evaluation loop. UI autocomplete uses a separate
rebuildable index over those definitions, as described in [definition storage](definition-storage.md).

## Original lifecycle to preserve

PoB constructs Config before Items, Skills and Calcs, and performs a configuration modifier
pass before importing the build's sections. Later section loads follow the document's
order, with passive-tree loading deferred. Configuration activation runs UpdateControls,
BuildModList and loadout synchronization; it can write inputs and placeholders that later
callbacks read. Constructor state, saved values and effective scenario are distinct stages.

The native loader prepares the first reached configuration section through set creation,
ordered authored writes, legacy migrations and selected-set binding. Its continuation is
before UpdateControls. Later configuration sections remain unexecuted because an earlier
activation can affect persistent controls and derived state. With no configuration section,
the prefix exposes constructor defaults and the initial BuildModList continuation.

The prefix does not synthesize the preceding root lifecycle. In particular, previously
derived enemy level, modifier lists and control state are not reconstructed by copying the
newly loaded maps. Independent original-runtime traces establish the context needed to
resume the full lifecycle. Those observations are test evidence, never runtime input.

Definitions supply defaults for every created set. Each creation starts fresh; it does
not copy placeholders that an earlier callback pass modified. Preserve repeated definitions,
set replacement, numeric-key behavior, order holes, malformed scalar precedence, ignored
local error returns, and partial source failures. A strict authoring inspector may reject
inputs that the source loader consumes; it is not the authoritative executable input model.
Use the existing source-occurrence and XML-content seam rather than a second XML parser.

## Effective settings and shared programs

The completed stage must execute callbacks in source variable order. Check, count,
zero-allowing count, integer, float, list and text widgets have different dispatch rules.
Nil, false, zero and empty strings retain their source meanings. Placeholder fallback is
part of dispatch, and enemy-level selection preserves source comparison and clamping order.
Settings without an apply callback can still be read directly by later calculations.

Controls need a portable state representation for selected list values, enabled state,
text and placeholder notifications. A non-notifying UI update differs from a notifying
placeholder write. Native evaluation does not need rendered widgets, but it must retain
these observable data effects. Unknown control behavior remains an explicit dependency.

Use the [shared program owner and session contract](shared-source-programs.md).
Reuse the shared typed-program verifier and executor for branches, locals, arithmetic,
loops, table construction and calls. Extend their source ownership beyond the current
modifier-parser catalog before admitting configuration programs. The common boundary must:

- Bind callback identities, captured values, definition roots and source spans to one
  immutable owner; unrelated catalogs cannot exchange numeric callback or table IDs.
- Separate structural program validity from permission to execute a domain effect. An
  extracted callback span or successfully lowered body does not prove calculation parity.
- Keep immutable definition/capture graphs read-only. Mutations require explicit per-build
  state ownership, with aliases preserved across sequential callback invocations.
- Resolve modifier-list, control and parser-service methods through typed, source-bound
  operations. Avoid configuration-name dispatch recipes and a second interpreter.
- Bound work, strings, tables, call depth and pattern matching cumulatively across a
  preparation. Failures retain the reached prefix and identify the pending consumer.

Class-backed modifier state follows the shared class/proxy protocol and uses opaque
per-session values. A source-observed partial class projection must list unrepresented
fields and preserve actual constructor/helper captures; plain result export cannot erase
metatable behavior. The optional PoB tooling now provides constructed-class/closure
observation as a reusable API; configuration effect admission remains a separate step.
Class capture receives explicit source aliases and requested methods, preserves complete
ordinary captured tables, and never runs a constructor itself. Callers establish the actual
source construction and cache state before observation.

The initial real paths require modifier insertion, common arithmetic and scalar operations,
control notifications and enemy/boss definitions. Broader source branches also require a
parser-service call and iteration with a justified order policy. Add these when complete
source consumers establish their semantics; do not enable a callback with missing branches.
Existing parser APIs retain their stricter borrowed-argument behavior through an adapter.

Custom modifier blocks use the same modifier parser and source attribution as other
consumers. Preserve enabled flags, the first consumed XML content slot, legacy migration,
blank blocks and parser leftovers. A child element in the source content array is not an
invented text string. Unsupported parsing must not silently remove an active modifier.

## Injected compatibility policy

Game-specific legacy input rewrites and default labels are generated from authenticated
ConfigTab source. Native code implements bounded rewrite operations; injected data supplies
the keys, patterns, replacements and their order. XML structural names and source-language
control flow belong to the reader. The configuration metadata catalog remains independently
useful for inspection, but its callback metadata is not executable configuration capability.

The existing narrow numerical adapters still consume explicit raw scenario inputs. When
injected loading policy changes those values or the separately parsed modifier blocks,
reject that adapter instead of calculating from stale XML. Apply the same guard during fixed-scenario candidate preparation. A passed
prefix guard does not establish general callback, build-legality or complete PoB parity.

## Verification and next integration

Compare all five original builds at the same source-method boundary using observation
wrappers around complete original methods. Preserve source bytes and original selections.
Exercise cold and reused runtimes, defaults, malformed/duplicate cases and injected policy
changes. Compare identities and state transitions as well as scalar values; sorting values
cannot prove lifecycle or alias parity.

After the loader-prefix checkpoint, advance the initial default modifier pass and saved
set activation on Twister and Skeletal Sniper together. Retain all five originals as
structural cases. Then continue root, item/passive/provider and actor/action preparation.
Neither an authored-prefix comparison nor a callback count completes the R3/R5 whole-build
numerical gates in the [real-build rollout](real-build-rollout.md).

## Next environment and notification dependency gates

The real configuration callbacks require an explicit distinction between a missing field
and an unrepresented field. `BuildModList` branches on `varData.apply`; boss presets branch
on optional definition fields. Projecting either graph by dropping unknown values would
silently skip effects. The implemented [environment and session coverage seam](shared-source-programs.md)
therefore makes known absence and unavailable state distinct at the access boundary,
without adding an artificial Lua value. Complete ordinary tables continue to be captured
as complete tables. A deliberately partial graph must carry explicit coverage, including
iteration/length and mutable-write rules.

The original `UpdateLevel` component now runs against an authenticated read-only environment
and actual live configuration/build projections. All five originals exercise initial/saved
direct and boss-callback calls, preserving selected-set/input aliases, placeholder precedence,
the configuration/build cycle and source clamping order. Continuing writes, source errors
and branch-dependent missing producers also match or stop at the explicit unavailable
boundary. The test-only producer does not complete earlier lifecycle stages in production.

The bounded live-control component now executes complete original `EditControl.SetPlaceholder`
and its numeric `changeFunc` through shared Rust programs. The ConfigTab state, closure
instances and shared capture cells belong to private sessions; immutable variable definitions
remain shared references. Capture slots retain live cell identity even when their current
value is immutable. Source observation authenticates this partition rather than baking a
live ConfigTab snapshot into shared definitions.

Pair notifying and non-notifying writes with later reads, selected-set changes, multiple
controls sharing ConfigTab, unavailable short-circuits and failure-prefix effects. The
five-build component oracle covers these cases and restores source state. The subsequent
inherited-method gate binds actual class instances and compares continuing native callback
sequences at original source entries/exits. It does not admit enclosing constructor execution
or general native closure creation. Source fault probes compare the reached state prefix;
an unavailable native dependency remains distinct from the deliberately induced source
error. The non-placeholder branch retains its input write before unavailable `AddUndoState`;
`BuildModList` is still unentered on that path.

The inherited callback sequence advances through `enemySizePreset`, `enemyIsBoss` and
`presetBossSkills`, retaining control state, modifier rows and source identities across
later calls. Complete source `Common.round` executes through authenticated floor and
exponentiation; its branches retain source rounding, coercion and failure behavior.
Generic iteration now calls exact source-bound iterator/state/control values. Complete
immutable boss definitions carry their actual observed traversal order; mutable and
partial collections remain explicit dependencies.

Actual `DropDownControl.SelByValue` executes with its class ancestry and shared list
identity. The component gate covers every injected preset and boss selection, including
non-None, base/Uber, earlier-Uber, multiple damage types, additional flag/numeric stats
and reset-to-None behavior. Each positive case uses a private session over one shared
compiled owner. Probes compare selection/enabled state, numeric notifications, input and
placeholder aliases, ordered modifier rows and reached state before/after source errors.
They restore projected source state and aliases, without claiming to restore Lua's
internal hash layout. UI selection callbacks and constructors are not admitted by this gate.

The current first frontier is the quest callback's escaped `string.gmatch` iterator.
Generic lowering succeeds, but the native intrinsic cannot yet return a callable iterator
with private progress state. The continuing callback gate rejects it before state writes.
Add bounded heap-resident iterator identity and progress through the shared call protocol;
prove aliasing, repeated calls, deferred pattern errors and cumulative allocation/work
before advancing quest processing. Existing direct pattern loops keep their behavior.
The test still observes the original enclosing loop and executes individual callbacks;
it does not yet execute complete native activation.

Custom-modifier and quest callbacks additionally require explicit parser-service result
ownership. Returned modifiers must be writable where original `setSource` mutates them;
callback IDs from different owners cannot be passed through by number. Continue complete
initial/default and saved activation only after these dependencies are bound. A compiled
callback inventory or a compared prefix of independent modifier callbacks does not complete
that activation stage.
