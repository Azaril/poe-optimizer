# Real-build API: concrete migration proposal

Status: **architecture direction accepted on 2026-09-10**. The owner prioritizes correct
structure over shortcuts that cause larger later refactors. This makes the
[general build proposal](general-build-input-proposal.md) concrete enough to review. It
defines the target boundary; delivered APIs and coverage remain in the implementation record. Follow the [rollout](real-build-rollout.md)
for delivery and the [implementation log](implementation.md) for the current resume point.

The immediate contract is an imported instance model and explicitly selected view shared
by all five originals. Effective actors, actions and modifier dependencies are produced
by native preparation stages as those stages become implemented. A source-only model must
not manufacture a complete effective graph before those producers exist.

## Delivered foundation

[R1a source ownership and identities](build-instances.md) now implement the initial
immutable import boundary. The compatibility decoder DTO remains `ImportedBuild`;
`build_instance::ImportedBuildInstance` owns exact source plus authored instance mappings.
The [selected-source view component](selected-views.md) now supplies independent choices,
typed overrides and source/data binding. The broader signatures below remain the target
contract: effective producers, complete loader lifecycle and native integration are not
implied by that component.

## Concrete counterexamples determine the boundary

| Observed input | Required representation |
| --- | --- |
| Sniper selects skill set ID4, item set ID2, passive spec position3 and config set ID1; skill IDs occur in order 2,3,4,5,6,1 | Independent, typed selection domains; source ID and source position are different concepts |
| Sniper's two ring slots both reference saved item26 | One saved item record, two slot-use occurrences and two receiving contexts; source references do not certify physical item availability |
| Twister has duplicate manual Spear Stab groups and manual/item-granted Spear Throw | Definition identity is separate from occurrence identity and provider identity |
| Twister's selected player action has runtime index9 but group index8; Sniper's player action index4 maps to group3 and its minion has a separate index1 | Imported group/gem positions and runtime actor/action indices must never be interchangeable selectors |
| Twister authors second-weapon-set enabled and retains grants from primary/swap slots | Preserve requested weapon state; native stages determine which contributions are effective |
| Crossbow contains Cast on Dodge/Tornado and an independent Tornado | Ordered entries plus provider/trigger relationships, with distinct action instances even when definitions match |
| Kelari's Deception is an unresolved authored label and separately a resolved owned action | Resolution applies to a specific occurrence and role; a matching display name cannot repair another reference |
| Crossbow and Whirling Assault have unavailable selected damage queries; Sniper has finite minion DPS and negative unreserved Spirit | Unknown/unavailable output, numerical parity and feasibility remain separate outcomes |

These observations reuse the frozen five-build corpus; they are not a fresh evaluator run.
The local review bundle `runs/real-build-r1-api-cases.json` records 11 cases, exact source
ranges, hashes and 20 selected damage measurement records. Source pin is
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. Values in this table are test evidence, never
production defaults or dispatch keys.

## Three identity layers

| Identity | Lifetime and purpose | Rules |
| --- | --- | --- |
| `SourceOccurrenceId` | Exact document snapshot plus typed occurrence path; links diagnostics and edits to original bytes | Keep raw external IDs as data. A new XML snapshot has new source identities. Ranges alone are not candidate identities. |
| `InstanceId` | Build lineage plus a stable local identifier; identifies a set, entry, item record or slot use through candidate edits | Assign once when importing/creating an instance; preserve through edits. Cloning an instance creates a new ID with an origin link. Equal definitions/names never merge instances. |
| Private compiled handle | Dense index owned by one admitted plan/view/generation | Never serialized as a public selector. Reject foreign plan/data/view handles even if their digests match. |

A newly imported independent document starts a new lineage. Materialization records an
explicit correspondence between candidate instances and reimported source occurrences;
export/reimport does not depend on byte offsets staying identical. Stable instance IDs
need no global registry and must not depend on labels, current vector order or thread order.

A generated instance has a bounded provenance path rooted in a concrete provider use,
followed by producer-definition and occurrence discriminators. For an equipped grant, the
root includes its slot-use instance rather than only the saved item record. Additional
effects and owned actions retain distinct occurrences. Provider removal invalidates its
generated descendants and selectors; it must not silently retarget a same-named survivor.
These provenance paths describe generation, not arbitrary cyclic dependency edges.

## Import and view contracts

The following Rust-shaped signatures describe the proposed responsibilities, not a checked-in
API or final wire schema. New fields/types receive their own explicit schema version before
persisted requests or candidates use them.

```rust
// core: portable values; no Lua or native plan handles
struct BuildInstance {
    lineage: BuildLineage,
    revision: BuildRevision,
    sets: SavedSets,                 // each domain retains its own source order
    items: Vec<ItemRecordInstance>,
    source_origins: OriginIndex,
}
struct ViewRequest {
    id: ViewId,                      // bounded caller label, not proof of identity
    skills: SelectionRequest<SkillSetId>,
    items: SelectionRequest<ItemSetId>,
    passives: SelectionRequest<PassiveSpecId>,
    configuration: SelectionRequest<ConfigSetId>,
    weapon_state: WeaponStateRequest,
}
struct SelectedView {
    id: ViewId,
    binding: ResolutionBinding,      // source/build + definitions + resolver semantics
    skills: SelectionEvidence<SkillSetId>,
    items: SelectionEvidence<ItemSetId>,
    passives: SelectionEvidence<PassiveSpecId>,
    configuration: SelectionEvidence<ConfigSetId>,
    weapon_state: WeaponStateEvidence,
}

// import: source ownership and source-format semantics
fn import_build(document: BuildDocument, limits: ImportLimits)
    -> Result<ImportedBuild, ImportError>;
fn resolve_view(build: &ImportedBuild, request: &ViewRequest,
                definitions: &BuildDefinitionCatalogs, limits: ResolveLimits)
    -> ViewResolution;

// native: internal lowering; the admitted plan is privately constructed
fn prepare_view(build: &ImportedBuild, view: &SelectedView,
                definitions: &NativeDefinitions, request: &CalculationRequest)
    -> PreparationOutcome; // Ready(plan) or Incomplete(report), never partial metrics
```

`ImportedBuild` owns the source buffer once (for example, `Arc<str>`) and span/index records,
not a self-referential structure of borrowed projections. Reuse existing root/skill/item/
configuration projections and identity lookup; do not introduce a second XML interpretation
or duplicate the delivered gem/effect catalog. The portable semantic model lives in core;
import owns the source buffer and format adapter. The exact split can use a wrapper rather
than copying entire documents per instance.

`ResolutionBinding` ties selection/identity evidence to the exact source/build revision,
definition catalogs and resolver semantics that produced it. Native preparation rejects
foreign/revised bindings or resolves them again; matching set IDs, view labels or package
digests do not authorize reuse of owner-bound objects. The view label identifies an explicit
request within a run, while the binding establishes which interpretation it describes.

A skill set contains ordered group instances; each group contains ordered entries and
per-entry authored values, definition references, source-grant hints and enablement inputs.
Primary/additional effects are definition-linked occurrences, not additional assumed gem
rows. A support assignment's authored presence is distinct from its eventual application.
An item set contains slot-use instances referencing saved item records. A passive spec
retains its class/ascendancy and separate allocation domains, including allocation providers;
ordinary, ascendancy and weapon-set point accounting must remain independent. Saved sets
remain intact even when inactive or unsupported.

`SelectionRequest` distinguishes preserving the saved request from an explicit stable
instance selection. `SelectionEvidence` retains the authored raw value, selector kind,
requested override, selected occurrence, and implemented precedence/fallback rule. An
unresolved/deferred choice is not a `SelectedView` ready for preparation. Do not use one
generic defaulting routine: PoB skill/item/config sets use IDs, while passive specs use
positions and a different original selection routine. Missing, malformed, duplicate and
out-of-range cases need original-source controls before claiming equivalent fallback behavior.

One evaluation uses one explicitly identified view initially. Preserve a named collection
seam for later explicit multi-view requests, without automatically coupling saved alternatives
or searching them all. Joint multi-view optimization needs its own sharing/usage semantics;
this does not block the imported model or the existing joint search dimensions within a view.

## Partial resolution is useful evidence, not admission

`ViewResolution` returns source/definition bindings plus classified diagnostics. It may
resolve selected sets and exact gem identities while leaving effect construction, effective
configuration, item grants, supports or actor producers deferred. A diagnostic includes
its instance/provider, source origin, stage/mechanic, dependency path and state:

- Resolved by a specific implemented rule.
- Unresolved or ambiguous source reference, retaining candidates without choosing one.
- Unsupported implemented boundary with a named missing mechanic.
- Deferred behind a producer that has not run; its unknown children are not enumerated as facts.
- Reference limitation or source error, distinct from native missing semantics.

Report every independently discoverable active prerequisite. Do not claim that a first-stop
list is the full calculation closure, and do not infer inactivity merely because a dependency
has no requested damage output. Original source observations can show that a relationship is
representable; only native producers establish native effective nodes and edges.

As R2 implements producers, an effective graph records player/owned actors, actions, providers,
support applications, weapon scope, reservations, buffs and triggers. Definition references
and typed operations remain injected data. Actor definition, owner and population model are
separate from display group or skill identity. This is a calculation actor representation,
not a simulation object per monster, projectile or summoned creature.

The graph is descriptive. Native stage ordering must reproduce tested source behavior;
mutually dependent grants/supports/conditions do not license an invented topological order
or generic fixed-point loop. A complete plan is admitted only after all active dependencies
needed for its declared outputs have implemented producers and consumers. A separately
requested diagnostic output subset records its limited scope; it is not full-build coverage.

## Metrics, backends and parallel execution

Keep the existing `CalculationBackend` and `EvaluationEngine` boundaries and their validated
result contract. Do not add a generic prepared-backend trait as part of R1. The native
adapter can translate an incomplete preparation report to its existing classified error,
while inspection exposes the detailed report separately. No blocked preparation becomes a
successful `EvaluationResult` with zeros or silently omitted measurements.

Version metric selectors when effective actor/action production needs them. Public targets
use semantic actor/action keys plus part/stat-set choices, resolved within an explicit view;
compiled dense indices stay private. Saved MAIN, CALCS display and requested measurement
selections remain distinct and are recorded. Existing `Player`/`SelectedMinion` aliases and
one-based options use an explicit compatibility adapter; ambiguity or a missing provider
is an error, never permission to select a different action.

DPS, average hit, population, uptime, trigger/resource sustainability and simultaneous action
use require their own implemented meanings. No sum or rotation is inferred from an action
list. Objectives and constraints remain caller-configurable, including multiple required
skills/items and actor-specific measurements. Numerical availability does not establish
feasibility, as the supplied Sniper's negative Spirit demonstrates.

Plans/catalogs are immutable and shared; Rayon workers own bounded scratch and candidate
state. Repeated native evaluations use dense bound references without Lua, XML parsing,
subprocesses or diagnostic serialization. Source-preserving materialization belongs at the
finalist boundary, followed by fresh native and optional PoB comparison. Measure preparation,
relationship rebuilding, worker memory, calculation and complete search separately before
claiming throughput for broad builds. The portable core must remain usable without Rayon
or an OS process model so a later WASM host can supply its own scheduling.

Real-path acceptance also follows the rollout's versioned metric/availability manifests,
changed-definition calculation control and fresh-versus-reused worker transition tests.
Those tests establish that injected definitions and current candidate/view state actually
control the result; representation checks and serial/Rayon agreement alone do not.

## Small migration slices and evidence

| Slice | Deliverable | Acceptance |
| --- | --- | --- |
| R1a | Owned import/source occurrence and stable instance mapping | All five originals retain every saved set, ordered duplicate and source range; two ring references remain two uses of one record. Unknown rows survive. Limits reject explicitly. |
| R1b | Independent selected views and exact identity bindings | Execute original selection/identity consumers for normal, duplicate/malformed and fallback cases. Compare all five originals; don't normalize ID and positional domains together. Preserve unresolved/deferred states. |
| R1c | Connect the outer model to native preparation | Existing Spark/Mace controls pass unchanged through the same import/view boundary. Real inputs reach named preparation frontiers, not one-set/group guards. Effective graph completeness remains R2/R3. |
| R2/R3 | Shared effective preparation and complete outputs, Twister and Sniper together | Native producers replace deferred frontiers; fresh whole-build reference comparisons cover damage/resources/defences and explicit bossing/mapping contexts. Neither fixture gets a new profile enum. |
| R4/R5 | General candidate edits, realization, parallel checks and independent holdouts | Ordered skill/effect/provider mutations and all existing joint dimensions use the same model; preserve earlier builds when new mechanics arrive. |

The current candidate schema and controlled materializer remain explicit legacy adapters
until their replacement is required by these slices. In R4, version the candidate contract
for ordered entries/effects and provider-aware edits; do not reinterpret its current unordered
support set. Existing 1..N locks and class/ascendancy search requirements remain in scope.

R1 is complete only when its executable source/model tests pass and old numerical controls
still calculate through the new outer boundary. It does **not** promise complete native actor
or action graphs for the five originals. R2/R3 must prove those producers and integrated
numbers. Documentation, source inventories and catalog counts alone cannot pass any gate.

The staged shared-model migration is accepted. Rust type spelling, ownership details and
bounded storage are implementation decisions within that direction. Validate them against
the contrasting supplied builds before freezing contracts; do not retain a narrow profile
assumption merely to minimize the immediate diff. Significant new product/design direction
changes still warrant discussion.
