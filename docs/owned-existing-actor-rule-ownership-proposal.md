# Proposal: rules owned by an existing actor

Status: **accepted; Player contract and first game-data migration published**.
Date: 2026-10-06. Core has 1 passing contract test, Data 5 passing validation
tests, and Engine 7 passing binding/evaluation tests. The checked
[Player ownership packet](../data/owned/poe2/3887ae68/player-rule-ownership/README.md)
removes intrinsic Player Life from all eight Class owners and installs it once
on shared Actor `332a`. That Actor retains
`shared-player-initialization-not-converted`; class base attributes, unarmed
facts and Class Partial coverage are unchanged. Publication passed in 26.27
seconds; four native migration tests passed in 4.25 seconds. Complete native
evaluations remain **0/5**. The [implementation plan](implementation.md) and
packet README record the active receipt and reproduction inputs.

The original ownership audit below used `runs/owned-attribute-step-02/package`,
input `268605405d639a0a84e67391ba7e0d50177447d2cc4e1a275786f0c053bd4f8b`;
that is an evidence baseline, not a claim that it remains the active endpoint.

## Accepted decision

Reuse `ActorDefId`, `ActorSchema` and ordinary `DefinitionRules` for shared actor
mechanics. Add an explicit, injected applicability inventory that binds a rule
owner to an **already existing** Player or a reviewed owned-actor target, exactly
once per concrete actor. Keep class-specific facts on Class owners and keep
stat-owned receivers restricted to their existing final-value role.

The implemented target domain is Player only. The published migration moves
the intrinsic Player Life contribution from the eight Class owners into one
shared Player rule owner. The numeric body is preserved; its target is now the
current Actor. Remaining shared initialization stays explicitly Partial. Closing
that inventory is separate work.

The public binding and coverage change is implemented in the single current
format. No new operation-version branch or compatibility implementation was
introduced; applicability is injected data, and affected fixtures are updated
in place. The owner has clarified that backward compatibility is unnecessary.
Regenerate affected artifacts instead of retaining replaced production paths.
Retain useful independent reference evidence and named historical receipts without preserving
obsolete executable paths solely for their old serialization.

## Historical ownership audit

Before this migration, Original05's Sorceress Class `0a23` had these three
implemented programs. Only intrinsic Player Life has moved to Actor `332a`.

| Program | Output at the audit baseline |
| --- | --- |
| `class-base-contributions` | Class-specific 7 Strength, 7 Dexterity, 15 Intelligence; one calculation emits to both Count passes `331b`–`3320` |
| `intrinsic-attack-baseline` | Class unarmed inputs `1d35`–`1d38` |
| `intrinsic-player-life` | `12 * character_level + 16` as an Add contribution to Actor Life `311a`, unit `3119` |

All seven Class declaration inventories are already Complete and empty; its
level domain, ascendancy membership and implicit-root relation are explicit.
The shared root `1790`/source `54447` is now complete for its default intrinsic
inventory. The Class's only remaining rule marker is the generic
`tree-game-rules-not-converted`, originally assigned by
`crates/poe-optimizer-import/src/owned_tree_catalog.rs:1103`–`1111`.

At the audit baseline, no separately bound owner accounted for unfinished
shared Player initialization. The migration resolves that ownership through
Partial Actor `332a`; it does not remove the Class marker merely because its
known programs exist. Class-specific closure still needs its own complete
inventory proof. Treating shared defaults as Sorceress mechanics would duplicate
rules across classes and obscure the completeness boundary.

The pinned source separates these responsibilities:

| Source location in `vendor/path-of-building-poe2/src/Modules` | Responsibility |
| --- | --- |
| `CalcSetup.lua:827`–`832` | Class-specific base attributes |
| `CalcSetup.lua:1853`, `1861`; `CalcPerform.lua:3230`; `CalcOffence.lua:2529`, `2559` | Class unarmed facts selected by ordinary, disabled-weapon or skill-specific attack-source policies |
| `CalcSetup.lua:26`–`114`, called at `835` | Shared actor caps, limits, status-dependent modifiers and reference-mode conditions |
| `CalcSetup.lua:834`–`899` | Player level and resource baselines, resistance configuration, combat modifiers, limits and other shared Player rules |

The shared initializer includes Mana, regeneration, Evasion, Accuracy, critical
damage, limits and conditional modifiers. It contains no further Strength,
Dexterity or Intelligence contribution beyond the class-base loop. Some source
initialization is reference machinery: `Buffed`, `Combat` and `Effective` at
lines 111–113 derive from calculation/display scope. Those source flags must not
be imported automatically as gameplay conditions. Each migrated mechanic needs
its own semantic classification and evidence.

Existing native channels already separate final attributes `1d2e`–`1d30`, six
Count inputs `331b`–`3320`, six ordinary attribute receivers, inherent-bonus
control Booleans `3315`–`3319`, derived Strength Life amount `331a`, and canonical
Life `311a`. This proposal supplies ownership; it does not invent defaults for
those missing controls, complete contributor groups or final Life.

## Boundaries retained by the implementation

`CharacterSpec` contains class, ascendancy, level and rewards
(`owned_build/records.rs:81`–`87`). Character provider discovery binds Class,
Ascendancy and implicit passive owners
(`owned_binding/selectors.rs:363`–`392`). Existing-actor applicability is bound
separately by the rule planner; it does not change the Character provider into
a universal Player owner.

`ActorSchema` already owns ordinary declarations and rules
(`owned_schema.rs:307`–`309`). For a supplied actor,
`ActorSlotSchema.provider_definition` names an Actor definition
(`owned_schema.rs:446`–`450`). That relation is followed through an actual
`GrantTarget::Actor`, producing a distinct `ActorKey::Owned`
(`owned_binding/selectors.rs:661`–`702`). That supply relation does not attach
rules to `ActorKey::Player`; the implemented applicability relation reuses the
existing Player directly. A fabricated grant would create the wrong actor and
provider ancestry.

Stat receivers already bind calculations to existing actors. Their validator
requires exactly one final `Derive` to the receiver's own Stat
(`crates/poe-optimizer-data/src/owned_rules.rs:815`–`822`). Widening them to run
arbitrary contribution inventories would merge two different responsibilities:
the sources of an actor's mechanics and the consumer of a final statistic.

Whole-owner coverage is deliberately conservative. Engine provider discovery
records `PartialPrograms` before instantiating a Partial owner's known programs
(`owned_plan/compile.rs:1348`–`1374`). The new owner must participate in those
same completeness checks, not provide an alternate partial-evaluation route.

## Implemented contract boundary

`RulePackageInput` carries this applicability inventory. The registry belongs
to the injected rule package, not the user-authored build or a Rust game switch.

```rust
pub existing_actor_rules: Option<DeclaredSet<ExistingActorRuleApplication>>;

pub struct ExistingActorRuleApplication {
    pub id: OwnedDefinitionKey,
    pub owner: ActorDefId,
    pub targets: Vec<ExistingActorRuleTarget>,
}

pub enum ExistingActorRuleTarget {
    Player,
}
```

The accepted current-format envelope allows `None` for a deliberately empty
applicability inventory; explicit null is rejected and omission serializes no
extra field. An actual game ownership migration must install its declared
applications and preserve unresolved shared coverage. Omission is not evidence
that PoE initialization is complete.

Each row applies the owner's complete declared program inventory. It does not
select convenient individual programs from a Partial owner. The validator
requires all seven declaration inventories to be Complete and empty: parameters,
choices, grants, actors, skill grants, outputs and sockets. The published shared
owner needs no new authored build inputs or generated topology. Unsupported
declaration or grant authority is rejected. Reusing the Actor definition type does not grant every actor-supply capability automatically.

Cold binding records `RuleOrigin::ExistingActor` with the applicability ID and
exact `ActorKey`, alongside the ordinary program owner identity. It binds Player
without allocating a new build-local ID, changing the Class provider or
fabricating a supply path. Reuse the current
actor context, typed reads, program compiler, dependency DAG, effect storage and
worker scratch. Shared rules can contribute or derive within their admitted actor
context; they do not gain arbitrary access to another provider's parameters.

The checked registry rejects duplicate applications of an owner to Player,
including duplicates under different application IDs. Missing definitions or
rule owners and unsupported targets are rejected. Partial applicability records
`PartialExistingActorRules`; a Partial owner records `PartialPrograms`, including
when its known program list is empty. Both preserve whole-plan refusal. Future
owned-actor targets must also prove that applicability cannot duplicate an
existing supply-bound invocation. No first-match selection, inherited Player
default for minions, or generic all-actors fallback is implied.

For later owned-actor targets, reuse actual discovered occurrences and their
ancestor/readiness/activation gates. False activation makes their rules inactive;
unknown activation stays unresolved. Repeated copies of a minion receive distinct
invocations. No applicability entry may create an actor that is absent from the
bound build. Shared all-actor laws can be reused through explicitly reviewed
targets or profiles; PoB's shared function name alone does not prove that every
native actor type should receive them.

## Dependencies, contributors and coverage

Register shared contributions and final producers through the existing graph.
Reject duplicate final producers and cycles exactly as for current provider and
receiver programs. Class changes must not duplicate Player-wide Life; actor
rules that read final attributes must preserve their real dependency rather than
running in an implicit initialization order.

The readiness and contribution-stage inventories must classify the new invocation
origin explicitly. The ordered-query framework currently accepts a bounded set
of provider origins. The first Life migration requires no new ordering policy;
an ordered query that cannot admit the new origin must keep rejecting it. Do not
pretend it originated from a Class to satisfy an existing ordering arm.

Account for shared rules with explicit Partial inventories: Player-specific
initialization and truly shared actor mechanics may have distinct owners. Every
source initializer row needs a disposition: implemented owner/program, existing
separate policy, unresolved mechanic, or independently justified reference-only
behavior. Empty output in one selected build is not an exclusion proof.

After this ownership exists, a Class can be reviewed against its actual
class-specific inventory while unfinished shared mechanics keep their own
mandatory gaps. Moving a marker alone does not complete a class. Independent
evidence must also separate unarmed numeric facts from action/equipment
selection, and preserve all their unresolved conditions and replacements.

Attribute contribution membership can be investigated and published while Class
remains Partial. It must be proved from admitted source mechanics, not just the
currently converted writers. Current Count data has 368 owners/programs: eight
Partial Classes, 356 Complete passive owners and four Partial modifier owners.
Other originals include additional passive and unadmitted item attributes.
Complete group metadata would not bypass any Class, modifier, input or whole-plan
gap; it would remain component progress until those obligations close.

## Alternatives and trade-offs

| Option | Advantage | Cost and consequence |
| --- | --- | --- |
| **Actor definitions plus explicit applicability — recommended** | Reuses typed owners, rule compilation and per-actor identity; shared mechanics live once; preserves Class and final-Stat boundaries | Adds a binding/coverage contract and explicit origin; requires completeness, activation and dependency tests |
| Copy shared programs onto every Class | Uses today's bindings immediately | Repeats data and coverage; a class change is entangled with universal mechanics; does not address shared minion/enemy laws |
| Add a namespace-wide/global rule owner | Makes one common inventory easy to find | Introduces a new owner category and still needs per-actor applicability, ancestry and closure semantics; namespace presence is not an actor occurrence |
| Widen stat receivers to contributions and multiple effects | Reuses their target-selection path | Blurs source ownership with final aggregation, complicates duplicate producer and contributor inventories, and weakens the intentionally narrow receiver contract |
| Create a synthetic Player actor grant/profile through existing supply | Appears to avoid a contract extension | Creates an owned child actor instead of binding the actual Player, changes identity/ancestry, and gives misleading grant semantics |

The recommended option adds only the missing applicability seam. It does not
require a new interpreter, general inheritance, a database, a second scheduler
or engine-level knowledge of PoE classes and initializer names.

## Validated contract and published game-data milestone

The `owned_existing_actor_rules` targets passed in Core (1 test), Data (5 tests)
and Engine (7 tests). They cover serialization and explicit-null rejection,
duplicate/missing owners and targets, illegal declarations and provider reads,
Partial registry/owner refusal, bounded owner/work accounting, existing-graph
producer collisions and cycles, and unsupported ordered-origin admission. Engine
tests reuse the actual Player across class and loadout changes and prove exact
scratch A/B/A and four-worker replay. These framework tests are separate from
the four native tests of the published ownership packet. Those migration tests
use all five selected Character inputs, all eight class choices and source level
controls; they verify exactly one shared effect, actual Actor/Class Partial
refusal, scratch reuse and Rayon execution. Their finite intrinsic component
does not establish broader shared initialization or complete builds.

The completed packet:

1. Installs one complete Player applicability row for Actor `332a`, using the
   validated current contract without a compatibility switch.
2. Removes `intrinsic-player-life` from all eight Class owners and runs the same
   numerical body once on the existing Player. Canonical channel `311a`, Life
   unit `3119`, Character level authority and the admitted level domain remain
   unchanged; the target spelling is generalized from Player to Current.
3. Preserves unfinished initialization in the Actor's Partial inventory, and
   leaves every Class Partial with its base attributes and unarmed facts intact.
   It adds no final Life, regeneration or inherent-control defaults.
4. Authenticates the exact whole-recipe change and inverse. The old eight
   class-owned bodies and the new shared body cannot both run. Publication
   preserves all five drafts, selections and 110 queries and rebuilds all
   eighteen runtime artifacts exactly.

The existing Player Life witness and packet are reference inputs:
`data/owned/poe2/3887ae68/player-intrinsic-life/` and
`runs/owned-player-intrinsic-life-source-02/`. They establish a numerical slice,
not all shared initialization. Reuse their independent original-call evidence
and retained warm/fresh distinctions instead of writing a second copied formula
as the oracle.

Retained proof gates for subsequent changes:

- Match original source intrinsic Life for all five real builds and the reviewed
  level controls. Exercise every supported class, including changes between
  classes, with exactly one shared Player effect and no class-owned duplicate.
- Preserve base attribute and intrinsic attack effects byte-for-byte apart from
  authenticated identity rebinding. Preserve original input issues and all query
  availability states; complete native builds need not increase at this step.
- Verify Partial applicability, missing/Partial owner, duplicates, illegal
  declaration authority, unsupported targets and duplicate final producers fail.
  Preserve whole-plan refusal even when a particular requested output would not
  read shared Life.
- Prove dependency cycles and unsupported ordered-origin use remain errors.
  Before enabling owned-actor targets, cover two real repeated occurrences,
  inactive and unresolved ancestors, and readiness requirements separately.
- Require exact fresh replay, scratch A/B/A reuse and concurrent Rayon results.
  Binding happens once during candidate compilation; evaluation needs no Lua,
  subprocess, shared mutable actor state or per-worker data parsing.

The **Actor-definition/applicability ownership model** is accepted and its
Player contract and first ownership migration are validated and published.
Shared initialization completion, source-mode interpretation and any extension
to owned actors remain evidence-driven implementation work.
