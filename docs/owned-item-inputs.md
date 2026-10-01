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
| Physical rarity and corruption | Typed item facts, distinct from increased rarity of found items and from a modifier's numerical corruption factor. Their later consumers include equipment counters and slot conditions. These inputs still need authored declarations, conversion and programs. |
| Authored requirement override | Explicit optional/presence information and its value, distinct from both item level and the computed equip requirement. Its representation must preserve absence and zero. Conversion and native consumers remain to be delivered. |
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

## Integration order

The immediate dependency is the existing flat-Life modifier's catalyst inputs.
Tattered Robe and Rope Cuffs have proven singleton modifier inventories but no
template producer for the shared catalyst properties. Add exact slots and the
existing native transport program, with source-checked header/default bindings.
Leave their physical parameter inventories and static schemas Partial.

Next account for rarity, corrupted state and authored requirement overrides under
this same contract. Establish their raw input roles and the finite inventory;
do not add derived counter channels merely because the reference initializes
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
