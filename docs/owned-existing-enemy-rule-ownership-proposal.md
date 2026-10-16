# Generic Actor identity and shared local mechanics

Status: **accepted 2026-10-15; implemented and locally validated**.
Updated: 2026-10-16. Decider: project owner. Scoped Core/Data/Engine/Import/CLI
validation, strict Clippy, full CLI publication and artifact inverses pass.
Numerical Encounter producer adoption remains separate work.

The owner requested generic actor properties and mechanics across Player,
Enemy and other units wherever their game semantics agree. This proposal
replaces the earlier narrower shared-Enemy binding draft. It retains the
accepted exact source/recipient and final-receiver contracts, and adopts one
current-format identity/context migration rather than a second rule system.
The source/implementation inspection is recorded in
`runs/owned-generic-actor-model-review-01.md`; the relevant findings and complete
migration requirements are retained below.

## Accepted decision

The current scenario opponent now uses the existing Actor identity domain and
one Actor-local mechanical rule context. For the current single-opponent
request, `ActorKey::ScenarioEnemy` replaces the separate concrete Enemy address
with `ConcreteEntity::Actor(ActorKey::ScenarioEnemy)`. This does not add
an arbitrary combatant collection, battle simulator, fake owned-minion provider,
or a second Combatant wrapper. The `ScenarioEnemy` name denotes the absolute,
request-local opponent; it is not a relative opponent lookup.

The existing shared-Actor applicability seam lets the **same authored
local rule inventory** bind Player, the scenario opponent and explicit
already-discovered supplied Actor slots. Keep exact source identity, recipient
identity, explicit applicability and coverage. Split common mechanics from
role-specific baseline data; do not split the common formula by role.

This revises the earlier Enemy proposal. Its separate Player/Enemy program
inventories would prevent copying rules across Encounters, but would still force
copies of the same local mechanic across Actor and Enemy contexts. It is too
narrow for the owner's direction. The owner approved the unified identity,
local-rule and role-authority migration as one structural change.

## What the source actually establishes

The complete pinned `CalcSetup.lua`, `CalcPerform.lua` and `CalcDefence.lua` bodies
were read from the `player-evasion-final/source-domain.json` archive references;
their decompressed SHA-256 values were checked against that existing manifest.
These are PoB reference design facts, not proof of the game's internal engine:

- CalcSetup 25-26 explicitly initializes stats and conditions common to all
  actors. The same function is called for Player at 835, Enemy at 903, and minions
  in CalcPerform 1059. It includes Maimed, Intimidated, Unnerved, Debilitated,
  Blind and Crushed, among other mechanics (CalcSetup 70-106).
- CalcSetup 752-766 creates Player and Enemy as actor-shaped objects with their
  own level and modifier store, then binds their opponent relationships. Generic
  EnemyModifier delivery follows `actor.enemy` (CalcPerform 563-578).
- Player-specific Life/Mana/Evasion sources and Tailwind records are added
  separately (CalcSetup 830-899); Enemy accuracy is separate (903-905); minion
  Life has its own profile and level table (CalcPerform 1057-1066). Equal property
  names do not imply equal base values or availability.
- CalcDefence 789 accepts an actor; its Player/minion call sites are CalcPerform
  3430 and 3495. That is evidence for reusable mechanics, not authority to apply
  every complete Player defence algorithm to Enemy unchanged. Role branches and
  missing source domains must still be checked.

PoB also distinguishes relative opponents from an absolute scenario Enemy.
Current native `RuleEntity::Enemy` is absolute. Do not silently reinterpret it as
"this actor's opponent" during this migration. A future opponent/owner relation
must be explicit and checked when a mechanic actually needs it.

## Pre-migration contracts and the reason a rename was insufficient

| Contract | Pre-migration implementation | Required treatment |
| --- | --- | --- |
| Actor identity | `owned_build/records.rs`: Player or provider-supplied Owned Actor; `ActorOccurrence` mirrors this | Add the real scenario singleton, with explicit resolver behavior; it has no build provider |
| Scenario ownership | `ScenarioInput.enemy` contains Encounter and level | Keep this data owner; resolve it to the new Actor address, not Character or an invented supply |
| Mechanical target type | `RuleEntityKind::Enemy` is distinct from Actor in Stat/Capability targets and program contexts | Replace Enemy mechanical context with Actor; preserve role eligibility through checked binding authority |
| Runtime values | `ConcreteEntity::Enemy` owns separate value/contribution keys | Use the one canonical Actor address in values, contributions, origins, applications and stage channels |
| Relative addresses | Actor, Player and Enemy are different `RuleEntity` selectors | Actor/Current become genuinely local; explicit Player/Enemy selectors remain role-addressed reads subject to permissions |
| Shared rule ownership | `ExistingActorRuleTarget` only Player; Engine creates one Player invocation | Extend the existing registry in place to explicit Enemy and supplied-slot applicability; bind each full owner inventory once per actual target |
| Contribution membership | ExistingActor membership checks Player; Enemy queries permit only Flags | Authenticate the exact generalized Actor invocation and recipient; use the existing numeric/Boolean query contracts on any admitted Actor |
| External assumptions | Enemy has its own target kind/address | Canonicalize the runtime address and retain role applicability; simply changing Enemy-only inputs to unrestricted Actor inputs would broaden authority |
| Metrics | Player/Owned role and provider-origin assumptions | Handle Enemy explicitly, without classifying it as Owned or Character; do not automatically expose unimplemented Enemy metrics |

Relevant implementation points:

- Core: `owned_build/records.rs`, `owned_draft/records.rs`,
  `owned_binding/occurrence.rs`, `owned_binding/selectors.rs`,
  `owned_schema.rs`, `owned_rules.rs`, `owned_metrics.rs`.
- Data: `owned_rules/existing_actors.rs`, `owned_rules/ordered.rs`,
  `owned_rules/applications.rs`, receiver validators, `owned_stages.rs`, and
  external-input/metric binding validation.
- Engine: `owned_plan.rs`, `owned_plan/compile.rs`,
  `owned_plan/compile/{ordered,reads,effect_applications,equipment_slots}.rs`,
  `owned_plan/metrics.rs`, and `owned_rules/compile.rs`.

Before migration, `RuleEntityKind::Enemy` was both a semantic value target and an
execution/permission boundary. Removing it therefore required replacing its
authority checks, not merely renaming a type. The current Actor model checks
the exact bound role and source permissions separately from mechanical typing.

The former Enemy/Environment contexts supplied a placeholder `ActorKey::Player`;
the old Enemy permission boundary fenced that fallback. Engine `Context.actor`
now represents absence explicitly, and scenario-Enemy contexts carry their real
Actor identity. Core occurrence resolution is exhaustive; a non-Owned key no
longer falls through to Player.

## Reuse mechanics, retain authority

Use the existing exact applicability/source validators, extended for the actual
targets. No general inheritance hierarchy, dynamic trait system, Lua-style
object model or permissive all-entities registry is needed.

1. A common baseline owner can use local `Current`/`Actor` typed Stats,
   capabilities, conditions and checked contributions. It can contribute/derive
   only on that bound Actor. Actor/Action/Equipment/Skill remain different entity
   kinds. Applications keep their distinct exact source and recipient.
2. Apply that owner to explicit Player/Enemy selectors and explicit supplied
   Actor slot selectors. Instantiate only discovered supplied occurrences; never
   create an Actor because a rule or query mentions its slot. Forbid duplicate
   `(owner, concrete Actor)` invocations and keep complete/Partial applicability
   independent of each owner's complete/Partial rule inventory.
3. Player-specific baseline owners retain Character/class/ascendancy and Player
   equipment authority. A shared Enemy binding does not gain those reads. Encounter
   programs retain selected-Encounter authority and the actual Enemy input
   bindings. Supplied Actors retain their checked provider/parent relation.
   Validate each whole bound inventory, including unused programs; do not rely
   on a runtime branch being false to hide an unauthorized read.
   Existing explicit absolute CharacterClassIs/CharacterAscendancyIs reads by
   Encounter programs remain authorized under their source contract; they do
   not acquire that authority from the local Actor. Likewise the explicit
   EnemyLevel selector retains its existing meaning, never a generic ActorLevel.
4. Common formulas consume local semantic values. A Player producer may read
   CharacterLevel, an Encounter producer EnemyLevel, and a supplied-Actor producer
   its exact projected level. They can provide a common local input when the
   mechanic needs one. Do not redefine CharacterLevel as generic ActorLevel or
   copy Player base values onto other actors.
5. Absolute cross-Actor reads and writes remain separately authorized. Generic
   Actor target typing alone does not authorize Enemy-to-Player reads, Player
   equipment reads, support inheritance or arbitrary recipient changes. Retain
   the accepted Player Stat receiver ownership for incoming-critical results.
   Reusable local formulas do not move those results back into baseline owners.
6. Keep integer ordering, Any reduction, numeric selection, unknown coverage,
   inactive/zero potential writers, stage freezes and late-writer checks exactly
   as explicit as today. Unknown Enemy conditions must not become false merely
   because no producer has yet been ported.

The same Stat should be reused across actors only when its semantic value and
unit are the same. Different role-specific inputs/profiles may feed it. Similar
labels are insufficient reason to merge two channels with different meanings,
applicability or calculation stages.

## Encounter ownership within the accepted migration

This migration absorbs the **numeric Enemy query** part: Enemy becomes an exact
Actor recipient, so it uses the existing checked Actor numeric, Boolean and
optional-selection machinery. The implemented checks retain exact role/source
authority without an Enemy-only query bypass. Enemy-specific source gaps remain
blocking.

The **Encounter producer origin remains necessary and distinct**. Configured
Armour `31e0` is owned by the selected Encounter; shared Crushed/Intimidated rules
are owned by their shared applicability. Identity unification must not disguise
one as the other. Checked Encounter origin membership now authenticates the exact
selected Encounter/program/effect and scenario-Enemy Actor recipient. This is complementary to
shared baseline ownership, even though both feed Actor channels.

## Alternatives

| Option | Assessment |
| --- | --- |
| Keep separate Enemy identity and add generic local-actor dispatch over Actor-or-Enemy | Can share formulas if every type, read and query uses that abstraction; however, it introduces a second sum-type/adapter and retains two identities for the same mechanical category. More ongoing special cases than the current singleton requires. |
| Unify Enemy into ActorKey, retain bounded role/source permissions | Accepted and implemented. Preserves one mechanical identity/context and existing graph. No broad simulation feature is required. |
| Add only Enemy shared applicability with separate contexts | Small ownership extension, but duplicates common formulas and condition definitions across roles. Does not satisfy the new direction. |
| Make every unit a provider-supplied Owned Actor | Incorrect. Scenario Enemy is not supplied by the Player build and must not inherit its provider permissions or lifecycle. |

## Implemented migration requirements

Make one reviewed current-format migration; do not retain aliases, two Enemy
addresses, old-format readers or a parallel evaluator.

1. Introduce the actual Enemy Actor address and exhaustive Core/draft/resolver
   handling. Remove non-Owned-is-Player fallthrough and no-Actor Player fallback.
   Keep the single `ScenarioInput.enemy`; do not add arbitrary enemies/actions.
2. Migrate Engine identity/keys and mechanical Actor contexts; replace old Enemy
   Stat/stage/application targets. Preserve role-restricted inputs and source
   capabilities explicitly. Audit metrics, usage and provider-only APIs so Enemy
   is rejected where unsupported instead of falling into Owned/Player behavior.
3. Extend shared applicability and exact contribution membership, and add the
   selected Encounter origin. Share common local formulas; retain separate
   Player, monster and supplied-Actor profile owners. Keep current Player-only
   final receivers Player-only unless separately admitted.
4. Rewrite current authored packets/drafts and regenerate current packages and
   fixtures. Original PoB source archives and observations remain unchanged.
   Hashes/serialized identities will intentionally change; numerical behavior
   for already admitted domains must not.
5. Prove one identical authored local rule on Player, two distinct supplied Actor
   occurrences, and scenario Enemy with independent inputs/results/provenance.
   Include source reordering, duplicate applicability, forged source/recipient,
   missing/unknown/zero/inactive and late-writer controls. Test explicit denial
   of Enemy access to Player gear/class and unrelated supplied-Actor state.
6. Prove the old alias/fallback cannot be deserialized or resolved, Enemy inputs
   cannot be assigned to Player, and unsupported Enemy metrics/actions fail
   explicitly. Preserve all current Partial and whole-build gates, incoming-
   critical Player receivers, deterministic fresh/reused/Rayon evaluations and
   all-five original imports. This change is structural, not new build coverage.

Current packages use the unified identity throughout; no intermediate package
mixes the old Enemy identity with generic Actor permissions. Numerical Enemy
mitigation and additional shared conditions still require their own admission.

## Current-format contract details

- `ActorKey` and `DraftActorKey` use `{"kind":"scenario_enemy"}`. Assumptions
  use `{"kind":"actor","value":{"kind":"scenario_enemy"}}`; remove the old
  separate Enemy assumption address.
- External-input eligibility is explicitly `player_actor`, `owned_actor` or
  `scenario_enemy_actor`. Migrate previous Actor eligibility to Player and Owned,
  and previous Enemy eligibility only to ScenarioEnemy. Mechanical Actor typing
  alone must not widen input authority.
- Shared applicability adds `scenario_enemy` and an explicit `owned_slot`
  selector to existing `player`. Encounter contribution membership uses
  `{"kind":"encounter"}` and authenticates the member's selected Encounter owner.
- Metric roles handle ScenarioEnemy explicitly without adding metric definitions
  or inventing a provider. Existing Player final receivers remain Player-only.
  Unsupported Enemy usage, support reception, receivers and Actions fail explicitly.
- Runtime contexts without an Actor store absence; contexts with an Actor store
  its real identity. No Enemy or Environment context may borrow Player identity.
- Validate role authority over each complete bound inventory, including unused
  programs and branches. The absolute `RuleEntity::Enemy` selector remains intact.
- Current schema packages use version 7 and rule packages version 4; current
  full operations use V28. Older operation capability subsets can still restrict
  a new-format package, but every subset has the same unified Actor identity.
  Historical schema-upgrade authoring branches are removed; one current migration
  shape retains declaration changes, allocation checks, capability upgrade
  checks and evaluation-artifact preservation.

## Implementation and validation status

The Evasion slot/override publication was frozen before runtime migration.
Core/Data/Engine, current packages and the portable replay now use the unified
contract. Core has 312 existing and 25 focused passing tests; Data has 205;
Engine has 718 passing tests and seven intentional ignored tests. Strict Engine
Clippy passes for the library and all 73 owned integration targets; Core/Data
strict checks also pass.

The exact package/replay inverses preserve mechanics. All five imports retain
110 queries and unresolved counts of 106/117/109/123/4. Scoped Import/CLI
regressions, catalog reproduction, the full CLI publication chain and strict
Clippy pass. The [implementation plan](implementation.md) records exact scopes,
logs and the separate hosted-CI status. No compatibility reader or old Enemy
identity path remains.

This completes the structural contract, not its next numerical adoption.
Binding real configured Critical Weakness and Armour suppliers into checked
consumers remains tracked in the [Encounter query proposal](owned-encounter-contribution-queries-proposal.md).
Existing Partial inventories, Player-only final receivers and the
**0/5 complete-build** count remain unchanged.
