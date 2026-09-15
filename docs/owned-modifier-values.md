# Modifier occurrence values

Status: implemented native binding; final item scaling remains incomplete. This is a
consumer-driven extension of the [owned domain architecture](domain-architecture.md),
not a source-object or source-interpreter interface.

## Identity and data flow

Item templates declare their authored inputs. Their EquipmentUse programs derive shared
semantic item properties. A modifier program reads those properties through Current,
reads its own declared rolls, and can derive a typed Stat at relative Modifier scope.
The concrete address is the exact ItemModifier provider: receiving equipment use plus
modifier instance. Two modifiers with the same definition, or one backing item used
twice, have different intermediate values. A later program on that modifier reads the
same Stat through the same scope.

Current continues to mean the receiving EquipmentUse. Modifier is currently a relative
value scope, not a program context; it is valid only for a Modifier definition's
EquipmentUse programs. Compilation rejects other owners, contexts, target types and
units. Binding additionally requires the exact direct ItemModifier root and matching
receiving use. Grant paths cannot borrow their root's modifier value. Existing producer
collision, cycle, activation, resource limits and scratch isolation apply unchanged.
No cross-owner Parameter read or copied mutable item field was introduced.

The wire package remains version 2; the closed operation contract is version 6 and the
prepared effect-plan identity domain is version 5. Offline artifacts are regenerated with
the production compiler. An older operation contract is rejected, not silently upgraded
while loading. Existing definition IDs and fixed/reward program meanings are preserved.

## Catalyst scalar consumer

The injected resistance package now implements an unrounded catalyst scalar. It consumes
the exact item selection/amount, complete required property predicates, and an explicit
unscalable input. Its data defines all 13 selectors, including ANY matching across each
of the four defence properties. Required facts remain unresolved when demanded. Selection
none or no property match does not demand amount; unscalable true returns 1 without
reading selection or amount. The formula preserves `(100 + amount) / 100` grouping.
Ordinary quality remains a distinct input.

This value is an intermediate, not a final resistance contribution. A partially declared
owner may omit other programs, but Partial cannot authorize an individually incorrect
known value. In particular, the catalyst-only amount cannot be contributed as final cold
resistance while magnitude or corruption may still change it. Final contributions must
read a separately resolved effective stage; missing stages remain unresolved, never
inactive, zero, or a guessed default. Final metric and whole-request closure gates are
unchanged.

## Remaining ordered transformation contract

Bind an explicit bounded transform sequence to each recipient modifier. Retain producer
and recipient occurrence keys, applicability, semantic order, and membership completeness.
Resolve each transform's own formatted amount before folding the recipient scalar. Mixed
additive percentages and doubling are not a commutative sum/product reduction. Prefer
cold-plan expansion into the existing typed dependency executor; do not add a second
interpreter or a general collection language without a concrete unmet consumer.

An empty transform sequence is valid only from complete owned membership and explicit
producer declarations. Recompute that result for the current build. A cached import flag
saying no transforms or ready would become stale after edits. The original ring's pending
member list does not currently establish this proof.

Keep the numeric stages separate: source range precision, corrupted-base rounding,
ordered scalar transformation, final scaling/truncation and semantic-unit conversion.
Ordinary canonical ranged source values retain nominal endpoints. Serialized rune values
can be baked. Fixed-text history is also ambiguous. These are importer encoding policies
and reference tests, not native PoB editor state. Source unscalable markers remain outside
the admitted ordinary grammar; advanced-copy control headers are rejected as a whole
source lifecycle. The exact admitted grammar may emit explicit unscalable=false. Broader
marker support needs reviewed metadata conversion distinct from property labels.

## Validation and retirement

Native tests exercise repeated occurrences, item edits and scratch reuse, missing stages,
cycles and duplicate producers, owner/context/type rejection, all persisted selectors,
explicit zero, fractional arithmetic and actual imported ring inputs. An optional PoB
oracle covers 19 ranged/fixed/rune encoding contrasts. Neither component suite is a
complete-build result: original builds remain 0/5.

No legacy numerical caller is replaced by this intermediate alone. The
[retirement inventory](legacy-retirement.md) retains its named consumer gates. Final
resistance preparation, receiver integration and all-five parity must migrate useful
numerical tests before their old dependency closure is deleted. No new Spark/Mace route,
source callback, UI lifecycle or parallel default evaluator was added here.
