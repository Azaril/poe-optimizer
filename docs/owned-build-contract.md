# D1: owned build and scenario contract

**Status: owned records, inventory and codec implemented, 2026-09-14; full D1 remains open.**
[Domain architecture](domain-architecture.md) controls the boundary and
[migration D1](architecture-migration.md) controls delivery. D1 establishes inputs,
identity, a codec and adapter normalization; it does not establish numerical coverage.
The source audit is in `runs/owned-build-contract-01/source-audit.md`.

## Implemented boundary and remaining work

`core::owned_definitions` implements typed owned IDs and bounded authored values.
`core::owned_build` implements raw `BuildInput`, `ScenarioInput`, `QueryInput` records,
private immutable `BuildSpec`, `ScenarioSpec`, `QuerySpec` wrappers and an
`OwnedEvaluationRequest`. Constructors validate structure; the version-3 JSON codec
roundtrips standalone documents (including inventory and projects) and combined requests. `check-owned-input` is the first
CLI consumer and can write canonical owned JSON without XML, PoB or a game package.

`core::owned_inventory` adds immutable, self-contained stock, canonical build/inventory
record unions and exact candidate-bound availability assignments. It reuses the same
item-record validation as BuildSpec. `core::owned_content` provides bounded, domain-separated
snapshot digests; content claims gain authority only when binding recomputes them.

`core::owned_project` now composes independent typed presets through the shared record
validator. `core::owned_binding` checks a combined request against an injected schema index.
The [composition/binding contract](owned-binding.md) details exact occurrence selection,
choice aliases, concrete required values and schema-versus-resolution status. Draft repair,
revisioned editing, legality, computability and the five-case adapter remain open. Numerical
evaluation still uses legacy inputs; these additions do not establish native rule coverage.

The envelope is `{ schema_version: 4, document: { kind, value } }`, where kind is `build`,
`project`, `inventory`, `scenario`, `query` or `request`. Required optional fields use explicit null; omission
infers no semantic default. Item level is an explicit optional intrinsic fact: a number
means supplied, null means unspecified, and omission rejects. Unspecified does not mean
zero, the equipment requirement, or the level of a granted skill. Versions 1 through 3
reject. Migration must preserve supplied values and establish the new modifier precedence
explicitly; it cannot infer it from record sorting. Semantic digest domains remain unchanged,
but adding or changing semantic fields changes content digests. Occurrence IDs remain stable
for the same supplying occurrence.
Unknown/duplicate fields reject. Defaults bound wire bytes to
8 MiB, collection entries to 16,384, total entries to 100,000 and provider paths to 64 steps;
callers can tighten these resource limits. These are not game-level caps. Unordered
occurrence/assignment collections canonicalize; ordered queries and grant paths retain
order. Each item has a required `modifier_order`: an exact permutation of that item's
modifier occurrence IDs. It is semantic precedence for order-sensitive operations, independent
of the canonical ID-sorted modifier record table. Duplicate, foreign, unissued or cross-item
IDs reject; membership edits must update and revalidate the permutation. Reordering storage
alone leaves identity unchanged; changing semantic precedence changes it. Operation stages
and program/effect order still belong to rules. No PoB category ordering or parser traversal
is built into this contract. Decoding establishes no prepared-plan reuse or stock authority.

## Separate values and ownership

Use portable core records with bounded constructors. Neither construction nor owned-format
decoding requires a game checkout, XML, a filesystem, a VM or a selected source document.

| Record | Concrete contents and contract |
| --- | --- |
| `BuildProject` | Host-owned lineage, revision and allocator watermark; item/gem records; independent typed character, equipment, allocation, skill and mechanic-choice presets; explicit saved combinations; saved scenarios/queries; optional display metadata. Inventories and import attachments are separate associated records. |
| `BuildSpec` | One self-contained character/progression, weapon loadouts and active loadout, referenced item/gem instances, equipment uses, allocations, authored skill uses, support assignments, payload links and mechanic choices. Every reference resolves within this value or names an owned game definition. No preset selection or `Saved` fallback remains. |
| `ScenarioSpec` | Game/version namespace, encounter/enemy definition and level, external environment/condition inputs, explicit usage/uptime policies. It contains no selected skill, equipment or derived character statistic. |
| `QuerySpec` | Ordered, caller-identified measurements: typed metric definition, actor/action target, and applicable part/mode/stat-set selectors. Scenario is supplied alongside it, so one query can be compared under several scenarios. |
| `InventorySnapshot` | Immutable, self-contained item-record table including unequipped stock, physical-copy identities, game/version namespace and explicit completeness policy. Candidate-specific equipment-use claims are separate. Search consumes this independently of numerical build meaning. |

`compose(project, VariantSelection, inventory?)` selects each preset by its own typed ID, supplies an
explicit active weapon loadout and returns a `BuildSpec`. It never pairs presets by equal
numbers, label or position. Save names and UI preferences outside semantic records. A direct
caller can construct `BuildSpec`, `ScenarioSpec` and `QuerySpec` without `BuildProject`.

## Definition IDs, instance IDs and the D2 seam

Reuse `BuildLineage`, `BuildRevision`, `InstanceId` and the allocator from
`core::build_identity`. Add domain wrappers where needed: `GemInstanceId`, `SkillUseId`,
`SupportAssignmentId`, `AllocationId`, `WeaponLoadoutId`, `InventoryItemId` and typed preset
IDs. Existing item-record and slot-use identities already express different occurrences.
Do not treat a transparent wire ID as evidence of collection membership.

Define `DefId<K> { namespace: GameVersionNamespace, key: OwnedDefinitionKey }` with distinct
Rust wrappers/wire fields for classes, ascendancies, rewards, item templates, modifiers,
gems, skills, passives, point pools, equipment slots, encounters, metrics and rule-defined
output/choice slots. Keys are bounded project-owned symbols, never display names, PoB keys,
source hashes or dense compiled indices. No enum variant names an individual skill.

D2 supplies an immutable **definition schema index** with typed lookups. Its small D1-facing
contract describes definition kind, legal references, authored parameter/choice slots,
output/grant slots, supported selector roles and metric units. The index contains no source
code or evaluator callbacks. A caller can inject a tiny authored index for D1 tests; the
owned codec can roundtrip IDs even when their package is unavailable.

The namespace establishes compatible game/version identity; exact `DataIdentity` content
and semantics identity bind definition validation and later resolution. A new balance
package may retain definition keys but must invalidate old plans. An explicit mapping
migrates renamed/removed keys. Existing legacy package sections are not required by this
interface. Do not create a second competing catalog or a hardcoded fallback ID registry.

`EditSameOccurrence` changes rolls, quality, level or other non-identity properties while
retaining the occurrence/use IDs. `ReplaceSupply` replaces an item/copy, gem, allocated node
or reward with another supplying occurrence and allocates a new provider use/occurrence ID,
even at the same destination or with the same definition. Changing a gem's definition is
replacement, not a level edit. Insertion/cloning also allocates new IDs. Composition and edit
APIs preserve a use ID only for the same supplying occurrence; they never retarget existing
queries or exact requirements to a replacement automatically.

Persist allocation watermarks including deletions. Hosts coordinate branches/ranges or use
a fresh lineage. Content equality, equal revisions and deserialized clone-origin links do
not confer ownership. Semantic edits advance revision and invalidate stale private plans;
public symbolic selectors survive ordinary property edits when their declaration still exists.
Presentation-only edits do not change semantic revision.

## Minimum semantic records

The following is proposed notation, not compilable Rust or an exhaustive game schema.
Lists have bounded size; entries with independent identity carry typed occurrence IDs.

```rust
BuildSpec {
    allocator: InstanceAllocatorState, revision, game_version,
    character: CharacterSpec,
    weapon_loadouts: Vec<WeaponLoadoutId>, active_weapon_loadout: WeaponLoadoutId,
    items: Vec<ItemRecord>, gems: Vec<GemInstance>,
    equipment: Vec<EquipmentUse>, allocations: Vec<Allocation>,
    skills: Vec<SkillUse>, supports: Vec<SupportAssignment>,
    payload_links: Vec<PayloadLink>, choices: Vec<MechanicChoice>,
}
CharacterSpec { class: ClassDefId, ascendancy: Option<AscendancyDefId>,
                level: u16, rewards: Vec<RewardSelection> }
ItemRecord { id: ItemRecordId, template: ItemTemplateDefId, item_level: Option<u16>,
             parameters: Vec<ParameterAssignment>, quality: Option<QualitySelection>,
             modifiers: Vec<RolledModifier> }
RolledModifier { id: ModifierInstanceId, definition: ModifierDefId,
                 rolls: Vec<ParameterAssignment> }
EquipmentUse { id: ItemSlotUseId, item: ItemRecordId,
               destination: EquipmentDestination, scope: LoadoutScope }
GemInstance { id: GemInstanceId, definition: GemDefId, level: u16,
              parameters: Vec<ParameterAssignment>, quality: Option<QualitySelection> }
SkillUse { id: SkillUseId, source: AuthoredSkillSource,
           enabled: bool, scope: LoadoutScope }
SupportAssignment { id: SupportAssignmentId, support: GemInstanceId,
                    target: SkillTarget, enabled: bool }
PayloadLink { id: PayloadLinkId, container: SkillUseId,
              payload: SkillUseId, role: SkillLinkRoleDefId }
Allocation { id: AllocationId, node: PassiveNodeDefId, pool: PointPoolDefId,
             scope: LoadoutScope, access: AllocationAccess,
             choices: Vec<ChoiceSelection> }
```

`EquipmentDestination` is a tagged semantic address: character slot, item socket identified
by its containing item-use and socket slot, or passive socket identified by its allocation
and socket slot. This covers ordinary equipment, runes and tree jewels without string slot
paths. Reject containment cycles. `LoadoutScope` is shared or an explicit nonempty set of
owned weapon-loadout IDs; the game's allowed loadout count is a definition rule.

Item/gem `parameters` describe intrinsic record state such as corruption, rarity, socket
capacity or variants through injected declarations. They cannot become fake modifiers or
use-local choices: two uses of one item record share these properties. The declaring owner
must equal the exact enclosing ItemTemplate/Gem definition. Modifier rolls, reward parameters
and usage-policy parameters likewise name the exact enclosing Modifier/Reward/UsagePolicy.
D2 checks whether each slot exists and permits that value. No inherited declaration fallback
is implied. Quality is an explicit optional, typed unit-bearing selection for items and gems;
its allowed kind, presence and bounds come from the package.

An `ItemRecord` is a concrete rolled specification occurrence, not proof of physical stock.
Several equipment uses can reference it and receive separate local effects/grants.

```rust
InventoryInput { allocator: InstanceAllocatorState, revision, game_version, items: Vec<ItemRecord>,
                    copies: Vec<InventoryItem>, completeness: Complete | Partial }
InventoryItem { id: InventoryItemId, item: ItemRecordId }
AvailabilityAssignments { build: BuildContentBinding, inventory: InventoryContentBinding,
                          uses: Vec<UseAvailability> }
UseAvailability { equipment_use: ItemSlotUseId,
                  claim: KnownCopy(InventoryItemId) | Unspecified }
```

The inventory owns every record referenced by its copies, including unequipped items.
`Complete` enumerates every copy available from this supplied inventory; absent copies are
unavailable within that stock domain. `Partial` leaves omitted stock unknown, and an absent
snapshot makes no stock claim. A seed need not contain inventory
alternatives: selecting an unequipped helmet resolves its record from the supplied inventory
and materializes a self-contained candidate `BuildSpec` with that selected record.

Composition/editing resolves the union of the supplied project, seed/build and inventory
record tables. Each table rejects duplicate IDs. Across tables, the same typed ID with
identical canonical semantic content denotes one record; conflicting content rejects before
selection, with no table precedence or last-write-wins rule. Reusing an underlying occurrence
ID across instance domains also rejects. Distinct IDs are never merged because their rolls
or definitions match. All tables use the same game/version namespace
and owned editing lineage. Adopting an independent inventory requires explicit ID remapping
and correspondence into that lineage; composition cannot silently rebase foreign IDs.
The resulting build contains all selected referenced records and requires no inventory or
project for numerical resolution. Unequipped stock remains outside its semantic digest.

Availability is an assignment for a particular candidate and immutable inventory snapshot,
not a property inherited from a slot or seed. Each binding includes lineage, revision and
canonical content digest; revision alone is insufficient. When availability is requested,
assignments cover every selected use exactly once. A known copy must exist in that snapshot
and reference the exact selected
item record; foreign, stale, duplicate-use or mismatched claims reject binding. Replacement
creates a new use and needs a new explicit claim or `Unspecified`. It never inherits the old
copy merely because the receiving slot stayed the same. Resolving `Unspecified` with evidence
for the same supplying occurrence is not replacement; switching known copies is. Reusing
one known physical copy in simultaneously active uses is a separate legality issue,
not a reason to erase computable
hypothetical measurements.

Implemented inventory operations are:

- `InventorySnapshot::new(input, limits)`, with borrowed `input`, `item`, `copy` and
  `validate_limits` accessors; construction canonicalizes unordered stock records.
- `union_build_inventory(build, inventory, limits) -> Result<ItemRecordUnion, InventoryError>`.
  The union carries the maximum input allocator watermark, and accepts unequal revisions.
- `build_content_binding` and `inventory_content_binding`, followed by
  `bind_availability(build, inventory, assignments, limits) -> Result<BoundAvailability, InventoryError>`.
  `BoundAvailability` is immutable; its accessor exposes the checked assignments.
- `BoundAvailability::authored_copy_overlaps(build, limits)` reports potential shared-copy
  use under the authored loadout scopes. This bounded advisory does not resolve nested
  socket/allocation/parent activation, establish full game legality or reject hypothetical
  bindings. A work-limit failure in that report does not invalidate structural binding.

Entry limits apply separately to each supplied document, the union item/occurrence table
and the availability assignments, rather than a cumulative budget across all nested
collections. Overlap analysis has a separate bounded work count. Snapshot digests include
all canonical input, revision and watermark, using `owned-build-v1` and
`owned-inventory-v1` domains. They are exact snapshot identities, not future numerical
plan digests: D3 must exclude unrelated stock and presentation from numerical cache keys.

An edit reads a coherent input union and emits the next self-contained build. If it changes
a shared stock record's rolls/level, rebinding claims requires coherently updated inventory
and project snapshots; the previous snapshots remain immutable. Container revisions may differ; conflicting content for the same record ID cannot
be combined in one union. A hypothetical numerical edit can use the build alone without
inventory claims. Sniper's shared saved ring record therefore becomes one record and two
uses with explicit unspecified copy claims and unresolved multiplicity diagnostics. It does
not invent owned copies. Identical rolled records on two known distinct copies are allowed.

`AuthoredSkillSource` is `Gem(GemInstanceId)` or a catalog-declared directly selectable
`SkillDefId`; it cannot forge an item/passive grant. Preserve duplicate authored uses and
multiple definition-declared outputs from one use. `SkillTarget` identifies one authored
use or a provider-bound generated skill key. A support's presence does not establish its
compatibility, capacity cost or application to every generated action. `PayloadLink` retains
an authored container/payload relationship; D2/D3 determine actual triggers and rates.
It distinguishes contained Tornado from an independently used Tornado.

`AllocationAccess` is ordinary allocation or a specific provider grant key. Pool identity
is explicit for ordinary, ascendancy and weapon-specific accounting; D2 checks node/pool
compatibility, earned budgets, connectivity and provider exceptions. D1 does not declare
every disconnected node invalid, make ascendancy points ordinary, or accept caller-written
point totals as authoritative. Reward selections name catalog-defined earned grants.

`MechanicChoice` and roll parameters are not arbitrary field maps. Each assignment names
a declared, typed definition slot and a permitted owner (character, item use, allocation,
skill/output or generated provider). The closed value forms are boolean, bounded integer,
finite unit-bearing quantity and typed option ID. The schema index supplies kind, unit,
bounds and allowed scope. No untyped JSON, Lua value, source field name, arbitrary stat
override or expression text substitutes for a missing semantic record. Examples include
an attribute-node option, output enablement and a rolled modifier magnitude. If this
vocabulary cannot express an imported mechanic, retain a typed unresolved mapping issue
and extend the reviewed domain schema; never hide it in a generic settings bag.

## Grants, actors and measurement selection

Selectors describe semantic ownership before dense plan indices exist:

```rust
ProviderRoot = Character | SkillUse(SkillUseId) | SupportAssignment(SupportAssignmentId)
             | EquipmentUse(ItemSlotUseId) | Allocation(AllocationId) | Reward(RewardSelectionId)
             | ItemModifier { equipment_use: ItemSlotUseId, modifier: ModifierInstanceId };
DeclaredSlot<S> { declaration: SlotOwnerDefId, slot: S }
ProviderKey { root: ProviderRoot, grant_path: Vec<DeclaredSlot<GrantSlotDefId>> }
ActorKey = Player | Owned { provider: ProviderKey, slot: DeclaredSlot<ActorSlotDefId> };
GeneratedSkillKey { provider: ProviderKey, slot: DeclaredSlot<SkillGrantSlotDefId> }
ActionKey { actor: ActorKey, provider: ProviderKey,
            output: DeclaredSlot<ActionOutputDefId> }
ActionSelection { action: ActionKey, part: ActionPartDefId,
                  mode: ActionModeDefId, stat_set: ActionStatSetDefId }
MetricRequest { id: QueryId, metric: MetricDefId,
                target: Actor(ActorKey) | Action(ActionSelection) }
QuerySpec { game_version, requests: Vec<MetricRequest> }
ScenarioSpec { game_version, enemy: EnemySpec,
               assumptions: Vec<ExternalAssumption>, usage: Vec<UsagePolicySelection> }
EnemySpec { encounter: EncounterDefId, level: u16 }
```

`SlotOwnerDefId` is a typed tagged reference to a declaring class, ascendancy, item template,
modifier, gem/skill, passive or reward definition, as permitted by the slot kind. Every slot
key includes that exact declaration and its namespace; binding checks that the actual provider
exposes it. Changing a Character-rooted class/ascendancy cannot satisfy an old key through
an equal slot name/number on another definition. D2 must give changed declaration/target
identity a new semantic slot identity rather than recycling an ordinal. Grant paths are
bounded, acyclic ownership paths, not callback stacks. Definitions supply stable
discriminators for repeated grant outputs; runtime array position is never a public
occurrence ID. Actor keys identify a calculation
actor/population owned by a summoning provider, not each simulated creature/projectile.
Player and minion requests share this model from D1 onward. Query IDs identify returned
request results, not build instances or admission authority.

These same typed selectors support future exact 1..N skill/item requirements. An authored
skill-use ID, generated-skill key, equipped-use ID and physical-copy ID select different
things; a definition-only filter must not satisfy a requirement for a particular instance.
Presence requirements and property-freezing locks belong to `OptimizationProblem`, not
`BuildSpec`. D1 preserves the addresses without implementing search policy.

Support-supplied actors have a SupportAssignment root, preserving supplying-support
occurrence independently of the target skill. An ItemModifier root pairs a modifier
occurrence with its receiving equipment use; the modifier must belong to that use's item
record. Equal-definition modifiers and two uses of one shared record therefore have
different provider addresses. D2 still checks exposed declarations and actual grant effects.

Removing/replacing a provider or disabling its effective loadout makes dependent selectors
unavailable; it cannot retarget an equal-named surviving skill. Same-occurrence numeric edits
retain the public key if the declared output remains effective; they still require compatible
private plan resolution against the changed content. A fresh plan never authorizes public
selector retargeting. D1 validates key structure and authored roots. Saved query/scenario selectors may retain
missing roots within the request lineage/watermark, while live wrong-domain or wrong-item
modifier roots reject. Build-internal references must exist. D2/D3 bind generated existence,
actor ownership and action compatibility;
D1 must not fabricate the resolved actor/action graph. Explicit part/mode/stat-set IDs
preserve primary/additional effects and average-hit versus DPS meaning. An adapter may
choose a default only when the catalog or an explained source rule establishes it.

Each D1 metric request has one explicit target. No automatic Full DPS sum or implicit
"selected minion" exists. D3 can admit explicit aggregation/usage policies with declared
units and timing semantics; unsupported policies remain unavailable. Existing classified
finite/nonfinite/unavailable measurement values are reusable independently of selectors.

`ScenarioSpec` references an encounter/enemy, level, external assumptions and explicit
usage-policy definitions with typed parameters. A scenario assumption slot must be declared
an external input: setting effective attributes, derived charges or computed conditions is
rejected. Optional enemy/incoming-damage overrides use declared typed fields/slots, not a
stat dictionary. All five imported bossing cases retain their explicit enemy-level-82
Pinnacle scenario; this is adapter evidence, never a default for other builds.

## Import, validation and codec

Implemented codec/availability signatures and planned remaining responsibilities:

```rust
decode_owned(bytes, limits) -> Result<OwnedDocument, CodecError>;
encode_owned(document, limits) -> Result<Bytes, CodecError>;
validate_structure(build, scenario, queries, limits) -> StructuralReport;
bind_definitions(index, build, scenario, queries) -> DefinitionReport;
import_pob(document, mapping_index, limits) -> ImportOutcome;
compose(project, typed_selection, inventory?) -> Result<BuildSpec, CompositionIssues>;
apply_build_edit(build, inventory?, edit) -> Result<BuildSpec, EditIssues>;
bind_availability(build, inventory, assignments, limits) -> Result<BoundAvailability, InventoryError>;
// D3, not part of D1 numerical completion:
resolve(rules, build, scenario, queries) -> ResolutionOutcome;
```

`ImportOutcome` contains a typed owned project draft, mapped semantic records, diagnostics
and an optional sidecar. Unmapped required values remain typed pending fields/records with
issue IDs and candidate mappings. They are not guessed definition IDs, empty effects or
successful `BuildSpec` values. Finalization requires the selected input's required semantic
facts; unrelated inactive drafts remain saved. Direct authors may use the same draft/finalize
workflow. A known definition with unsupported rules is different from an unknown definition.

Import-only sidecars hold original bytes, source hashes/occurrences, external IDs, mapping
decisions, warnings and export correspondence. They are neither a required field of the
owned build nor authoritative runtime state. Unknown semantic-affecting source fields create
blocking mapping/coverage obligations for affected inputs. A source history/default rule
must yield an explained explicit choice or remain unresolved. Export is an adapter operation
and reports nonrepresentable features; it does not authorize dropping them from the build.

| Check | Meaning; never substitute for the next check |
| --- | --- |
| Codec/structure | Supported schema, bounded values, unique identities, collision-free record unions, correct domains/lineage, membership and references; no containment/ownership cycles. Game-effect dependency cycles belong to D2/D3. |
| Definition binding | Referenced definitions exist in the injected namespace/package; parameters, choices and selectors have the declared types, units and owners. |
| Legality | Class/equipment/support requirements, pool budgets/connectivity, inventory availability and other game constraints. May be unknown or infeasible while calculation is possible. |
| Coverage/computability | All dependencies needed for requested outputs have supported semantics; an unavailable metric is not zero. D1 records prerequisites without claiming this check passes. |

Issues use typed semantic locations and stable codes; source locations attach only through
the sidecar. A negative unreserved resource can remain a finite value with a separate
infeasibility result. Unknown inventory stock alone need not block numerical coverage.

Use a separately versioned owned JSON envelope for project, standalone build and request
documents. Each persisted envelope includes the allocation watermark, including standalone
builds; request documents contain independent build, scenario and query values. Specify
UTF-8/text/count/depth limits and reject duplicate object members/instance IDs, unknown
semantic fields, unsupported versions, nonfinite inputs and invalid value encodings.
Definition-specific kind/unit compatibility remains definition binding. Preserve existing
fixed-width lowercase hexadecimal lineage/local/revision
encodings through browser roundtrips. Typed ID fields remain typed on decode.

Roundtrip preserves values, occurrence/provider correspondence, meaningful sequence order
and allocation watermarks. Sort unordered collections by their typed IDs; retain order for
explicit semantic sequences. Encode finite quantities with a deterministic decimal spelling
that roundtrips the accepted binary value. Normalize input negative zero to positive zero
at construction: authored quantities do not encode sign-of-zero state. Names, source
formatting and adapter sidecars are excluded from
semantic digests. Migrations are explicit version-to-version transformations; unknown
semantic fields are not silently ignored. Plans, scratch, caches, raw VM values and dense
handles are never persisted in this codec. D3 binds private plans to full semantic
build/scenario/query content and exact rules/evaluator identity, not merely a revision.

## Smallest complete D1 implementation slice

1. Add owned IDs/records, bounded construction and structural membership validation to
   portable core. Reuse the existing identity allocator; do not introduce a profile enum.
2. Agree the small typed D2 schema-index boundary and provide caller-authored test indexes.
   Keep structural roundtrip independent of package availability; report missing definitions.
3. Implement standalone build/request and project codecs, coherent record-union resolution,
   candidate-bound inventory claims, edit-versus-replace/clone behavior, independent preset
   composition and draft finalization. No numerical result or search algorithm API yet.
4. Add a PoB normalization adapter using the existing decoder/import occurrence evidence.
   Reuse source ownership only in its sidecar. Map all five selected combinations; retain
   inactive alternatives and typed unresolved cases. Do not replay native UI constructors.
5. Replace the old source-required shape for the new request path. Keep legacy entry points
   explicitly separate only until their named consumers are retired under D0/D5.

Required tests are contract tests, not evidence of whole-build calculation:

| Case | Required distinction |
| --- | --- |
| Direct authored input and codec | No XML/checkout/process; roundtrip IDs, values and watermarks; reject malformed/foreign/dangling IDs and unknown fields. |
| Independent presets | Sniper's skill/item/passive/scenario choices cannot select one another by matching positions/IDs. |
| Duplicate skills and grants | Equal definitions retain distinct uses; manual/item/tree providers and primary/additional outputs remain distinct. Same-item roll edits keep public selectors but invalidate stale plan bindings; replacing it with another copy granting the same skill allocates a new use and leaves old selectors unavailable. Changed class/ascendancy cannot reuse an old declared grant slot. |
| Items and inventory | A known unequipped helmet can become selected without preloading it into the seed. Equal repeated record content resolves once; conflicting content/duplicate IDs reject. Replacement requires a new candidate-bound copy claim, even at the same slot. Distinguish one record/two receiving uses, two physical copies, simultaneous known-copy reuse and unspecified availability. |
| Supports and triggers | Authored support does not imply application; contained/independent identical actions remain separate; disabled/output/loadout choices survive the codec. |
| Allocations | Ordinary/ascendancy/weapon pools and provider-granted access stay distinct; a disconnected node is not rejected by an invented universal rule. |
| Actor/query/scenario | Player and owned minion, action part/mode/stat-set, unavailable selection, average damage and DPS are distinct; derived state cannot be injected as an external assumption. |
| Injected definitions | Same input against a changed package rebinds; wrong kinds/units/owners reject; missing definitions remain explicit; no fixture IDs are built into code. |
| Five originals | Frozen selection manifest is preserved; every required unresolved mapping has a typed location; importing all five is not reported as five complete native evaluations. |

Direct construction, the owned codec, inventory unions and availability bindings are
implemented. Project/preset composition, definition binding and the
[owned draft/finalization boundary](owned-drafts.md) are also implemented. Full D1 still
needs revisioned repair/edit/adoption and the five-case adapter. An XML wrapper renamed
`BuildSpec` or a catalog-only facade does not meet that gate.
