# Owned attribute resolution

Status: class-base conversion, integer quantization, intrinsic attack baselines and shared
action-source selection are implemented components. Complete native evaluation remains
0/5 originals. The [domain architecture](domain-architecture.md)
and [implementation log](implementation.md) control integration and validation claims.

## Inputs and ownership

Strength, Dexterity and Intelligence use injected integer contribution channels. Class
bases are class-owned contributions; allocated nodes, items and other effects retain their
own provider identities. The eight supported source classes share the same native rule
path. Their names, base values and stat bindings are data, never a Rust class switch.

The offline class converter reads caller-supplied source JSON whose normalized byte hash
matches the existing mapping pin. Reviewed field membership and source-field to owned-stat
bindings are supplied policy. It emits ordinary literal Add effects using existing IDs.
Source text and paths do not enter native evaluation. Numerical Class game-rule membership
remains partial: class-specific unarmed weapon defaults are outside the tree class record.
Their owned intrinsic baseline is a separate component; selection, activation and remaining
class-mechanic obligations must close before the Class rule gap can close.

Occupied hands and the selected numeric attack source are separate facts. In the current
original corpus, equipped caster weapons in originals01/04/05 have no source attack-weapon
data; PoB supplies class unarmed defaults and sets its Unarmed condition. Originals02/03
use real attack-weapon data. Hollow Palm/Facebreaker hand-emptiness checks are separate.
Represent the equipment relation, fallback source and resulting conditions explicitly;
neither fabricate an empty slot nor use a source sentinel/UI object as the native model.

Class declaration closure is separate from rule coverage. A reviewed class has no extra
input ports here, while its level range, ascendancy membership and implicit passive roots
stay exact. The version-2 declaration refinement names only Class or PassiveNode owners
and binds both schema endpoints. It changes only the seven port-list closure markers,
with all members and existing slots preserved. Version-1 passive manifests remain readable
with unchanged wire bytes and digest semantics.

## Intrinsic attack baseline

The offline converter injects intrinsic actor attack facts: rate
1.65/s, critical chance 5%, physical minimum 2, and class-dependent maximum 5/6/8.
Maximum is 5 for Witch, Ranger, Sorceress, Huntress and Monk; 6 for Mercenary and Druid;
8 for Warrior. These reviewed source values belong in configuration data. Source class 0
is outside the current eight-class namespace. Numeric profile identity is separate from
inventory equipment and from an action's selected source.

The version-2 action-routing seam has an explicit selected attack source: active equipped
weapon, intrinsic actor baseline or skill-specific source. Concrete equipment eligibility
and action policies still need coverage for the original builds. Preserve unknown versus absent
states, loadout scope, compatibility and activation. The existing PlayerEquipment route
cannot infer this from a missing stat or equipment match. Hollow Palm off-hand creation,
disabled weapons and skill-specific replacement are separate policy cases. Directly
selected Punch can use the existing direct skill path; automatic PoB main-skill UI fallback
is not a class grant and does not justify extra class input ports.

Class game-rule coverage can close after all intrinsic facts are represented and shared
selection/activation obligations have explicit owners in action/equipment policy. Unsupported
activation must remain visible in those owners. Evidence above is a source audit; independent
numerical and changed-loadout comparisons remain required.

## Shared action-source selection

An action chooses its numeric attack source once, then all related channels use that
choice. The routing package owns named source selectors per declared action output and
exact part/mode/stat-set selection. A source origin is the selected actor, the current
action (for a skill-specific replacement), or an exact player equipment slot. Each route
provides an exhaustive source-name-to-stat mapping; actor and equipment statistics retain
their distinct target types and exact units. This does not add an item-shaped intrinsic
profile or read PoB source keys at runtime.

Equipment selection proves active-loadout slot occupancy before inspecting a separately
injected EquipmentUse capability. Known true selects the equipped source. Known false
uses the explicitly declared ineligible outcome. A proven empty slot uses its separately
declared empty outcome. A present unresolved item, ambiguous occupancy, unknown capability,
or missing selected stat remains unresolved. Failure to obtain a stat never retries another
source. A declared Unavailable outcome means known absence (inactive), not numeric zero.

A shared selector node precedes the chosen stat reads in the native dependency plan.
Every selected route uses the same decision; only that branch's numerical read is demanded.
Conservative dependency and cycle validation still sees both branches. Capability providers,
source activation and complete contributor membership obey the existing global gates.
Selectors cannot make an otherwise partial original build complete. Fixed action-local
sources cover replacements without pretending they occupy a hand; secondary equipment
fallbacks require their own occupancy proof and are outside the first selector policy.

Routing version 2 makes selector membership explicit. Earlier version-1 packages retain
exact wire and identity semantics and cannot use the new fields/variants. The intrinsic
baseline conversion itself does not claim the equipment eligibility or action-policy
coverage needed to wire all original skills through this seam.

The optional Rust/`mlua` exporter authenticates reviewed source bytes and evaluates only
finite data in an isolated, bounded environment. It emits all source rows, including class
0; the injected conversion policy explicitly excludes that class from the eight-class
namespace. The default/native command reads the catalog without Lua. It checks a reviewed
catalog digest independently from carried source provenance, accounts for every source
field, and preserves partial Class rule membership. Existing Python tooling/tests remain
unchanged; both this converter and its new tests are Rust.

## Numerical step

An attribute step first sums eligible flat contributions. An exact zero base skips modifier
reads. Otherwise it multiplies the base by the combined increased/more factor, rounds and
clamps the resulting integer at zero. Attribute calculation does not read override modifiers;
resistance override rules must not be reused as universal scalar semantics.

The owned arithmetic interface keeps integer counts and quantities distinct. Existing
`ScaleInteger` converts an integer count to a quantity using an explicitly injected unit
quantum. `QuantizeInteger` converts a quantity back to an integer count by dividing by a
positive quantum of the exact same unit and applying an explicit rounding mode. The count
must fit `BoundedInteger`; a float cast must never saturate or silently lose that contract.
These are general mathematical operations, not attribute-specific dispatch.

PoB's `round(x)` computes `floor(x + 0.5)`. Owned `NearestTiesPositive` deliberately has
mathematical tie semantics, including values adjacent to a tie and large integers. A
source-faithful recipe therefore authors the addition of 0.5 and Floor explicitly. Likewise,
decimal rounding must preserve multiply/add/floor/divide order instead of assuming division
by a decimal quantum has identical floating-point behavior. Coefficients stay in data.

PoB rounds each local MORE product to two decimal places before multiplying any separately
rounded parent product. One global unrounded Product reduction does not establish this
parity. Owned effect groups need explicit membership, rounding policy and order; source
store objects and UI caches are not runtime entities. Independent cold/warm reference
comparisons must establish any required grouping policy before totals are advertised.

## Finite staged conditions

The source uses exactly two passes, not convergence to a fixed point. Within each pass the
order is Strength, Dexterity, Intelligence. Attribute-comparison conditions update only
after those three values. Per-stat scaling can read live outputs, so an earlier attribute
may be from the current pass while a later attribute still belongs to the previous state.
A simultaneous vector update is not equivalent.

```mermaid
flowchart LR
  I[Initial output and condition snapshot] --> S1[Strength 1]
  S1 --> D1[Dexterity 1]
  D1 --> N1[Intelligence 1]
  N1 --> C1[Comparison conditions 1]
  C1 --> S2[Strength 2]
  S2 --> D2[Dexterity 2]
  D2 --> N2[Intelligence 2]
  N2 --> C2[Final attributes and conditions]
  C2 --> B[Inherent bonuses and dependent calculations]
```

Represent these as finite owned stages with explicit input/output snapshots and contribution
membership. Lower their dependencies into the existing acyclic plan; do not permit final
Stat feedback, generic Lua state mutation or an unbounded iterative solver. Missing initial
conditions/outputs require reviewed seed semantics. Missing evidence is not a false flag.
Final comparison flags and inherent bonuses consume the second pass, not the first.

The staged contribution binding is not implemented by class literals or quantization.
Until it exists, ordinary scalar component tests cannot certify complete attributes. Per-stat
flooring, limits, disabled/doubled/halved inherent bonuses and actor-specific receivers also
need explicit recipes and coverage. Candidate evaluations can run in parallel even though
this short dependency chain within one candidate remains ordered.

## Breadth and integration gates

All five originals remain fixed inputs with 110 query rows. Their class bases are shared
coverage, but selected choice passives alone are insufficient: ordinary passive and item
flat attributes, all/paired-attribute modifiers, increased attributes and other contributors
must join the same stage model. Examples include 6% increased Intelligence on original01
and 8% increased Dexterity on originals02/03. Absence of a conditional phrase in a text
census does not prove complete effect coverage.

Next integration must establish complete selected contributor membership, staged conditions,
concrete attack-source eligibility/activation and inherent-stat receivers. Test ties, signed values,
zero-base lazy reads, noncommuting rounding order, per-stat cross dependencies, inactive
provider paths, partial contributors and actual changed builds. Compare original and held-out
builds independently against the optional PoB backend. Preserve global closure gates and
retire replaced legacy preparation only after its real numerical invariants move here.

Source audit references in the pinned optional checkout: `CalcSetup.lua` 827-832 (class bases),
1853/1861 (unarmed defaults), `CalcPerform.lua` 233-260 (two passes), 496-520 (inherent bonuses),
`CalcTools.lua` 16-57 (scalar arithmetic), `Common.lua` 722-728 (rounding), `ModDB.lua` 214-252
(MORE grouping) and `ModStore.lua` 469/605-645 (live per-stat reads). These explain conversion
and parity obligations; the native evaluator does not load those files.
