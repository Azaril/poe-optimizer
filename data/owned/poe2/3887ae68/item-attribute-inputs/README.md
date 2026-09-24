# Ordinary item attribute inputs

Four injected families represent flat Strength, Dexterity, Intelligence and all Attributes.
Their names, fixed grammars, typed identities, properties, numeric precision and membership
are data. Native evaluation gains no game-name switch, PoB source interpreter or new opcode.

## Data contract

The four owners are `295c`, `2974`, `298c` and `29a4` in registry namespace
`poe2/owned-mechanics-v1`. Each declares 23 required raw inputs: one amount, twenty explicit
source properties, ordinary unscalable state and a corrupted-base factor. Partial input
and rule coverage remain. The shared Count unit `295a` and Modifier Quantity output `295b`
keep attribute counts separate from percentages and resource points.

One all-Attributes line emits one owned modifier occurrence with one amount. Source parity
witnesses retain Str, Dex, Int and All records inside that occurrence. All is not a fourth
Actor attribute. Actor contributions are deferred: existing Strength/Dexterity/Intelligence
channels are Integers and require an explicit reviewed conversion and activation policy.
Requirements, requirement reductions and derived Life/Mana/Accuracy are separate mechanics.

The generic modifier-value compiler emits precision-1, display-precision-0 component rules.
The existing catalyst, ordered magnitude and corrupted-base producers retain their exact
units and order. Catalyst quality stays percentage data. Attribute tags are explicit source
facts; English text never invents them. Numeric component results do not certify a complete
item, eligible effect or actor contribution.

Eight fixed signed grammars retain raw decimal components. Only plus-prefixed ASCII integer
amounts 0..1,000,000 with no physical tags and no generated base prefix obtain independent
source-member authority. Negative, decimal, scientific, crafted, rune, range and paired
attribute cases receive no new source proof. Preceding unreviewed lines continue to block
independence. Repeated physical occurrences remain separate.

## Compact authoring and publication

`extension.json` allocates the 98 entries and sixteen programs. `membership-patch.json`
names all 1,756 reviewed base templates and the four modifiers explicitly, adding 7,024
finite structural memberships through the existing checks. It does not infer affix legality.
The extension (178,797 bytes) and patch (199,551 bytes) replace a 9,225,003-byte explicit
extension for this transition; the final schema is identical. Published runtime packages
remain materialized. See [the authoring contract](../../../../../docs/owned-recipe-membership.md).

`items.json` and `item-source.json` bind the exact successor schema and line policy.
`bindings.json` retains authored IDs, source expansion witnesses and catalog provenance.
Publish through the native Rust CLI:

```text
poe-optimizer extend-owned-recipe PRIOR --extension data/owned/poe2/3887ae68/item-attribute-inputs/extension.json --membership-patch data/owned/poe2/3887ae68/item-attribute-inputs/membership-patch.json --items data/owned/poe2/3887ae68/item-attribute-inputs/items.json --item-source data/owned/poe2/3887ae68/item-attribute-inputs/item-source.json --output NEW
```

The authoring request binds exact prior registry/schema/rules/routing and supplied extension
identities. Publication does not overwrite destinations or repair stale caller bindings.
Rust tests cover explicit/compact equivalence, source parsing and numeric formatting,
full native publication, original occurrence/query preservation and incomplete coverage.
Whole-build native parity remains a separate integration gate.