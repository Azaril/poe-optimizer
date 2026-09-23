# Weapon catalyst inputs

This data-only extension adds two declared parameters and one `catalyst-inputs`
program to each of the 337 already registered weapon templates. It allocates
674 parameter slots (9855 through 10528), preserves each template's Partial
parameter membership, and uses the same typed selection/amount contract as the
existing Sapphire Ring adapter. The native consumer reads exact EquipmentUse
properties; it never dispatches on a base name or imports source objects.

Apply this stage after canonical admission revision 2 (9854 entries), supplying
both replacement policies together:

```powershell
poe-optimizer extend-owned-recipe PRIOR --extension data/owned/poe2/3887ae68/weapon-catalyst-inputs/extension.json --items data/owned/poe2/3887ae68/weapon-catalyst-inputs/items.json --item-source data/owned/poe2/3887ae68/weapon-catalyst-inputs/item-source.json --output NEW
```

Item-line policy v5 shares two header grammars across all 338 reviewed templates.
Their `TemplateParameter` emissions bind only after a unique item template is
known, selecting that template's exact declared slot and value schema. Binding
maps are Import configuration; the owned build and evaluator retain ordinary
`ParameterAssignment` values. The new policy does not duplicate a header grammar
for each base. `bindings.json` inventories every owned address.

The source policy retains its v4 wire model. Only a fresh source item with an
admitted layout can receive the reviewed missing-input defaults: selection None
and amount 20. Selection-only uses20, amount-only remains inert, and explicit0
is preserved. Malformed, unknown or duplicate headers cannot authorize fallback.
This does not model reparsing mutable cached PoB Item state. New weapon defaults
leave item-level and ordinary-quality absence Pending; the existing ring's
independently reviewed absence declarations remain unchanged. Ordinary quality
continues through its own field and later weapon calculations.

The pinned Item parser and catalyst helper accept these headers independently
of base category. This is a computational input contract, not proof that a given
catalyst can legally be applied to a weapon in the game. Legality and acquisition
remain separate checks. Unknown templates do not fall back to the ring adapter.

The optional Rust full-Item oracle contrasts fresh ring and spear instances,
missing/explicit header combinations, ordinary quality and matching/nonmatching
modifier labels. Rust publication tests exercise all declared bindings and the
five real imported originals. Parameter and modifier collections remain Partial;
this extension supplies no rune, range-prefix, source-order or global contribution
completeness proof. Complete native original-build evaluations remain 0/5.
