# D2: owned definition schema and offline mapping

**Status: schema DTOs, typed index interface and data loader implemented in source,
2026-09-14.** The core [schema contracts](../crates/poe-optimizer-core/src/owned_schema.rs)
and data [package loader/index](../crates/poe-optimizer-data/src/owned_schema.rs) are present.
Request binding, the offline compiler, the durable ID registry and external mapping artifacts
remain planned in this document. No validation receipts are asserted here.

This boundary supports [owned D1 inputs](owned-build-contract.md) under the accepted
[domain architecture](domain-architecture.md) and [migration plan](architecture-migration.md).
It establishes definition identity, input schemas and declared relationships. It does not
calculate effects, prove generated actors exist, or establish complete D1/D2 delivery.
Five-build normalization, representative numerical rule conversion and D3 resolution retain
separate completion gates.

## Shared types and ownership

Use `core::owned_definitions` directly. Do not duplicate its ID parser or scalar wire types:

- `OwnedDefinitionKey` is at most 128 ASCII characters matching `[a-z0-9][a-z0-9_.-]*`.
- `GameVersionNamespace { game, version }` contains owned keys.
- `DefId<K>` encodes `{ kind, namespace: { game, version }, key }`. Closed, sealed domains
  supply the typed aliases and stable snake-case `DefinitionKind` tags.
- `FiniteQuantity` encodes `{ value, unit: UnitDefId }`, rejects nonfinite values and
  normalizes negative zero. `BoundedInteger` admits integers within ±(2^53−1); descriptor
  ranges narrow that representational bound.

Use the core input records' `DeclaredSlot<S> { declaration: SlotOwnerDefId, slot: S }` and
`ParameterValue::{Boolean, Integer, Quantity, Option}`. `SlotOwnerDefId` includes Class,
Ascendancy, Reward, ItemTemplate, Modifier, Gem, Skill, PassiveNode and UsagePolicy.
The last variant lets an independent usage policy declare its own parameters.

Core implements these portable value/schema contracts and the binding-facing trait. Data
implements package decoding, validated immutable descriptor storage and its index.
Planned offline tools will own upstream readers, owned-ID allocation/mapping and conversion
reports; a future import adapter will consume the separate external-to-owned map. No source
program, raw PoB callback/control field or source lookup is a dependency of the native index.

`BuildInput`, `ScenarioInput` and `QueryInput` are raw DTOs; their `BuildSpec`, `ScenarioSpec`
and `QuerySpec` wrappers establish structural validity. `QueryInput` carries its own
`game_version`, like build/scenario input; standalone validation checks that namespace and
referenced key shapes. `OwnedEvaluationRequest` checks a common namespace and provider-root
lineage against the build. Definition binding is subsequent and uses the exact index identity.
Well-formed saved queries can name missing/disabled providers; neither decoding nor this
index silently retargets them. Support assignments are provider roots too: a support gem
can declare an owned actor/action, independently of the supported action's provider.

## Durable owned IDs and package identity (registry planned)

The offline compiler must maintain a version-controlled owned-ID registry; that registry
and allocation workflow are not implemented yet. That registry will allocate symbols once
and record aliases to external identities separately. An owned key is not a PoB key,
a slug of the current display name, a source-path/index, or a content hash. New keys may
be allocated deterministically from the registry's persisted counter; subsequent builds
read the registry rather than regenerate IDs from source order. Never reuse retired keys.
Concurrent allocation conflicts require registry reconciliation before publication.

Names and upstream keys can change while the owned identity remains stable. Retain identity
only when review establishes semantic continuity. A replacement definition or changed
grant/output meaning receives a new identity, with an explicit migration when appropriate.
Aliases are mapping facts, not native lookup fallbacks. Labels may be localized independently.

The owned namespace identifies a compatible game/rules family. A balance package has its
own release and exact content/semantics identity; a balance change need not rename every
definition. The loader uses `DataIdentity` for the validated package digest, schema and
semantics version. Its shape alone proves neither trust nor source equivalence. Exact package identity
binds reports and later private plans; public definition IDs survive compatible updates.

## Minimal descriptor vocabulary

Core implements closed `DefinitionDescriptor` and `SlotDescriptor` enums with typed payloads.
Every ID alias below belongs to `owned_definitions`. Each reference names a typed ID;
all displayed labels and original text live in optional presentation/debug sidecars.

| Descriptor family | Minimum binding facts |
| --- | --- |
| Class, Ascendancy, Reward | Typed class/ascendancy relationships, choice slots, reward parameter slots, potential grant slots. Numerical attributes and earned-point rules belong to the subsequent rules layer. |
| ItemTemplate, Modifier, Quality | Item-level ranges; eligible destination/socket kinds; template-owned instance parameters and modifier-owned roll slots; quality presence/allowed-kind policy and quality unit/range; potential grants. Actual parameters and rolls remain build data. Affix legality and local arithmetic are not implied by membership. |
| Gem, Skill | Allowed authored roles (skill use and/or support assignment), gem-level ranges and gem-owned instance parameter slots, quality presence/allowed-kind policy, directly selectable skill status, typed gem-to-skill links and declared outputs/grants. A support may declare grants and actors. No per-skill Rust variant. |
| PassiveNode, PointPool | Owned node/pool references and topology, choice/socket slots, shared versus loadout-scoped pool identity, potential allocation-access grants. Known compatibility is distinct from earned budgets, connectivity exceptions and transform evaluation. |
| EquipmentSlot, SocketSlot | Semantic destination kind, containing declaration and loadout scope. An item socket and a passive socket are different addresses. No PoB slot-name matching. |
| Encounter, ExternalInput | Enemy-level input range, allowed external-input definitions and target kinds; external values have declared types and units. There is no generic character-stat override. |
| Metric, Unit | Metric target kind, result unit and supported selector roles; unit identity and dimension. Average damage and damage/time are distinct definitions. Unit equality is exact; D1 does not silently convert units. |
| UsagePolicy, SkillLinkRole | Permitted actor/action/skill target kinds and parameter slots; permitted authored container/payload roles. This declares input structure, not actual trigger rates or support applicability. |
| Option, ActionPart, ActionMode, ActionStatSet | Typed alternatives and membership in their declaring choice/output context. A stat set is not another generated action. |

The implemented DTOs distinguish known empty membership from incomplete coverage. These
excerpts omit visibility and derives; field and variant shapes match the core API:

```rust
enum SchemaState<T> { Known(T), Unmapped { gaps: Vec<SchemaGap> } }
struct DeclaredSet<T> { members: Vec<T>, closure: SchemaClosure }
enum SchemaClosure { Complete, Partial { gaps: Vec<SchemaGap> } }
struct DefinitionEntry<I, D> { id: I, schema: SchemaState<D> }

struct IntegerRange { minimum: BoundedInteger, maximum: BoundedInteger }
struct QuantityRange { minimum: FiniteQuantity, maximum: FiniteQuantity }
enum ValueSchema {
    Boolean,
    Integer(IntegerRange),
    Quantity(QuantityRange),
    Option { allowed: DeclaredSet<OptionDefId> },
}
enum QualityPresence { Forbidden, Optional, Required }
struct QualityUseSchema {
    presence: QualityPresence,
    allowed_kinds: DeclaredSet<QualityDefId>,
}
struct QualitySchema { amount: QuantityRange }
enum SlotPresence { RequiredOnce, OptionalOnce }
enum ParameterSite {
    ItemParameter, GemParameter, ModifierRoll, RewardParameter, UsagePolicyParameter,
}
struct ParameterSlotSchema {
    value: ValueSchema, presence: SlotPresence, sites: Vec<ParameterSite>,
}
struct ChoiceSlotSchema {
    value: ValueSchema, presence: SlotPresence, owners: Vec<ChoiceOwnerScope>,
}
struct ExternalInputSchema {
    value: ValueSchema, targets: Vec<AssumptionTargetKind>,
}
struct UsagePolicySchema {
    targets: Vec<UsageTargetKind>, declarations: DeclaredSlots,
}
struct DeclaredSlots {
    parameters: DeclaredSet<DeclaredSlot<ParameterSlotDefId>>,
    choices: DeclaredSet<DeclaredSlot<ChoiceSlotDefId>>,
    grants: DeclaredSet<DeclaredSlot<GrantSlotDefId>>,
    actors: DeclaredSet<DeclaredSlot<ActorSlotDefId>>,
    skill_grants: DeclaredSet<DeclaredSlot<SkillGrantSlotDefId>>,
    outputs: DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
    sockets: DeclaredSet<SocketSlotDefId>,
}
```

Entries carry identity; payload schemas do not duplicate it. For example,
`DefinitionDescriptor::Quality` contains `DefinitionEntry<QualityDefId, QualitySchema>`;
`SlotDescriptor::Parameter` contains
`DefinitionEntry<DeclaredSlot<ParameterSlotDefId>, ParameterSlotSchema>`. There are 22
standalone descriptor families and six declared-slot families. SocketSlot is standalone,
with an explicit `owner`, `kind` and `scope`. Option/ActionPart/ActionMode/ActionStatSet have
empty typed payloads; their contextual membership is declared by the relevant slot/output.

`SchemaGap { subject, facet, code }` uses
`SchemaSubject::Definition(DefinitionAddress)` or `SchemaSubject::Slot(SlotAddress)`,
`SchemaFacet::{Identity, InputSchema, StaticLinks, GameRules}` and a bounded
`OwnedDefinitionKey` issue code. Source spans and explanatory text belong to future
conversion evidence. Partial collections and unmapped entries require nonempty gap evidence
in the loader. Empty closed role/site vectors mean known none, never unrestricted use.

Core DTO construction/deserialization alone does not establish a valid package. The data
loader checks ordered range endpoints and exact quantity-unit agreement. Closed
`UnitDimension` variants are DimensionlessFactor, PercentagePoints, Count, Time, Rate,
Distance, Damage, DamagePerTime, ResourcePoints and Rating. Distinct unit IDs can share a
dimension without becoming interchangeable. Candidate-dependent caps and requirements remain
rules, not schema defaults.

The following input-binding behavior is the pending binder contract; the package loader
validates schema declarations, not the values or availability in a concrete request.

Both item and gem quality are `Option<QualitySelection>`; the wire field is required even
when its value is null. Their template/gem descriptor supplies `QualityUseSchema`. A present
quality kind must be allowed and its amount must match that kind's `QualitySchema` unit and
range. Null, a present zero amount and an omitted field are distinct. No default quality kind
or amount is inferred; incomplete allowed-kind membership remains unresolved coverage.

The five `ParameterSite` variants correspond to `ItemRecord.parameters`,
`GemInstance.parameters`, `RolledModifier.rolls`, `RewardSelection.parameters` and
`UsagePolicySelection.parameters`. Structural validation requires every declared slot's
owner to equal the exact enclosing ItemTemplate, Gem, Modifier, Reward or UsagePolicy
definition, respectively. D2 checks definition existence, explicit slot membership, value
kind, unit, allowed options and range. There is no implicit inherited-slot lookup. Intrinsic
rarity, corruption, socket or variant inputs belong to declared instance parameters when
applicable; an equipment-use choice or fabricated modifier cannot stand in for them.

`ChoiceOwnerScope` corresponds to core Character, EquipmentUse, Allocation, Skill, Action and
Provider owners, with typed provider-role restrictions where needed. An Action choice binds
to its complete `ActionSelection`, including output, part, mode and stat set; it does not
apply to every action supplied by the same skill. `AssumptionTargetKind` is
Environment/Enemy/Actor/Skill; `UsageTargetKind` is Actor/Action/Skill. Classifying the owner
kind is insufficient by itself: the selected provider's actual declaration must expose the
slot. `OptionDefId` must be a declared alternative for that exact slot.

Presence never inserts an implicit value. The author/import adapter must normalize an
established default into explicit input, or report a missing required choice. A source
widget's scalar kind, placeholder or callback is not an owned value schema. External inputs
feed declared external-input nodes; they cannot overwrite effective attributes, derived
conditions or other candidate-computed nodes by sharing a name.

## Declared grants and output links

The index stores **potential topology**, before conditional rules produce a plan. Identity
is held by each `DefinitionEntry<DeclaredSlot<...>, ...>` outside these slot payloads:

```rust
struct GrantSlotSchema {
    provider_roles: Vec<ProviderRole>, target: GrantTarget,
}
enum GrantTarget {
    Skill(DeclaredSlot<SkillGrantSlotDefId>),
    Actor(DeclaredSlot<ActorSlotDefId>),
    AllocationAccess { pools: DeclaredSet<PointPoolDefId> },
}
struct SkillGrantSlotSchema {
    skill: SkillDefId, outputs: DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
}
struct ActorSlotSchema {
    skills: DeclaredSet<SkillDefId>,
    outputs: DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
}
struct ActionOutputSchema {
    actor_role: DeclaredActorRole,
    parts: DeclaredSet<ActionPartDefId>, modes: DeclaredSet<ActionModeDefId>,
    stat_sets: DeclaredSet<ActionStatSetDefId>,
    choices: DeclaredSet<DeclaredSlot<ChoiceSlotDefId>>,
}
enum DeclaredActorRole {
    Player,
    ProviderActor,
    OwnedSlot(DeclaredSlot<ActorSlotDefId>),
}
```

`ProviderRole` covers Character, EquipmentUse, ItemModifier, SkillUse, SupportAssignment,
Allocation and Reward. `DeclaredActorRole` states whether the output belongs to the player,
its provider's actor, or a declared owned-actor slot. It is relative to the concrete provider
path, never a runtime actor number. An actor slot describes a calculation actor/population,
not one object per simulated summon. The binder supports both player and owned actor queries through these declarations.

A slot's identity is its exact declaring owned definition plus typed slot ID. The loader
indexes that pair, rejects duplicates and checks referenced entries exist. Direct
`DeclaredSlots` lists must name their enclosing owner. Cross-links in grant, skill and actor
payloads remain explicit typed references and may cross declarations; their contextual
compatibility belongs to binding. A registered slot omitted from its known, complete owner's
list rejects; a partial or unmapped owner leaves that relationship unresolved.

Request binding must check that the selected part/mode/stat-set and choice belong to the
selected output. Narrower applicability needs an explicit rule constraint. Required choices
apply only to the concrete authored/bound context, not every potential output in the package.
Declaration membership is a registry fact, not proof of contextual reachability.

The reviewed grant-path contract retains the parent provider identity when entering a target:

- Let `P` be the provider before grant `G`. Traversing `G -> Actor(A)` enters `A`'s explicit output
  context while retaining `OwnedActorKey { provider: P, slot: A }` as the current actor key.
  The traversed provider path `P.G` is a context address; an actor key with provider `P.G`
  and slot `A` would identify a different child slot. It must not alias the current actor.
  `ActorSlotSchema` currently exposes no child-actor collection.
- Traversing `G -> Skill(S)` enters the referenced `SkillDefId` context with `S.outputs` as
  an explicit output constraint. The supplied skill keeps
  `GeneratedSkillKey { provider: P, slot: S }`; extending the path does not rebase its identity.

A binder must not re-expose sibling actor/grant slots by looking up all declarations on a
traversed target's owner. It must preserve the current provider/actor/skill context and follow
only links exposed there. These are implemented binding requirements, not behavior performed by the
loader. Generated activation and existence remain D3 responsibilities.

An item-use or support-assignment root owns its generated descendants. Changing the supplied
occurrence allocates a new root under the D1 edit contract. A class/ascendancy change cannot
reuse another declaration's slot. Same-occurrence numerical edits can preserve public keys,
but require fresh compatible plan binding. Neither a new plan nor a same-named target may
silently repair an obsolete public selector.

`ProviderRoot::ItemModifier { equipment_use, modifier }` identifies an exact equipment-use
and modifier occurrence pair. Build-internal structural references require that the modifier
belongs to the item referenced by that use; D2 binds its grants to that occurrence's exact
Modifier definition. Equal-definition modifiers on one item, or the same modifier under
different equipment uses, remain distinct providers. Same-identity roll edits retain public
selectors and require fresh plan binding. Replacing either supplying occurrence changes the
pair identity; an old saved query stays representable and binds as unavailable, never to the
next equal-definition modifier.

Declared support/trigger/grant links do not certify support application, activation, costs,
actor inheritance, ownership-sensitive effects, point grants or feedback behavior. Those
require typed domain-rule conversion and D3 resolution. Do not synthesize a complete actor
graph from this index or accept an empty rule list as the meaning of an unconverted effect.

## Lookup and validation contract

The implemented interface uses sealed associations and immutable address lookup hooks.
Generic methods have default bodies; signatures are shown here with bodies omitted:

```rust
trait SchemaDefinitionId {
    type Descriptor;
    fn address(&self) -> DefinitionAddress;
    fn project(descriptor: &DefinitionDescriptor)
        -> Option<&SchemaState<Self::Descriptor>>;
}
trait SchemaSlotId: Sized {
    type Descriptor;
    fn address(key: &DeclaredSlot<Self>) -> SlotAddress;
    fn project(descriptor: &SlotDescriptor)
        -> Option<&SchemaState<Self::Descriptor>>;
}
trait DefinitionSchemaIndex {
    fn identity(&self) -> &DataIdentity;
    fn namespace(&self) -> &GameVersionNamespace;
    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor>;
    fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor>;
    fn definition<I: SchemaDefinitionId>(&self, id: &I)
        -> SchemaLookup<'_, I::Descriptor>;
    fn slot<S: SchemaSlotId>(&self, key: &DeclaredSlot<S>)
        -> SchemaLookup<'_, S::Descriptor>;
}
enum SchemaLookup<'a, T> {
    Known(&'a T),
    Missing,
    Unmapped(&'a [SchemaGap]),
    NamespaceMismatch,
    InconsistentIndex,
}
```

The omitted private sealing bounds prevent callers from adding ID families. `DefinitionAddress`
contains the 22 standalone typed IDs; `SlotAddress` contains the six exact declared pairs.
Both are ordered owned keys. Their `namespace()`, `key()` and `kind()` helpers and each
descriptor's `address()` avoid repeated enum dispatch in consumers. `SlotAddress::declaration()`
returns the owner; its `namespace()` returns the slot ID namespace, checked separately.

The defaults reject namespace mismatches before lookup. Otherwise they call one hook,
check the full returned address and project the immutable typed payload. A wrong hook result returns `InconsistentIndex` rather
than silently selecting another entry. The data implementation stores BTreeMap address-to-index
maps into canonical descriptor vectors; lookup does not scan the package or compute effects.
Missing is not proof of absence when enclosing membership is partial. No lookup traverses
links, loads source, mutates state, supplies defaults or invokes a numerical callback.

The following request-binding outcomes remain requirements for the pending binder:

| Situation | Binding outcome |
| --- | --- |
| Lookup hook returns an inconsistent address or descriptor | Explicit index-implementation failure; no lookup fallback or input blame. |
| Malformed ID/value/wire kind, or a record-local parameter declaration differing from its enclosing definition | Structural/codec error before definition lookup. |
| Unavailable package, missing definition, unconverted schema or partial required relationship set | Explicit unresolved schema/coverage issue. No invented ID, value or empty effects. |
| Known authored parameter/choice slot with wrong value kind/unit/option, absent declared membership or incompatible semantic owner context | Definition binding violation at the semantic input location. |
| Structurally valid saved query whose concrete provider was removed/disabled or no longer exposes its declaring definition | Selector unavailable for this request; the saved query remains roundtrippable. |
| Declared generated output whose activation/producer is not yet resolved | Selector pending resolution, not a fabricated actor or proof that it is absent. |
| Bound input with incompatible game requirements or unavailable requested measurement | Legality/measurement classification, separate from schema validity. |

Reports retain the input binding and exact index identity, with typed semantic locations.
A schema-only report is never an evaluation-plan authority token. Successful lookup does
not authenticate package provenance or implemented mechanics. D3 independently checks its
required operation versions, plan ownership and complete dependency coverage.

## Package and offline mapping artifacts

The data loader implements version 1 with this raw artifact shape:

```rust
struct SchemaPackageInput {
    schema_version: u32,
    namespace: GameVersionNamespace,
    release: OwnedDefinitionKey,
    semantics_version: OwnedDefinitionKey,
    definitions: Vec<DefinitionDescriptor>,
    slots: Vec<SlotDescriptor>,
}
struct OwnedSchemaLimits {
    max_entries: usize,
    max_collection_entries: usize,
    max_wire_bytes: usize,
}
```

`OwnedDefinitionSchemaPackage::new(input, limits)` validates and canonicalizes raw DTOs.
`decode_schema_package(bytes, limits)` bounds input bytes, decodes the strict wire shape and
uses that constructor. `encode_schema_package(&package, limits)` checks retained resource
counts and returns canonical bytes. These return `SchemaPackageError` on failure. The
private package exposes `input()` and `identity()` and implements `DefinitionSchemaIndex`;
deserializing a raw DTO cannot construct validated package authority.

Implemented loader checks include schema version, namespaces and typed reference closure,
duplicate addresses/members, direct declaration ownership, reverse membership for complete
owners, parameter-site/owner compatibility, socket owner/kind compatibility, ordered exact-unit
ranges, nonempty gap evidence and resource bounds. Gap subjects must also resolve to package
entries. All descriptor vectors and membership collections are canonicalized as unordered
sets; this schema has no implicit sequence order. Partial/unmapped coverage remains explicit,
including when a referenced entry has identity but lacks its schema.

Default limits are 1,000,000 aggregate collection entries, 100,000 entries per collection and
64 MiB of wire bytes. Callers may adjust them up to hard caps of 4,000,000, 1,000,000 and
256 MiB respectively. These are implementation bounds, not measured full-game capacity.

`DataIdentity.content_sha256` hashes the canonical package JSON bytes; game, release, schema
version and semantics version derive from the validated artifact. Its namespace is included
in those bytes. Source filenames, declaration winners, source programs and raw text are
absent. This source implementation is independent of legacy package schemas and named skill
profiles. It contains no rules and cannot supply evaluation authority or satisfy D2's
representative rule-conversion gate. No test, CI or numerical receipts are claimed here.

Two associated tooling artifacts remain planned outside the native package. An external
mapping entry will pair a pinned source selector with Mapped, Ambiguous or Unmapped outcomes.
Conversion evidence will pair an external/owned subject and semantic facet with a stable
issue code and optional source location. Neither artifact nor its compiler is implemented by
the schema loader.

`SchemaSubject` is the shared closed union of typed definition and declared-slot addresses;
the implemented offline mapping uses it without adding another erased native lookup path. External selectors are tagged by source record kind:
gem `(gameId, variantId)`, explicit effect key, item base/prototype, tree version/node plus
view discriminator, configuration key and value role. The pin includes revision and relevant
file digests. A missing selector component is distinct from an empty string or an unknown
value. One exact selector cannot silently map to different owned definitions under the same
pin. Multiple aliases may map to one owned ID only through explicit reviewed continuity or
equivalence; equal labels alone do not merge definitions. Mapping tables are data; runtime
Rust must not switch on a named build/skill.

The planned offline pipeline is:

1. Read the pinned upstream sources or reusable extracted factual catalogs. Optional Lua
   execution stays in acquisition tooling. Do not require constructing the entire legacy
   source-program package to convert one catalog, and do not make acquisition a Cargo
   `build.rs` or native-startup step.
2. Resolve external identities through the persisted owned-ID registry. Exact gem/variant
   matches map directly. A documented single-candidate fallback is adapter compatibility
   evidence; a multi-candidate `pairs` winner or a matching display name is not a mapping.
   An authored unknown external gem ID must not fall through to a supplied effect ID.
3. Convert record-kind facts to typed input descriptors and declared links. Normalize raw
   numeric units with explicit tested conversions; separate primary/additional effects,
   actor-granted actions and stat sets. Translate configuration by semantic role into build
   choices, external assumptions or query selection; UI-only records stay in provenance.
4. Record every unresolved identity/schema/rule mapping. An identity-only entry can retain
   a reviewed owned ID while its schema is `Unmapped`; it cannot claim known empty behavior.
   Unowned external rows stay in the mapping ledger and import drafts, not fake native IDs.
5. Validate/canonicalize, compare registry and semantic diffs, then publish schema bytes,
   optional adapter map and conversion evidence with their own hashes/versions. Existing
   source observers can investigate discrepancies without becoming release parity targets.

Later D2 rule packages lower local arithmetic, conditions, support applicability and grants
into reviewed typed operations using these same IDs. Unknown operations or missing active
dependencies stay explicit. Compatible coefficient changes update data; new operations
require versioned Rust semantics and tests. There is no source VM or generic text-expression
fallback inside the package.

## Remaining implementation and acceptance gates

The core DTOs/index, bounded data loader, request binder and durable offline ID-registry/mapping
artifacts are implemented. See [composition and binding](owned-binding.md) for the delivered APIs.
The next work is source conversion for representative record kinds and all-five normalization. Directly authored and imported D1 inputs must use the same
binding boundary. Search, simulation and numerical rule conversion remain separate work.

The binder checks every concrete authored parameter, choice and quality selection
against indexed kind, unit, range, membership and owner/action context. Its report binds
to the exact request content and `DataIdentity`, retaining every query in its original order.
It distinguishes invalid input, unresolved schema, saved-query selector unavailability and
generated targets pending resolution. `RequiredOnce` applies to concrete authored sites and
selected outputs, not all inactive potential definitions. Missing, Unmapped and inconsistent
index results remain distinct; none authorizes a fallback to a different definition or slot.

Core [schema tests](../crates/poe-optimizer-core/tests/owned_schema.rs) and data
[loader tests](../crates/poe-optimizer-data/tests/owned_schema.rs) exist in source; their presence
is not a validation receipt. The following acceptance requirements remain distinct from
implemented API inventory and must be tied to receipts before claiming completion.

Required contrasting evidence:

- Pure authored package loads without a PoB checkout; two injected packages coexist.
  Changing a bound/range or later coefficient retains IDs but changes package identity.
- Definition/slot kind, namespace, owner, duplicate pair, unit/range and option-membership
  failures are explicit. Required choices are never filled from UI defaults.
- All five parameter sites enforce their exact enclosing definition, then bind through
  explicit declared membership. Action choices are checked in the full selected action
  context. Quality forbidden/optional/required presence, allowed kind and amount are checked;
  null is never silently replaced by a kind or zero.
- Registry IDs survive renamed external keys, labels and source ordering; genuine semantic
  replacements get new IDs. Ambiguous variants and name-only rows remain unresolved.
- Duplicate skill/item/support instances keep different provider roots. Repeated
  equal-definition item modifiers and separate equipment-use/modifier pairs also remain
  distinct. Item-, modifier- and support-granted actors retain their provider identity;
  replacing a provider cannot retarget a query. Traversing an actor/skill grant must retain
  the parent-provider target identity and must not re-expose sibling slots.
- Partial slot topology differs from a known empty set. Missing/disabled query roots remain
  serializable; binding reports unavailable or pending as appropriate. Standalone query
  namespaces are checked before common-namespace request binding.
- The original-five selection manifest remains unchanged, including player/minion action
  ownership, primary/additional outputs and separate stat sets. Report mapping coverage,
  schema binding and rule coverage separately; none is a count of completed evaluations.

Earlier source findings and type coordination are recorded in
`runs/owned-package-01/source-audit.md`. Current API facts above come from the linked source
files. Validation receipts, mapping coverage and numerical results are separate evidence.
