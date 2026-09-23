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

## Canonical numeric components

`compile-owned-modifier-values` lowers a schema-bound policy into ordinary owned rule
programs. The evaluator continues to execute one typed rule graph. The policy selects
explicit component slots, units, precision, scalar channels and semantic sign; game
values and bindings remain data. The compiler performs no source parsing or schema
allocation and preserves the operation contract and every unresolved owner gap.

Inputs are canonical **unrounded** quantities. Multiply by the declared internal precision,
apply literal signed half-offset rounding, then (when component-scalable) apply corrupted
base rounding followed by ordered magnitude truncation. Divide by the internal precision,
round to the declared display precision, and finally apply an optional qualifier sign.
An explicit unit factor bypasses its stage; an absent factor is unresolved. A non-scalable
component bypasses both scalar stages. A line-level unscalable flag has a different role
in scalar eligibility and must not silently bypass corruption.

Import's [v4 numeric projection](owned-normalization.md#injected-item-line-conversion)
normalizes admitted text into these quantities. A recipe explicitly selects capture or
unrounded offset interpolation, negation, decimal transport and signed/magnitude/direction
projection. The reviewed range recipes use fourteen significant digits before reparse;
fixed captures retain exact decoded values. Qualifiers use a nonnegative magnitude plus a
required Boolean sign, including the temporary sign of negative zero. Source spellings and
that temporary state stay in Import. V2/v3 policies retain their existing wire, identity and
arithmetic semantics; literal v3 interpolation remains available without decimal transport.

This distinction matters at real floating-point boundaries: the interpolated value
`2 + 0.5 * (2.01 - 2)` must be multiplied by 100 before half-offset rounding. Dividing by
0.01 instead can select a different integer. Multiplication and division are separate
operations; reciprocal substitution is not part of the numeric contract.

Revision 2 of the [canonical input package](../data/owned/poe2/3887ae68/modifier-value-inputs/README.md)
publishes eleven canonical owners, explicit corrupted-base inputs and sixteen numeric
programs. It branches from the 9533-entry ordered-transform ancestor and ends at 9854;
the superseded, unconsumed 9843-entry revision remains immutable. Previous nominal owners
and their Complete parameter declarations retain their meaning. The canonical owners'
known parameter membership is Partial, leaving eligibility inputs explicitly unresolved.
The 27 reviewed source recipes capture raw values anew rather than copying rounded
predecessor rolls. All known required slots remain mandatory, and admitted rolls preserve
Pending collection closure through normalization.

Eleven factor-read programs derive the distinct corrupted-base channel from an explicit
required roll. The admitted ordinary grammar emits factor 1 only after proving absence
of a per-line corruption control; an item-level `Corrupted` header does not set it.
Unsupported controls remain blocked. The two resistance owners retain their four reviewed
catalyst/ordered-magnitude programs; local magnitude producers remain absent. Neither raw
admission nor these producers establishes complete transform membership, fixed-text cache
authority or a final weapon/character contribution. Missing factors never become unity.

## Assembled elemental and chaos channels

The [elemental weapon package](../data/owned/poe2/3887ae68/elemental-weapon-inputs/README.md)
adds separate assembled minimum/maximum endpoints for Cold, Fire, Lightning and Chaos.
The four canonical flat families contribute effective values to these receivers; raw
profile channels stay baseline producers. Elemental endpoints add flat values, then
apply the sum of independently reduced type-specific and shared local-elemental
percentages, followed by the explicit half-offset floor. Chaos preserves the raw-plus-
flat fractional result. Required contributor membership, source eligibility and action
routing remain independent proof obligations. Explicit component facts do not prove
that real imported occurrences supply every input. An optional Rust reference target executes
the authenticated original local-damage span and compares the eight production receiver
programs across 208 endpoint cases. It observes values before positive-pair suppression
and does not reproduce a PoB UI or input-loading lifecycle.

Before claiming bitwise occurrence-level parity, validate contribution reduction order
against source list order near rounding boundaries. Even small decimal cancellation can
change a later half-offset floor: with raw 0.5, the flat sequence [0.6, 0.2, -0.8]
sums to zero while [-0.8, 0.2, 0.6] can leave a tiny negative residual. Current component
tests use already reduced facts. A deterministic native order is not automatically the
source order, and no tolerance or completeness claim may hide this unresolved contract.

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
explicit zero, fractional arithmetic and actual imported ring inputs. Optional PoB tests
include the existing 19 ranged/fixed/rune encoding contrasts, 16 numeric-stage cases and
136 public Import-to-native projection cases against authentic pinned formatting. These
cover signed qualifiers, negative zero, range decimal transport, precision and scalar
ordering; they do not establish source admission or final contributions for every build.
Complete native original builds remain 0/5, and all 110 query rows stay in the denominator.

No legacy numerical caller is replaced by this intermediate alone. The
[retirement inventory](legacy-retirement.md) retains its named consumer gates. Final
resistance preparation, receiver integration and all-five parity must migrate useful
numerical tests before their old dependency closure is deleted. No new Spark/Mace route,
source callback, UI lifecycle or parallel default evaluator was added here.
