# Proven source category as an owned modifier input

This narrow successor adds an ordinary required Option roll to modifier 30ca.
The injected options are Explicit 30e2, Implicit 30e3 and Enchant 30e4; the declared
ModifierRoll slot is 30e5. Its allowed values are exactly those three options.
The existing 23 roll descriptors and every calculation program remain unchanged.

The new item-line V7 `ContextOption` uses the authored input key
`modifier-source-category`. The source V8 finite mapping supplies that value only
for an exact member of a Proven whole-item source layout. It cannot substitute
the preliminary category attached to a member in a Pending layout. Standalone
item-line conversion without the context therefore remains Pending, with no
default category or partial modifier.

This restriction corrects a real ambiguity in original 01: the Lapis Amulet's
incomplete layout preliminarily labels its minion-level line Implicit, while
the complete authenticated source identifies it as Explicit. Its canonical
minion-level modifier must be withdrawn until the full scope is proven. The
original text, candidate rule and diagnostic evidence remain available.
Original 05's Iron Crown has a Proven complete layout and supplies Explicit.

`extension.json` uses the existing checked monotonic extension mechanism to
append four IDs and add one parameter member while retaining Partial inventory.
`roll.json` and `source-binding.json` supply the finite conversion data. A
separate checked release revision then changes only this modifier's input
inventory to Complete: all 24 required canonical fields are represented. Every
rule-owner, item-template, routing, contributor and source-encoding gap remains
unchanged. Category-sensitive magnitude rules still need conversion before
native numerical coverage can be complete.

The test helper assembles a full endpoint from the checked transition and
preserves all 11 predecessor provenance records. It adds explicit append and
inventory-revision records. The final real test publishes via
`assemble-owned-release --revision`, rebuilds the package, normalizes all five
originals and preserves all 110 query rows byte-for-byte. It compares retained
occurrences and references using an injective source-correspondence mapping;
it does not strip or assume stable allocated IDs.

```powershell
$env:POE_OPTIMIZER_TEST_CATEGORY_PRIOR = 'runs/owned-global-minion-level-release-03/package'
$env:POE_OPTIMIZER_TEST_CATEGORY_OUTPUT = 'runs/owned-modifier-category-inputs-01'
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_modifier_category_cli real_category_successor_preserves_originals_and_never_promotes_pending_category -- --ignored --exact
```

The output directory must not already exist. The real test is explicitly ignored
unless its checked predecessor is supplied. This is an input-correctness
publication, not a complete-build numerical parity claim.

The checked run published `runs/owned-modifier-category-inputs-01/package`:

- Input: `6640af4390c83fccc4d99bf3dc42030b468ac23343b761ef5b376f5dea856a0c`.
- Definitions: `de7ff449b2aecf225a1730a14a6901632daedd721372ef865d8ae17fd66da5ca`.
- Registry: `13a3d4a7c3c52f887530de2112717a44cf33160a2517fe42ef34778200e223c8`, ending at `30e5`.
- Release: `pob-3887ae68-modifier-category-inputs-v1`; schema 4 and rule operations 13.
- 18 published files, 58,432,980 bytes and 13 provenance records; rebuilding
  produces identical bytes and the predecessor remains unchanged.

`authoring.json` records the exact predecessor commitment and authenticated
pinned-source module hashes. The complete source witness uses original item
parsing and modifier construction, including category-specific magnitude
controls; these observations justify the finite category vocabulary rather
than a guess based on text position. The new canonical roll is injected data,
not a source-parser enum exposed to the evaluation core.

The two focused default tests passed, the explicit real publication passed,
and the prior global-minion publication regression passed after extracting its
unchanged identity-correspondence helper. Seven positive and negative probes
verify the three category options and refusal of tagged, desecrated, fractional
and unknown-prefix cases. The execution receipt records commands, binary hash
and report paths. `validation.json` records exact occurrence changes, issue
identities and allocator differences. The original Lapis occurrence is source
ordinal 305, line 13; the Crown occurrence is ordinal 576, line 20. These are
test evidence joins, never runtime dispatch keys.

All five original selections remain Pending and their calculations were not
run: numerical coverage is still 0/5. The selected diagnostic counts are
316/322/313/379/151; the two retired roll-inventory issues reflect different
changes (withdrawal of an unproven occurrence versus completion of a proven
input), not numerical evaluation progress. The next selected original 05 blockers are
the six missing item-level inputs. Tattered Robe and Rope Cuffs already have
Proven layouts; the other four require their actual unresolved modifier lines
to be handled before the existing absence-default gate can apply. See the
checkpoint's next-blocker artifact for the exact saved-selection census.
