# Local weapon input recipes

These are directly authored owned rule expressions, consumed by the native
`extend-owned-recipe` command. Neither the command nor native evaluation reads
PoB source, runs Lua or selects a formula by a weapon/skill name. The formulas
were reviewed against pinned `Classes/Item.lua` local assembly and `Common.lua`
rounding. This is intermediate numerical coverage, not complete item evaluation.

Apply to the 9254-entry raw-profile successor described in
[weapon profiles](../weapon-profiles/README.md):

```powershell
cargo run --bin poe-optimizer -- extend-owned-recipe runs/owned-item-numeric-01/package --extension data/owned/poe2/3887ae68/local-weapon-inputs/extension.json --output runs/owned-local-weapon-01/package
```

The extension allocates eight statistics (9255–9262), adds standard-quality
membership to the 337 explicit weapon templates, and installs template-owned
quality adapters plus four shared equipment receivers. `bindings.json` documents
these injected addresses; it is not an implicit runtime lookup table.
All previous schema gaps and program closures remain unchanged. New known members
of Partial schema sets use explicit endpoint-bound membership refinement v3.
Applying the extension again is an identity-preserving no-op for constituent data.
Previously published program/receiver/table values cannot be overwritten.

The four receivers consume exact EquipmentUse baselines and explicit contribution
sets. Their outputs are pre-override attack rate, pre-override critical chance,
and rounded physical minimum/maximum before the positive-pair emission check.
They account for alternate-quality speed/critical increments and suppression of
ordinary physical quality. Decimal rounding retains the source floating operation
order: scale, add one half, floor, divide. Physical scaling retains its two
successive multiplications. The test case at 1.005 distinguishes this sequence
from an assumed decimal-rounding idealization.

Quality requires an explicitly selected standard quality (definition 6), including
an explicit amount of zero. Missing or another quality kind does not become an
invented zero. Source import has not yet admitted the original Quality headers.
Complete-empty contribution sets have their declared additive zero identity;
incomplete contributor membership stays unresolved. Source-facing modifier rules
must still distinguish local effects from global effects such as elemental damage
with attacks. These new statistics are not yet routed as final action inputs.

Remaining local assembly includes elemental/chaos channels, range, reload,
positive-pair field emission, ordered WeaponData overrides, crafted-quality state,
per-hand conditional modifiers and DPS channels with correct units. Original rune
headers require finite injected rune definitions and socket/category selection,
ordinary/Bonded reconstruction and reconciliation; saved rune modifier lines alone
are not authoritative. Unknown variants or mismatches remain Pending. The opt-in
source flag dialect preserves fractured/desecrated provenance but does not solve
runes or close modifier order/membership.

Validation covers publication through the production CLI, owned data changes,
independent arithmetic expectations, occurrence binding in engine tests, and all
five original imports retaining all 110 query rows and Pending allocation/item
coverage. Complete native original builds remain **0/5**. No Python utility or test
is introduced or converted.
