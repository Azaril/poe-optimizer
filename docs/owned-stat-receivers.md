# Stat-owned actor receivers

**Status: component implemented, 2026-09-15.** This is the common actor calculation
boundary within the [owned domain architecture](domain-architecture.md). Complete-request
and global contributor-closure requirements remain unchanged. Real game receiver data
and complete original-build evaluation are separate work; see
[implementation](implementation.md) for validation and the current resume point.

Common final statistics need an owner independent of class, equipment, encounter and user
usage choices. An explicit stat-owned receiver can combine contributions for a concrete
actor using the existing typed rule DAG and dependency scheduler. Player and owned-actor
formulas have explicit applicability; a missing owned-actor recipe never inherits player
defaults. No source callback, UI object or fabricated provider is involved.

## Contract

Rule-package wire version 2 requires an explicit receiver registry:

```rust
pub receivers: DeclaredSet<ActorStatReceiver>;

pub struct ActorStatReceiver {
    pub id: OwnedDefinitionKey,
    pub stat: StatDefId,
    pub program: OwnedDefinitionKey,
    pub targets: Vec<ActorReceiverTarget>,
}
pub enum ActorReceiverTarget {
    Player,
    OwnedSlot { slot: DeclaredSlot<ActorSlotDefId> },
}
```

The program remains in existing `DefinitionRules`, owned by the receiver's `StatDefId`.
Stat owners already support compiled programs without authored parameter or grant ports.
A receiver row selects where that program runs; it does not create an actor occurrence.
One row can name several exact actor slots without copying its program. Distinct player
and owned-actor formulas use distinct program IDs. A generic all-actors fallback is outside
this first contract.

Validate unique receiver IDs and `(stat, program)` references, nonempty duplicate-free
targets, Known exact schema/namespace references, an existing program and Actor context.
Initially require one final `Derive` to the owner stat on `Current` or `Actor`; intermediate
values remain DAG nodes. Separate final channels use their own receivers and explicit
`Stat` dependencies. Do not add grant creation, parameter projection or cross-actor writes
to this receiver boundary. Existing provider programs retain their existing effects.

Reads reuse typed contributions, stats, capabilities and explicit external inputs. They do
not acquire another owner's item/gem parameters or choices. `CharacterLevel` still means
the player character's level. An owned actor's effective level must arrive through an
explicit actor-stat dependency or projection.

## Occurrence, activation and coverage

After provider discovery, instantiate applicable receivers once per actual `ActorKey`,
including Player. Reuse distinct owned-actor identities even when their definitions or
supplying gem records match. Discover receivers independently of query order, since another
stat or action may depend on their outputs. An applicability declaration alone must not
create an actor or adopt a provider.

Give receiver invocations a distinct origin `RuleOrigin::Receiver { receiver, actor }`.
Reuse actor-query context resolution: retain the parent provider, ancestry, possible
generated Skill exposure and exact actor grant. A context containing only an actor key
would lose supplying Skill required-input readiness and ancestor activation. False grants
gate child receivers inactive; missing grants or required inputs remain unresolved. A
parent projection may remain Known diagnostic evidence without authorizing a child metric.

Register receiver outputs with ordinary final producers before resolving reads. Normalize
`Current`/`Actor` aliases and reject competing producers, including a receiver and provider
writing the same final stat. Do not select the first recipe or invent override priority.
A receiver can reduce its own incoming contributions; reading its own final value creates
a cycle. Activation/final-stat cycles require explicit future semantics, not repeated
execution until values settle.

A complete empty receiver registry supplies no final producers; missing requested stats
remain `MissingProducer`. A Partial registry adds a `PartialReceivers` coverage gap even if known roots
run. Program and contributor membership retain their existing closure checks. This boundary
does not introduce partial-draft evaluation or demand-local completeness. A Known component
value is not automatically an available final metric.

## Limits and identities

Bound receiver, target and gap counts, schema lookups and matching work, including unused
rows. Compile an applicability index once and charge actor/path expansion before allocation.
Use existing invocation, effect, edge and work ceilings for instantiated roots. Plans and
programs remain immutable; each worker owns its scratch state.

Receiver declarations participate in canonical rule-package identity. Storage uses
`owned-rule-package-v2`; compiled input/program domains use `owned-rule-input-v2` and
`owned-rule-programs-v2`; effect plans use `owned-effect-plan-v4`. Numerical operations
remain v5. Old wire versions and a missing registry reject explicitly. Receiver rows and
targets canonicalize before final identities; effect ordering remains meaningful.

Base, import seed, CLI-published catalog and resistance artifacts have migrated with
complete-empty receiver registries. Schemas, IDs and existing formulas are preserved.
An empty registry supplies no game receivers. New real recipes must explicitly declare
reviewed programs and their coverage.

## Why this extension

| Option | Assessment |
| --- | --- |
| Explicit stat-owned receiver roots | Reuses semantic IDs, typed programs and the occurrence graph; adds applicability and instantiation metadata. Recommended first slice. |
| Run every stat program on every actor | Implicit applicability would apply player defaults to owned actors and activate unused programs. |
| Generic module definitions and arbitrary input bindings | May help larger multi-output families later, but adds identities, ports and binding contracts before they are needed. |
| Class, Encounter or artificial UsagePolicy ownership | Duplicates shared formulas or makes unrelated selections determine common calculation behavior. |
| Hard-coded resistance receiver | Hides game constants and dependencies in dispatch. A pure numeric kernel remains useful beneath injected semantic recipes. |

## First receiver and acceptance

Use player cold resistance to exercise the boundary with actual item/reward/scenario
contributions and the [metric layer](owned-metrics.md). The
[resistance data slice](../data/owned/poe2/3887ae68/resistance/README.md) supplies partial
contribution recipes; it currently supplies no final receiver.

Keep cold and all-elemental contribution buckets explicit. The ordinary formula combines
BASE with `max((1 + INC/100) * MORE, 0)`, truncates toward zero, then caps and floors the
selected result. Player base maximum 75, general cap 90 and floor -200 come from reviewed
injected source data, not Rust constants or universal owned-actor defaults. A numeric zero
override differs from no override; maximum overrides can bypass the ordinary cap. Override
presence/priority, conversions, block-related maxima and actor-specific inputs need explicit
semantics or coverage gaps. Missing producers cannot prove that those mechanics are absent.
Existing typed arithmetic and the pure resistance kernel provide reusable numerical laws.

The component acceptance suite covers:

- Direct-authored contributions through receivers to real metric queries, changed input,
  and equivalent facts across classes without class-specific receiver dispatch.
- Player and two same-slot owned actors with different formulas/values; an unlisted actor
  has no fallback. False/missing grants and missing/unsupported supplying Skill inputs
  block the child while preserving parent projection diagnostics.
- Complete-empty versus Partial receiver/program/contribution membership, duplicate final
  producers and aliases, valid derived dependencies, and direct/indirect cycles.
- Strict wire/version/reference/type/unit validation, budgets, canonical identity,
  A→B→A scratch reuse and shared immutable worker execution.
Real cold-resistance comparisons remain an integration obligation after selected
contributions and global closure are established. Existing numerical source tests alone
do not establish the new receiver's whole-build fidelity.

Receiver ownership alone does not complete source normalization, full metric coverage or
legacy retirement. Preserve all five originals and fixed expectations. Replace active
legacy consumers only after their selected-occurrence and independent numerical laws move
to the owned path; see the [migration plan](architecture-migration.md).
