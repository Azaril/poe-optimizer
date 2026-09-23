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

This seam was introduced with rule wire version 2, operation contract v6 and prepared
effect-plan identity v5. The current package uses operations v10 for ordered transforms;
explicitly supported older contracts retain their semantics. Offline artifacts are
regenerated with the production compiler; loading never silently repairs or upgrades
unsupported contracts. Existing definition IDs and program meanings are preserved.

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

## Ordinary local weapon nominal values

The [authored local modifier package](../data/owned/poe2/3887ae68/local-modifier-inputs/README.md)
adds shared nominal percent/minimum/maximum channels for nine weapon-local families.
Its 23 explicit grammars carry typed roll and property facts. Three increase/reduced
families apply the `reduced` Boolean after nominal interpolation, avoiding reversed
range endpoints. Explicit fractured/desecrated flags remain metadata facts; no source
flag or text parser is executed by the runtime recipes.

These Modifier-scoped outputs do not contribute to final weapon totals. Source encoding,
numeric-component scaling and effective magnitudes each remain visible gaps. Matching
text on an amulet or jewel does not acquire weapon-local meaning, and Partial template
membership cannot certify a complete transform sequence.

## Ordered transformation contract

Operation set v10 adds `ProjectModifierTransform` and `ModifierTransforms` to the existing
native rule/plan executor. Producers declare a factor channel, finite recipient definitions,
optional recipient Boolean predicates, a nonnegative step and an Add/Multiply operation.
The recipient read starts from its own explicitly derived factor stat. Both operations
require the same dimensionless-factor unit; this is not general collection execution.

Cold binding expands direct ItemModifier producers onto matching sibling occurrences on
the same backing item and receiving equipment use. It sorts by `ItemRecord.modifier_order`,
then by the producer's explicit step. Source/recipient activation and recipient predicates
are ordinary dependencies. Duplicate positions, missing facts, cycles, incompatible scopes
and units are rejected or unresolved through the existing contracts. Target, effect, edge
and work budgets apply before expansion. Separate receiving uses retain separate values.

The fold preserves interleaved arithmetic: starting at 1, add 0.2 then multiply by 2 yields
2.4; reversing the order yields 2.2. Producer record order, IDs and program discovery are
not semantic order. The producer amount is its pre-magnitude formatted value; reading its
own final transformed value would invent amplification and dependencies absent in the source.
The executor uses its existing checked arithmetic and per-worker scratch, with no PoB VM.

Only globally complete owned membership permits a known fold, including an empty sequence.
Empty preserves the explicit initial stat; unknown membership, predicates or initial values
never imply an identity. Candidate edits rebind the sequence. Versions v6-v9 retain their
previous serialized semantics and plan identity domain; v10 has its own plan identity domain.

The [authored consumer](../data/owned/poe2/3887ae68/modifier-transform-inputs/README.md)
adds one factor stat and two programs to the existing catalyst path. Both owners remain
Partial. The original ring's pending modifier membership/order cannot establish an empty
sequence; actual source producers and final effective-value formatting remain unconverted.

## Remaining effective-value and encoding stages

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
