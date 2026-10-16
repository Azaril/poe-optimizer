# Shared baseline rules on the existing scenario Enemy

Status: **proposed, awaiting project-owner review**. Date: 2026-10-15.
Decider: project owner. No runtime changes are implemented by this proposal.

## Decision to review

Extend the existing explicit shared-definition applicability mechanism to bind
an injected baseline rule owner to the **actual scenario Enemy**, once. Keep the
Enemy's baseline mechanics separate from Encounter configuration. Reuse the
current rule graph, source membership and coverage machinery; create no Actor,
provider, Enemy copy, or second evaluator.

The earlier [shared Player ownership decision](owned-existing-actor-rule-ownership-proposal.md)
approved the implemented Player scope only. This is a new Enemy ownership and
local-permission contract, requiring review. It does not reopen the accepted
[stat-owned incoming-critical receiver decision](owned-actor-enemy-reads-proposal.md).

## Concrete need and current boundary

Original05's physical mitigation needs these shared Enemy baseline suppliers:
conditional DamageTaken for Intimidate and Unnerve, and PhysicalDamageReduction
for Crushed. CalcSetup initializes them on Enemy before loading the selected
configuration. They remain potential suppliers even when their conditions are
false. Configured Armour BASE `31e0` is different: it belongs to the selected
Encounter's actual configuration program. The
[source review](../data/owned/poe2/3887ae68/enemy-physical-mitigation/source-domain-review.md)
records their distinct ownership and remaining late-factory obligations.

Current runtime identities are explicit:

- `ActorKey` is Player or a supplied owned Actor. It has no Enemy variant.
- Enemy is `ConcreteEntity::Enemy`; the validated request supplies one scenario
  Enemy. It is not `ActorKey::Player` or an owned minion.
- `ProgramOccurrenceKey` already carries origin, owner, program and concrete
  entity. `EffectOccurrenceKey` adds the exact effect ID.
- Existing shared applicability owns an `ActorDefId`, but targets only Player.
  Data requires Actor-context programs and actor-local effects; Engine binds
  `RuleOrigin::ExistingActor { application, actor: ActorKey::Player }`.
- Checked contribution membership admits that explicit applicability origin,
  and currently verifies that its concrete invocation is the actual Player.

An `ActorDefId` here is a reusable static rule definition, not evidence that its
runtime recipient must be represented by `ActorKey`. Reusing that definition
type does not require changing Enemy identity or renaming unrelated Actor schemas.

## Recommended contract

Generalize the existing applicability seam **in place**. Suggested current-format
names below make its two admitted target kinds explicit; names are secondary to
the ownership decision. Replace the old field/types/origin spelling rather than
keeping aliases or a parallel registry.

```rust
pub existing_entity_rules: Option<DeclaredSet<ExistingEntityRuleApplication>>;

pub struct ExistingEntityRuleApplication {
    pub id: OwnedDefinitionKey,
    pub owner: ActorDefId,
    pub targets: Vec<ExistingEntityRuleTarget>,
}

pub enum ExistingEntityRuleTarget {
    Player,
    Enemy,
}

// Engine invocation origin; the existing invocation separately carries entity.
RuleOrigin::ExistingEntity {
    application: OwnedDefinitionKey,
    target: ExistingEntityRuleTarget,
}

// Injected checked contribution producer declaration.
ContributionOrigin::ExistingEntity {
    application: OwnedDefinitionKey,
}
```

The bounded target enum is preferable to placing arbitrary `ConcreteEntity` in
this origin: it cannot name EquipmentUse, Action, Skill, Environment or a supplied
Actor. During binding, Player must resolve to
`ConcreteEntity::Actor(ActorKey::Player)` and Enemy to `ConcreteEntity::Enemy`.
The origin target and invocation entity must agree. Never derive this identity
or permission from a generic compiler context's Actor field or Player fallback.
Existing owned-Actor targets remain unsupported by this decision.

Keep the existing complete, empty declaration requirements for shared owners:
no parameters, choices, grants, Actor slots, Skill grants, outputs or sockets.
Bind each application owner's entire declared program inventory; do not select
only convenient programs. Duplicate `(owner, target)` applications are invalid,
including duplicates with different application IDs.

Player applications retain Actor-context inventories. Enemy applications require
Enemy-context inventories. The current proposal does not introduce polymorphic
program contexts: a single owner cannot apply a mixed or incompatible inventory
to both target kinds. Use a separate shared Enemy owner from the existing shared
Player owner, while reusing the definition type and applicability machinery.
Shared Enemy rules then exist once across Encounter choices. A shared owner must
not also acquire an unreviewed provider/supply invocation that duplicates or
changes its target authority.

## Target-specific permissions

Validate each whole shared-owner inventory against its declared target, including
unused programs. Do not widen a common Actor allowlist and rely on runtime values
to avoid forbidden reads.

| Authority | Existing Player application | New Enemy application |
| --- | --- | --- |
| Program context | Existing Actor context | Enemy context only |
| Local Stat/capability reads | Existing Player/Actor/current permission | Current or Enemy, resolving to the same scenario Enemy |
| Checked contribution reads | Existing Player permission | Local Enemy Flag queries; numeric queries/selection only if the separate proposal is accepted |
| External inputs | Existing reviewed permission unchanged | Typed inputs bound to the same Enemy only |
| Canonical level | Existing Character/Enemy-level permission unchanged | EnemyLevel only |
| Writes | Existing actor-local Contribute/Derive permission | Contribute/Derive to Current or Enemy only, with matching Enemy Stat targets and types |
| Character class/ascendancy/level and Player equipment slots | Existing explicit Player permission unchanged | Rejected |
| Player, Actor, provider, equipment, Skill, Action, effect source/recipient or cross-Actor access | Existing restrictions unchanged | Rejected |

Enemy rules gain no Environment reads/writes, topology mutation, new external
input ownership, or authority to follow arbitrary relative actors. Local reads
still require real producers, typed definitions, stage ordering and complete
checked membership. Unknown conditions remain unknown; no missing source becomes
false, zero or a neutral multiplier. Existing data tables and typed expressions
remain available through ordinary program validation.

This supplies intrinsic/source ownership. It does **not** move final
incoming-critical results from their accepted Player Stat receivers, broaden
shared Player computed Enemy reads, or turn a shared owner into a general final
metric aggregator. Stat receivers retain their existing target and single-final-
Stat write contract. Adding Enemy stat receivers is outside this proposal.

## Exact checked source identity

For a declared ExistingEntity producer, Data authenticates application ID,
matching Actor definition owner, exact program/effect, context, contribution kind,
unit and destination. Engine then authenticates the real application invocation,
its bounded target, and the exact contribution recipient.

The existing occurrence key remains the source identity:
`application + target + owner + program + concrete entity + effect`. Enemy
baseline contributions cannot masquerade as Encounter contributions or reuse a
Player occurrence merely because their values agree. The same logical Enemy
singleton in another compiled request is distinguished through the existing
request/schema/rule/plan identities; do not add a fake build-local Actor ID.

Keep the existing complete potential-writer census, including zero, inactive,
unknown, unqueried and late effects. Reject missing, extra, duplicate or spoofed
membership. Preserve explicit numeric order and multiplicity where required;
the new origin creates no implicit order relative to Encounter, provider or
application contributions. Equipment slot ranks have no authority for this
origin. Boolean Any, exact integer sum and checked override agreement retain
their separate reduction contracts.

Applicability and owner inventories remain independently complete or Partial.
Binding known programs from a Partial shared Enemy owner must retain the mandatory
gap, even when no requested metric consumes them. Omission of the registry is not
proof that game baseline mechanics are complete. No native admission can be
inferred from PoB's shared initializer name, an observed scalar, or a filtered
modifier query.

## Relationship to the Encounter proposal

The [Encounter/numeric Enemy proposal](owned-encounter-contribution-queries-proposal.md)
is orthogonal and still awaiting review. It names the actual selected Encounter
as a contribution source and permits reviewed numeric queries on Enemy. This
proposal instead binds shared baseline definitions independently of Encounter.

Either contract can be implemented and tested independently. Using both kinds
of numeric suppliers for final physical mitigation needs both decisions: for
example shared Crushed reduction and configured Armour have different origins,
but their consumers use the same checked graph. Numeric Enemy query permissions
must be added once in the shared query validator, not separately by each origin.
Neither decision completes the relevant source inventories or the whole build.

## Alternatives and trade-offs

| Option | Benefit | Cost or limitation |
| --- | --- | --- |
| Generalize explicit existing-entity applicability (recommended) | One shared Enemy baseline owner, exact native identity, existing graph and completeness rules | Requires target-specific validation and current-format origin migration |
| Separate ExistingEnemy registry/origin | Small initial changes to Player code | Duplicates applicability, membership, coverage and binding machinery for the same responsibility |
| Copy baseline programs into each Encounter | Uses today's Encounter binding | Duplicates universal mechanics and ownership; Encounter changes can omit or duplicate baseline effects |
| Add a namespace/global rule owner | Could host common definitions | Adds an owner kind but still needs exact target applicability and closure |
| Fake Enemy Actor or broaden final Stat receivers | Appears to reuse a current binding | Changes runtime identity or conflates source inventories with final calculations; violates existing boundaries |

A bounded origin target costs a small explicit dispatch while preventing broad
entity authority. An unconstrained ConcreteEntity payload would represent more
future targets but would require rejecting most variants immediately; it provides
no needed capability for this decision.

## Implementation and validation after acceptance

1. Update the single current applicability/origin format and all direct callers.
   Rebuild current artifacts and fixtures; add no legacy reader, operation-version
   compatibility branch or second production binding path. Ensure serialized
   rule and compiled-plan identities reflect the new contract.
2. Validate target/context agreement and complete empty declarations. Reject
   duplicate application IDs, duplicate owner/target bindings, missing/foreign
   owners, incompatible mixed targets, missing programs and unreviewed supplied
   invocations. Preserve bounded work and owner counts.
3. Exercise actual Player and Enemy bindings in the same plan. Prove exactly one
   invocation per admitted owner/target, no created Actor/provider/local ID, and
   no dependence on which Actions or metrics are requested. Change Encounter
   selection/level/inputs and verify correct rebinding without duplicating shared
   mechanics; change Player class/loadout without duplicating either baseline.
4. Negatively test Enemy attempts to read Character identity/level, Player slots,
   Player/Actor Stats, arbitrary providers, Skill/Action state or Environment;
   reject all cross-target writes. Preserve the existing Player slot-read checks
   and rejected shared-Player Enemy reads. Tests must cover unused and inactive
   programs as well as demanded reads.
5. Bind real checked shared Enemy contributions. Test true/false/unknown condition
   inputs, zero values, repeated distinct suppliers, missing and spoofed membership,
   wrong recipient/target, duplicate producers, late writes, frozen stages and
   dependency cycles. When numeric Enemy permission is available, combine a
   shared Enemy source with an actual Encounter source in one checked query.
6. Preserve Partial registry/owner/query and global complete-request refusal,
   including empty known program lists. Prove exact fresh evaluation, scratch
   A/B/A reuse and Rayon equality. Share immutable plans/data; add no worker-side
   parsing, source VM, subprocess or mutable global state.
7. Migrate existing Player applications with unchanged numeric behavior and
   accepted final receiver ownership. Publish a separately reviewed shared Enemy
   baseline data inventory with unresolved mechanics left Partial. Preserve all
   five imported builds, existing queries, unrelated source dispositions and
   coverage through the checked publication workflow.

The first Enemy data slice should preserve the actual conditional source model,
including Intimidate non-stacking and Unnerve's Spell applicability. Exact selected
negative domains may be admitted only with checked source/native supplier guards;
the proposed ownership mechanism does not itself establish those negatives or
authorize full physical mitigation.
