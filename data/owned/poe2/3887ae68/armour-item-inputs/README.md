# Crown and Leggings physical input profile

This checked authoring delta extends existing finite item-input and singleton
member contracts to Iron Crown and Cryptic Leggings. The exact predecessor is
the scoped-allocation-access release named in `authoring.json`. Runtime rules
contain owned identities and typed values, without source item IDs or build names.

The complete unchanged source witness observes one explicit member per base and
no buff, implicit, enchant, rune or class member. The two original items carry
RARE, explicit raw LevelReq 5/80, ordinary quality 20, and three sockets with three
empty rune entries. Fresh raw corruption, catalyst kind, catalyst amount and item
level are absent. Source construction flags and saved affix/display records stay
accounted by the existing finite source grammar; they are not inferred modifier
tags or additional canonical modifiers. Active base armour, energy shield,
quality and movement records remain separate template calculation semantics.

The extension appends twelve template-owned item parameter slots, six per base:
catalyst kind and amount, rarity, fresh corruption, optional raw level override,
and physical socket capacity. It reuses existing rarity/catalyst Options and
percentage units. The admitted profile still requires an explicit LevelReq
header, even though its schema slot is OptionalOnce. Every static template
declaration and owner closure remains Partial, with all old members and gaps.
There is no new Stat or numerical final-output channel for the four raw fields.

Each template gains the existing `catalyst-inputs` program, with only its exact
template and parameter slot identities changed. It transports declared values
to the same EquipmentUse catalyst channels. The injected source defaults supply
owned None and helper-effective amount 20 for absent catalyst headers; this is not
a claim that the raw source amount is stored as 20. Explicit zero is preserved by
the shared header codec. Missing/invalid native parameters receive no runtime
fallback. Existing absent item-level and Pending missing-quality defaults are
unchanged; the actual quality 20 comes from each saved header.

`bindings.json` supplies the four raw projections, `catalyst-bindings.json` the
two scalar inputs, and `source-defaults.json` the exact defaults. `membership.json`
adds the two reviewed bases and their existing modifier rules to the singleton
profile. Explicit catalyst headers, additional members, occupied or uncertain
augments, unexpected source fields and other unreviewed constructions retain the
existing conservative proof exclusions.

The helper uses checked schema-membership transition before authoring the new
item/default/policy rows. Both physical and singleton policies bind the final
schema, item-line policy and source policy. Existing Gem inventory commitments
are rebound by the checked transition, and the V2 tree content is preserved.
Removing exactly the twelve slots, two programs and added policy/default/header
rows, then restoring only enumerated dependency commitments, must recover the
entire predecessor. All old programs, source pins, registry history and 110 query
rows remain intact.

The complete source witness runs three full loads and 35 fresh controls in each
JIT mode, with two reused-object contrasts. Its additional flask-base pin belongs
to a negative generated-buff control and does not expand the admitted base set.
The real CLI regression is the authority for the expected six selected issue
retirements. Source component evidence and native physical-input admission do not
establish final resource/defence results, complete owner coverage or whole-build
numerical parity.
