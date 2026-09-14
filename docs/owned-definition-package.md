# D2: owned definition schema and offline mapping

**Status: implementation proposal, 2026-09-14.** The shared ID/value shapes below were
coordinated with the core input and `owned_definitions` implementers. The package, schema
index and compiler described here are proposed, not delivered. This specifies the smallest
D2 boundary needed to bind [owned D1 inputs](owned-build-contract.md), under the accepted
[domain architecture](domain-architecture.md) and [migration plan](architecture-migration.md).

A schema index establishes definition identity, input types and declared relationships.
It does not calculate effects, prove a generated actor exists, or establish complete D1/D2
delivery. Root-owned record/codec work is one D1 checkpoint; project/inventory composition,
five-build normalization and numerical rule conversion have separate completion gates.

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

Core owns these portable value/schema contracts and the binding-facing trait. Data owns
package decoding, validated immutable descriptor storage and its index implementation.
Offline tools own upstream readers, owned-ID allocation/mapping and conversion reports.
The import adapter consumes a separate external-to-owned map. No source program, raw
PoB callback/control field or source lookup is a dependency of the native schema index.

`BuildInput`, `ScenarioInput` and `QueryInput` are raw DTOs; their `BuildSpec`, `ScenarioSpec`
and `QuerySpec` wrappers establish structural validity. `QueryInput` carries its own
`game_version`, like build/scenario input; standalone validation checks that namespace and
referenced key shapes. `OwnedEvaluationRequest` checks a common namespace and provider-root
lineage against the build. Definition binding is subsequent and uses the exact index identity.
Well-formed saved queries can name missing/disabled providers; neither decoding nor this
index silently retargets them. Support assignments are provider roots too: a support gem
can declare an owned actor/action, independently of the supported action's provider.

## Durable owned IDs and package identity

Maintain a version-controlled owned-ID registry in offline tooling. It allocates symbols
once and records aliases to external identities separately. An owned key is not a PoB key,
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
definition. Reuse `DataIdentity` for the validated package digest, schema and semantic
version. Its shape alone proves neither trust nor source equivalence. Exact package identity
binds reports and later private plans; public definition IDs survive compatible updates.

## Minimal descriptor vocabulary

Use a closed `DefinitionDescriptor` enum with typed payloads, not a string-to-JSON map.
Every ID alias below already belongs to the agreed leaf. Each reference names a typed ID;
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

Membership lists must distinguish known empty from incomplete. Suggested DTOs:

```rust
enum SchemaState<T> { Known(T), Unmapped { gaps: Vec<SchemaGap> } }
struct DeclaredSet<T> { members: Vec<T>, closure: SchemaClosure }
enum SchemaClosure { Complete, Partial { gaps: Vec<SchemaGap> } }
struct DefinitionEntry<I, D> { id: I, schema: SchemaState<D> }

enum ValueSchema {
    Boolean,
    Integer { minimum: BoundedInteger, maximum: BoundedInteger },
    Quantity { minimum: FiniteQuantity, maximum: FiniteQuantity },
    Option { allowed: Vec<OptionDefId> },
}
struct QualityUseSchema {
    presence: Forbidden | Optional | Required,
    allowed_kinds: DeclaredSet<QualityDefId>,
}
struct QualitySchema { id: QualityDefId, minimum: FiniteQuantity, maximum: FiniteQuantity }
enum ParameterSite {
    ItemParameter, GemParameter, ModifierRoll, RewardParameter, UsagePolicyParameter,
}
struct ParameterSlotSchema {
    key: DeclaredSlot<ParameterSlotDefId>, value: ValueSchema,
    presence: RequiredOnce | OptionalOnce, sites: Vec<ParameterSite>,
}
struct ChoiceSlotSchema {
    key: DeclaredSlot<ChoiceSlotDefId>, value: ValueSchema,
    presence: RequiredOnce | OptionalOnce, owners: Vec<ChoiceOwnerScope>,
}
struct ExternalInputSchema {
    id: ExternalInputDefId, value: ValueSchema, targets: Vec<AssumptionTargetKind>,
}
struct UsagePolicySchema {
    id: UsagePolicyDefId, targets: Vec<UsageTargetKind>,
    parameters: DeclaredSet<DeclaredSlot<ParameterSlotDefId>>,
}
```

`SchemaGap` contains an owned definition/declared-slot subject, semantic facet and stable
issue code. Source spans and explanatory source text attach in conversion evidence, not in
the native descriptor. Partial collections never gain completion through an empty list.

These are proposed Rust-shaped records, not checked-in implementations. Range endpoints
must be ordered, finite and use the same exact unit. Closed `UnitDimension` categories cover
dimensionless factor, percentage points, count, time, rate, distance, damage, damage/time,
resource points and rating; distinct unit IDs can share a dimension without becoming
interchangeable. No value carries arbitrary text or an expression. Bounds describe accepted
input values; candidate-dependent caps/requirements remain domain rules, not schema defaults.

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

The index describes **potential topology**, before conditional rules produce a plan:

```rust
struct GrantSlotSchema {
    key: DeclaredSlot<GrantSlotDefId>, provider_roles: Vec<ProviderRole>,
    target: Skill(DeclaredSlot<SkillGrantSlotDefId>)
          | Actor(DeclaredSlot<ActorSlotDefId>)
          | AllocationAccess { pools: Vec<PointPoolDefId> },
}
struct SkillGrantSlotSchema {
    key: DeclaredSlot<SkillGrantSlotDefId>, skill: SkillDefId,
    outputs: DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
}
struct ActorSlotSchema {
    key: DeclaredSlot<ActorSlotDefId>,
    skills: DeclaredSet<SkillDefId>,
    outputs: DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
}
struct ActionOutputSchema {
    key: DeclaredSlot<ActionOutputDefId>, actor_role: DeclaredActorRole,
    parts: DeclaredSet<ActionPartDefId>, modes: DeclaredSet<ActionModeDefId>,
    stat_sets: DeclaredSet<ActionStatSetDefId>,
    choices: DeclaredSet<DeclaredSlot<ChoiceSlotDefId>>,
}
```

`ProviderRole` covers Character, EquipmentUse, ItemModifier, SkillUse, SupportAssignment,
Allocation and Reward. `DeclaredActorRole` states whether the output belongs to the player,
its provider's actor, or a declared owned-actor slot. It is relative to the concrete provider
path, never a runtime actor number. An actor slot describes a calculation actor/population, not one object
per simulated summon. These links can bind player and minion queries from the first slice.

A slot's identity is its exact declaring owned definition plus typed slot ID. Store and
look it up by that pair; never use a bare slot ordinal. Duplicate pairs reject. The index
checks that each link reaches the declared target definition/output and that each action's
part/mode/stat-set and selected choice slot belong to that output. Narrower choice
applicability within a part/mode/stat-set requires an explicit rule constraint. Definitions
provide durable discriminators for multiple emitted slots. Where repetition is driven by
distinct authored providers, the instance/provider key supplies that distinction; unknown
repetition semantics remain a gap.

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

Suggested typed trait; generic lookup is intentional, so D1 consumers use a type parameter
rather than requiring a trait object:

```rust
trait DefinitionSchemaIndex {
    fn identity(&self) -> &DataIdentity;
    fn namespace(&self) -> &GameVersionNamespace;
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
}
```

Sealed `SchemaDefinitionId`/`SchemaSlotId` traits associate existing typed aliases with their
closed descriptor records; they introduce no new wire ID representation. Slot families use
the declared-pair method. Missing is not proof of absence when the enclosing declared set
has partial closure. Data's validated immutable index is the production implementation;
caller-authored package bytes use the same loader to create independent test indexes.
Lookup has no filesystem, process, VM, mutation, hidden defaults or numerical callback.

Keep outcomes separate:

| Situation | Binding outcome |
| --- | --- |
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

The first artifact can be a separately versioned, bounded `OwnedDefinitionSchemaPackage`:
namespace, owned release/schema-semantics version, typed descriptor entries, declared slots
and semantic coverage gaps. The loader rejects duplicate standalone typed IDs and duplicate
declared-slot pairs, namespace disagreement, invalid reference kinds, required-value shapes,
range/unit conflicts and resource-limit violations. Every emitted resolved link reaches a
package entry/declared slot. An unconverted link stays a gap; it is not a dangling reference
presented as resolved. An identity-only target entry still reports its unmapped schema.
Canonical bytes determine its `DataIdentity`. Source filenames, declaration winners,
Lua flags/tables and raw source text are absent. Do not reuse the legacy package schema or
require Spark/Mace sections. A schema-only package is explicitly insufficient for evaluation;
this milestone does not satisfy D2's required representative rule-conversion gate.

Keep two associated tooling artifacts outside the native schema package:

```rust
struct ExternalMappingEntry {
    source: PinnedExternalSelector,
    outcome: Mapped(OwnedDefinitionRef)
           | Ambiguous(Vec<OwnedDefinitionRef>) | Unmapped(MappingIssueCode),
}
struct ConversionGap {
    subject: ExternalSubject | OwnedSemanticSubject,
    facet: Identity | InputSchema | StaticLinks | GameRules,
    code: ConversionIssueCode,
    evidence: OptionalSourceLocation,
}
```

`OwnedDefinitionRef` here is a closed tooling union of the agreed typed IDs, not an erased
core key or new native lookup path. External selectors are tagged by source record kind:
gem `(gameId, variantId)`, explicit effect key, item base/prototype, tree version/node plus
view discriminator, configuration key and value role. The pin includes revision and relevant
file digests. A missing selector component is distinct from an empty string or an unknown
value. One exact selector cannot silently map to different owned definitions under the same
pin. Multiple aliases may map to one owned ID only through explicit reviewed continuity or
equivalence; equal labels alone do not merge definitions. Mapping tables are data; runtime
Rust must not switch on a named build/skill.

The offline pipeline is:

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

## Smallest implementation and acceptance slice

Implement descriptor DTOs, the portable schema trait, one bounded loader/index and an
offline identity-registry/mapping compiler for representative record kinds. Bind directly
authored D1 inputs through the same index used by imported inputs. Do not implement search,
simulation or a new profile to demonstrate lookup.

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
  replacing a provider cannot retarget a query.
- Partial slot topology differs from a known empty set. Missing/disabled query roots remain
  serializable; binding reports unavailable or pending as appropriate. Standalone query
  namespaces are checked before common-namespace request binding.
- The original-five selection manifest remains unchanged, including player/minion action
  ownership, primary/additional outputs and separate stat sets. Report mapping coverage,
  schema binding and rule coverage separately; none is a count of completed evaluations.

Static source findings and the shared-type coordination record are in
`runs/owned-package-01/source-audit.md`. No runtime or numerical result was produced for
this proposal.
