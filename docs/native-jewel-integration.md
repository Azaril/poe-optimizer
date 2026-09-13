# Native jewel and selected-tree integration

Status: planned next integration at the existing injected-data/item/tree seams. This is a
source-informed design, not implemented capability or parity evidence. It does not select an
execution model in the [A1-A4 investigation](rule-execution-model-investigation.md).

## Separate item data from tree application

Pinned `Item.lua` has three distinct operations. `ParseRaw` (816–830) records a radius label
and either looks it up in the current radius definitions or defers a Variable label. The local
jewel branch (2616–2667) builds jewel modifier data without reading a passive tree. After
assembly, `ParseRaw` (1797–1802) applies a deferred radius index and then a time-lost override.
The native loader currently stops all jewel radius headers before these operations. Replacing
that stop should not require an already calculated actor or complete spatial tree.

The radius definitions are version dependent. `Data.lua:638–668` selects the greatest available
major/minor radius version no later than the requested tree version, derives squared radii using
the injected distance multiplier, and sets a maximum radius. `TreeTab:SetActiveSpec` installs
that state for its selected spec. `ItemLoadingData.jewel_radii` already retains all raw source
versions; it is not a resolved radius context or a computed spatial graph.

Spatial use is a later stage. `PassiveTree.lua:349–406` constructs node sets around sockets and
keystones, with different eligibility predicates and inclusive inner/outer boundaries.
`PassiveSpec` consumes those sets for allocation/connectivity, including Intuitive Leap-like
and From Nothing effects. `CalcSetup` applies jewel effects and limits using selected sockets,
item sets and actor state. Completing local jewel data alone cannot retire these consumers.

## Context and ownership

Introduce an immutable resolved radius context derived from injected definitions and an
explicit tree-version selection. Its evidence records requested tree version, selected radius
version, distance multiplier, data identity and the source/selection provenance that chose it.
Keep these concepts separate from a passive-spec selection and from a fully prepared tree.
An implementation name such as `ResolvedJewelRadiusContext` is provisional; this document
specifies responsibilities, not a new public API already available to callers.

The native build coordinator should construct the context once for a validated build/view/data
owner and pass it to item loading. Reuse shared definitions across workers, while retaining
item-local scratch. Do not read process globals or choose the package's latest tree silently.
An explicit standalone item API can accept a validated context without inventing an authored
passive spec. A missing required context remains a named preparation dependency.

Pinned `Build.lua:519–589` resets radius context to `latestTreeVersion`, constructs default
components, then defers every saved Tree/legacy Spec section until after ordinary sections,
including Items. Thus the final selected spec is not the context of earlier Item radius parsing.
`Main:LoadTree` changes radius state even when its tree is cached; reused hosts also pass through
the Build reset. Bind the context to the actual item-call lifecycle event, keeping requested
version distinct from the resolved radius version. Startup/latest provenance must be explicit,
not a silent fallback. The final selected view remains an artifact owner, not proof of earlier
ambient state.

Current source definitions contain only radius version `0_1`, while supported requested tree
versions span `0_1`–`0_5`. Equal final values across existing builds can therefore hide incorrect
ordering. Before import-parity claims, record exact original function events/arguments and
item/XML occurrence identities for fresh/reused imports, reordered/repeated ordinary and
Tree/Spec sections, and failures. Compare observed and uninstrumented controls. A separate
injected multi-version fixture tests the native seam but does not replace source lifecycle proof.
The source review is retained in `runs/r2aa-weapon-local-01/jewel-context-lifecycle-review.md`.

A later tree switch may invalidate spatial/actor caches without replaying Item headers. Retain
previously parsed radius fields unless the source actually reparses the item; a subsequent
reparse consumes its then-current context. Do not retroactively rewrite item data merely because
the active spec changes. Complete indirect loader/control effects remain a proof gate.

Preserve source version parsing, no-matching-version errors, label extraction, missing labels,
Variable deferral, and retained state across reparses. Do not substitute lexicographic ordering
for numeric version comparison. After selecting numeric major/minor values, the source
reconstructs a canonical version key for lookup; preserve that lookup and its failure when only
a noncanonical spelling exists. Duplicate radius labels require represented source selection
order or an explicit unsupported boundary; a deterministic Rust map order is not evidence of
original traversal. Preserve the original maximum-radius and squared-radius arithmetic before
proposing an equivalent simplification.

## Local jewel producer and downstream consumers

The local producer consumes the existing ordered modifier graph. Inject names, selectors,
cluster correction rules and bounds through typed policies. Preserve Grand Spectrum modifier
aliasing, ordered JewelFunc/JewelData lists, last alternate class start, From Nothing maps,
cluster notable/added-mod lists, corrections, clamping and validity result semantics. Empty,
missing and retained fields differ; fresh `jewelData` at assembly entry does not imply every
header field resets at ParseRaw entry.

Opaque callbacks must not disappear during projection. Retaining a callback reference does not
authorize its execution; require validated definition identity and a supported consumer, or
report the reached dependency. Adding a general runtime feature remains subject to A1/A2 review.
Local assembly completion, inventory registration, equipment activation and actor calculation
remain separate gates.

Spatial preparation then combines an owned selected tree, radius context and socket/keystone
identity. Cache pure geometry by all relevant definition identities and spatial inputs. Keep
allocation-dependent reachability, jewel limits, transformed nodes, item/weapon-set state and
actor effects outside that geometry cache. Tree or radius changes must not reuse stale results.
Cluster-generated topology and callbacks require their own complete source comparisons; they
must not be inferred from ordinary radius membership. This also supplies a future input seam
for the opt-in [seeded-jewel search](seeded-jewel-search.md), without making seeds a dependency
of ordinary jewel preparation.

## Delivery and validation gates

1. Authenticate and resolve versioned radius definitions behind the injected context. Test
   numeric version fallback, malformed/no-match inputs, explicit multiplier changes, context
   owner rejection and isolation between simultaneous snapshots.
2. Implement complete reached local jewel production and header/final-radius sequencing.
   Compare whole declared owned graphs and failure prefixes through unchanged original methods,
   including nil overrides, callback boundaries, no-base reparses and changing contexts.
3. Integrate the context with the real import lifecycle and ordered inventories. Preserve all
   116 originals, including all 18 jewels and every unsupported record. Exercise inactive specs,
   reordered/repeated containers and selected-view changes; do not whitelist original IDs.
4. Implement socket/keystone geometry, allocation/connectivity changes and actor effects with
   boundary/eligibility, overlapping jewels, changed tree/radii and complete build comparisons.
   Keep the five-original full-native count explicit until their complete calculations pass.

The current weapon checkpoint can proceed independently of these gates. The
[living implementation record](implementation.md) owns sequencing and measured completion.
