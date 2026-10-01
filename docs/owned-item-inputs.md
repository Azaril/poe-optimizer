# Physical item inputs and derived equipment facts

This contract applies the [domain architecture](domain-architecture.md) to item
parameters. It uses existing owned records, typed parameter declarations and
EquipmentUse programs. It does not introduce another item model or source-state
interpreter. See the [implementation log](implementation.md) for delivered scope.

An item is a physical inventory occurrence. Equipping it creates an equipment-use
context; the same occurrence can supply more than one context. Its template and
authored parameters belong to the physical occurrence. Effects, slot conditions,
requirements and contributions are resolved for the applicable context. An item
edit changes every affected use without copying mutable facts into its modifiers.

## Where information belongs

| Information | Owned representation and responsibility |
| --- | --- |
| Template identity, item level and ordinary quality | Existing ItemRecord fields. Unknown, explicitly absent and zero remain distinct. Item level is not an equip requirement. |
| Catalyst selection and applicable amount | Exact template-declared Option and Quantity parameters. The template program projects them into shared typed EquipmentUse properties; each modifier uses the properties of its actual supplying use. |
| Physical rarity and corruption | Typed item facts, distinct from increased rarity of found items and from a modifier's numerical corruption factor. Scoped conversion supplies these facts; a later equipment counter or slot condition needs its own declared consumer. |
| Authored requirement override | An optional integer parameter, distinct from both item level and the computed equip requirement. The first scoped source proof requires an explicit `LevelReq` header, including zero. Missing and alternate header forms remain unresolved; native final requirements remain a separate calculation. |
| Socket capacity | An integer physical parameter, distinct from the inventory of installed augments. Empty sockets still contribute capacity. |
| Base defences, inherent penalties and base requirements | Injected template definitions/programs, shared by all instances of that template. Do not copy a reference evaluator's computed values into a build as raw inputs. |
| Final requirements, effective quality and local defences | Native results derived from applicable base facts, physical inputs, augments and modifiers, with their own dependency and contributor coverage. |
| Modifier membership, rolls and semantic order | Existing modifier occurrence records and explicit order. Inherent template effects and quality-derived effects are not extra authored modifier occurrences. |
| Augments, granted effects and variants | Their declared ownership and selection contracts. An unsupported member or lifecycle blocks the relevant proof; an empty list requires evidence. |
| Display values, source affix labels and editor fields | Source sidecar where appropriate. Exclusion from semantic inputs requires evidence for the admitted lifecycle; recognition as a header is insufficient. |

Physical facts and derived properties are not interchangeable. In particular,
source `corrupted = true`, the numerical multiplier applied to a modifier and the
number of corrupted equipped items are three different quantities. A generic
EquipmentUse Stat channel is for a named, typed semantic consumer, not a bag of
source object fields. Parameter reads remain bound to their exact declaring owner.

## Absence and defaults

Only Import resolves source-format defaults. Native evaluation reads explicit
owned inputs and reports missing dependencies; it cannot fill in a missing
catalyst amount, rarity or requirement based on a build or base name.

For the admitted fresh-item catalyst lifecycle, a missing selection normalizes
to the owned None option. A separately absent amount normalizes to the source
helper's applicable amount of 20. This is a computational input, not a claim that
the source stored 20 or that the item can legally receive that catalyst. Explicit
zero stays zero. An amount without a selection is inert. A selected catalyst
scales only matching modifier properties; the word “Life” in a displayed modifier
does not establish a source property tag.

Each absence proof binds the exact template, source layout and conversion policy.
A malformed, unknown or duplicate header cannot authorize fallback. Defaults are
independent per field, so a known amount does not prove a missing selection or
vice versa. Reusing a mutable source Item object can retain previous values;
fresh-import evidence does not cover that history-dependent lifecycle. Preserve
source/default provenance in the sidecar without adding source state to Core.

## Completing an item input inventory

Adding two known parameters does not establish a complete parameter list. To
complete an admitted physical inventory, account for every semantic field in its
source lifecycle, every required declared slot, and any excluded variant, augment
or construction path. Keep unconverted facts visible until their roles are proved.

The gates remain independent:

1. Physical inputs establish the retained parameters, modifier members and order.
2. Definition schemas establish which inputs and choices exist and are required.
3. Native owner programs establish how supplied inputs produce equipment facts.
4. Routing, contributor inventories and aggregate formulas establish the requested
   numerical outputs.

Neither known parameters nor complete modifier membership closes later gates.
A component test may provide explicit finite synthetic coverage to exercise real
programs, but must also test the actual Partial declarations and must not report
whole-build coverage from that fixture.

Shared source recipes need particular care. Changing a common metadata header
into a template parameter with only two bindings can add unresolved inputs to
every other template. Inventory affected templates and source roles before that
change; do not silently discard unbound emissions or treat gameplay metadata as
harmless. Revalidate every original selected request and preserve archived inputs
and queries as well.

The optional `PobFreshOrdinaryInputsV1` normalization policy is an Import-only
adapter bound to the exact definitions, item-line policy and source-layout
policy. Its header captures use existing checked codecs; its outputs are owned
typed parameters. Core and Engine do not receive this source grammar. Omitting
the policy preserves the earlier unresolved inventories and sidecar version12;
using it emits version13 with per-parameter source evidence.

Its first finite domain is a fresh rare item with saved affix metadata, one
converted explicit modifier, reviewed display fields and empty augments. The
proof accounts for all source lines and exactly the known declared parameter
set. Recognizing a line as metadata is not sufficient: source setters, aliases,
duplicates and unsupported construction paths still reject completion, even
if another policy recognizes them as display observations. A `Crafted:` header
is construction evidence; the source enables it by presence, including the text
`false`, so it is not parsed as an ordinary Boolean. Fresh corruption exclusion
does not authorize reparsing a mutable source object.

Successful projection retires only the converter's resolved missing-parameter
diagnostic. It retains static schema gaps and does not establish derived base
requirements, final requirements, defence totals or modifier coverage.

A bounded source domain contains one implicit and one explicit modifier.
It needs a separate versioned Import admission profile: a singleton proof cannot
establish this inventory or its order. Join every proven source member to its
exact converted emission and complete roll set, then carry those identities in
the proof. Materialize canonical order from the source's implicit-before-explicit
construction. A general source-text ordering rule does not follow from this
two-member case. The original singleton profile retains its behavior and bytes.

The corresponding physical-input construction may admit an absent sockets header
only when the exact source-bound augment proof establishes zero capacity. Record
that absence as its own provenance case; do not fabricate a source line. A new
sidecar version identifies this evidence, while existing profile output stays on
its previous version. These source contracts remain inside Import and do not add
source-format fields to Core or Engine.

Quality absence is independent of this membership proof. For a base without
ordinary quality, missing quality can remain absent through fresh construction.
Explicit zero must retain its value: it can cause source behavior that absence
does not. Default admission still requires a proven whole layout and complete
header census. Neither a missing header nor a numerical zero closes the static
quality domain or proves all potential quality consumers accounted for.

## Integration order

Tattered Robe and Rope Cuffs have exact catalyst slots and native template
transport programs, with source-checked header/default bindings. Their finite
physical input policy also accounts for rarity, corruption, explicit requirement
level and socket capacity. Static parameter declarations and owner coverage stay
Partial independently of a complete concrete item parameter list.

The [armour input profile](../data/owned/poe2/3887ae68/armour-item-inputs/README.md)
applies those same contracts to the independently witnessed Iron Crown and
Cryptic Leggings lifecycles. It adds injected base/member facts and twelve
template-owned slots. Their source-derived armour, energy shield and quality
contributions remain separate calculation dependencies; the finite explicit
modifier inventory does not absorb those contributions or close static coverage.
No new Core type or Engine operation is required for this extension.

Do not add derived counter channels merely because the reference initializes
them. Prioritize numerical producers only when an actual requested dependency
needs them. The current original05 selection has no identified rarity/corrupted
item-count or physical requirement consumer. Other originals contain reduced or
ignored attribute requirements, which are relevant to equipment legality rather
than a requested requirement metric. This does not establish that those physical
facts are irrelevant to every build or justify closing their unresolved roles.

When requirement calculations are needed, derive base and final requirements
through injected programs instead of importing final source values. Include
crafted state, saved affix/display fields, augment capacity and variants in the
finite inventory or explicit exclusions before claiming complete inputs. Keep
the existing numerical dependency graph and item-input coverage checks distinct;
do not silently relax either gate to obtain an evaluation.

For the two admitted Life occurrences, catalyst transport fills the last missing
raw scalar producer identified in the current component graph. Remaining
transform membership, owner coverage and contributor coverage still prevent a
whole-result claim. The Player Life contribution also needs a final resource
aggregate. For these same items, existing template programs provide ordinary
quality and raw Armour, Energy Shield and movement-penalty values, but local
defence composition and penalty application still need consumers. These are
concrete dependencies of the requested build outputs and take priority over
unused reference counters after the input boundary is resolved.

At every checkpoint, run finalization for all five unchanged selected requests;
run native evaluation where admitted. Record the first failed boundary and its
next concrete dependency. A lower diagnostic count is not the success criterion:
the target is complete native evaluation with the original query set and checked
parity. Source witnesses and component tests explain individual steps toward it.
