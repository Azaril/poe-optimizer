# Selected Player off-hand facts

Original-call source authentication, all-five publication and native tests pass.

This packet adds two injected EquipmentUse capabilities to every one of the
1,756 authenticated item-base templates: selected item is a Shield or a Focus.
It appends one program to existing Player Actor `332a`. That program reads the
authored active-loadout occupant of off-hand slot `0065`, derives selected empty
off-hand, and lazily reads each classification only when the slot is occupied.
Missing classification remains unresolved; it is never an inferred false value.

These are **selected equipment structural facts**. They do not implement full
`GetCondition` semantics, effective hand profiles, disabled-item filtering,
substitutions, blocked-hand legality, Unarmed, Unencumbered or Hollow Palm.
PoB `CalcSetup` may filter or replace saved items before `CalcPerform`; numerical
source comparison therefore requires the actual selected/prepared item join.
Source absence/nil versus false remains diagnostic. Modifier condition flags and
inherited conditions retain their own incomplete producer inventories.

The complete finite catalogue contains 193 Shield and 51 Focus bases. Source
names and type strings are consumed by offline authoring and are absent from
the native rule bodies, which contain only typed IDs, Boolean literals and
existing `PlayerEquipmentSlot`, `Not` and lazy `Select` operations. The catalogue
is the existing authenticated export, including original construction order
and duplicate-base overwrite behavior; it is not a filename guess or a list of
the five example builds' items.

All 1,756 template owners, the shared Player Actor and all Classes retain their
prior closures. Five allocated definitions (`332f` through `3333`) and 1,757 appended
programs are the only gameplay-data changes. No action is required to discover
the Player rule. No Class, full condition, item applicability or build coverage
is completed. Publication must authenticate the source witness, exact catalogue
mapping, full recipe inverse, unchanged five imported originals and 110 queries.

Source02 passes all ten cases with fresh repeats and unhooked comparisons in
both LuaJIT modes (60 complete loads, 159.39s). All five unchanged originals,
empty/Shield/loadout controls and a parsed hand-slot disabling modifier are
retained. The disabling control has a selected Focus and an empty prepared
off-hand: this intentional contrast prevents structural facts from being
misrepresented as the effective source conditions. Source01 failed before any
loads on a test-only XML text framing assumption and is retained diagnostically.

The two source reports are byte-identical, each 10,503,843 bytes, SHA-256
`d4bbaebc058287d511cc9bdd1497feb8a9f8ba7eea8d65c40d26a01f54bf27b7`.
The compact projection preserves exact item identity, original call provenance,
raw condition presence and relevant condition ancestry; the full reports retain
scalar outputs and their fresh/unhooked equality. The publication gate verifies
exact report-to-projection and native-case equality, not merely report hashes.

Publication/inverse checks pass in 27.24s at `runs/owned-player-offhand-01`, with
all five originals and 110 queries preserved and a byte-identical rebuild.
Five native component tests pass in 3.65s, including the source projections,
opposite-hand/exact-use and loadout controls, missing/ambiguous input refusal,
actual Partial coverage, query independence and A/B/A plus Rayon replay.
The finite fixture explicitly excludes unrelated mechanics. No complete build
or production owner is closed by these results.
