# Fixed increased Fire Damage

This candidate family addresses Original04's actual Ruby jewel (physical Item1,
source171, line8), `14% increased Fire Damage`, equipped at passive node46882.
Its predecessor is the published configuration-rating endpoint
`4e81b3ba1994c7a2052cb336a9efeba0baedeae11eed477b6d9a503390ad86ad`.
The complete-source witness passes 12 fresh build loads and 29 isolated item
controls in each JIT mode. The evidence files are byte-identical, SHA
`2196b7220ebd5b07114596b13ead6cea62866ddfb27a87b5fd3446165d2ff8cd`.
All three native component tests and both publication tests pass. The checked
endpoint is `runs/owned-fire-damage-modifier-04/package`, input
`043b0f0466aa2fa8516e985a5660e2deeb4e061dac9f8999f15624d48e4435d5`.
This family does not establish successful build evaluation.

The parser's `FireDamage INC` modifier has no Spell restriction. This is distinct
from local added Fire Damage and from Spell Damage. Source property flags are
read from actual source tags; the word Fire, a jewel base tag, and an affix name
must not synthesize the catalyst `fire` property. The numeric stage does not yet
route damage to an actor or skill.

The 25 new allocations are modifier `31e4`, raw amount `31e5`, twenty property
flags `31e6` through `31f9`, unscalable `31fa`, corrupted factor `31fb`, and category
`31fc`. Existing category options `30e2`/`30e3`/`30e4`, catalyst inputs, and transform
channels are reused. Raw and effective amounts have percentage-point unit
`0002`; effective stat `253e` is modifier-scoped. The ordinary numeric compiler
uses precision1 and display precision0, with existing corrupted-base, catalyst,
and ordered-magnitude stages. The extension explicitly preserves operationsV14.
Dependencies are copied from the exact predecessor schema.

The initial source admission is a bounded unsigned integer from 0 through
1,000,000, without source scaling tags or generated buff members and with initial
scaling one. Complete-source observations confirm amount, formatting, category,
property and exclusion behavior, including zero as one clean FireDamage INC0
member. Fractional, ranged, signed and transformed encodings
remain outside this admission. The 24-member parameter inventory is Complete
only for this canonical family. Fractured/desecrated properties are retained and
rejected by the existing unconsumed-property guard.

The helper appends membership only to Ruby template `200b`. A scoped
source-default row supplies ordinary quality absence only after the existing
whole-layout proof and header guards succeed. Item-level default remains Pending;
the actual Known55 item level is preserved. No catalyst default or raw physical
parameter slots are added. Ruby modifier inventory, order and physical parameter
fields stay Pending. The quality issue alone retires. Whole-corpus publication
preserves all 110 queries and checks exact old-fact/ID correspondence plus 20 input
probes. Recognition also exposes existing Partial-roll rarity and Lightning
modifiers after Fire lines on two rings, without admitting those templates to
this family. The selected rarity gap offsets the retired quality gap; the
Lightning gap is archived. Selected counts remain 123/124/116/153/20, with 0/5
complete native evaluations. Static rule-owner/contributor closure remains
Partial, with no final DPS or whole-build coverage claim.

The source witness also distinguishes imported and crafted items. PoB only
preprocesses modifier magnitude when `crafted` or `advancedCopy` is present.
The imported Ruby has neither marker and its explicit-magnitude contrast stays
14; the crafted contrast becomes 21. Numerical fixtures may supply a synthetic
eligible transform to validate the published arithmetic, but do not establish
production eligibility. That producer remains a separate prerequisite.
