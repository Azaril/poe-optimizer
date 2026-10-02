# Binding socketed augments to owned builds

## Status and purpose

The native augment reconstruction component prepares source text from an explicit finite catalog, policy and request. It preserves ordered contributing socket occurrences and the source grouping behavior, including its unusual decimal tokenization. It does **not** establish that caller-supplied item IDs, slot IDs, categories, activation or scaling facts belong to an owned build. Its `PreparedAugmentReport` is a preparation report, not evaluation authority. The CLI remains an explicit preview of that preparation step.

This document defines the next connection to real build imports. The intended runtime has owned item, socket, modifier and rule data; PoB header parsing and source-selector names remain in Import. The evaluator must never read PoB text, execute a source parser, or accept a preview report as proof that an effect applies.

## Current evidence

The 2026-10-01 checkpoint is `runs/owned-empty-character-runes-01/package`, input
`7ef336de52a8ee2d4ed976db212676d91aada0e79f0be95834a3fd9fa03d51c8`.
Its five saved selections remain Pending with 116 / 117 / 109 / 122 / 20 issues
and 0/5 complete native evaluations. The explicit empty character-rune controls
resolved there are separate from the occupied physical item augments below.

| Original | Selected occupied hosts | Nonempty Rune headers |
| --- | ---: | ---: |
| 01 | 6 | 7 |
| 02 | 6 | 7 |
| 03 | 5 | 7 |
| 04 | 6 | 9 |
| 05 | 0 | 0 |

All 23 hosts retain `RuneLifecycle` source gaps and unresolved item-parameter,
modifier-membership and modifier-order inventories. Header transport alone cannot
retire these inventories. Materializing missing augment occurrences may expose
additional obligations before their inputs/effects become complete. Preserve
alternate-weapon occurrences and all 110 requested queries during this work.
The current release has twelve `SocketSlot` definitions for passive-tree
destinations and no item-kind slots; those existing definitions cannot serve as
equipment augment sockets.

The selected comparison domain includes Greater Iron Rune on both weapon and
armour hosts, repeated Iron runes in three body armours and a quarterstaff,
Glacial/Desert variants, caster runes, named runes, Soul Cores and Idols. Two
selected hosts contain names absent from the pinned catalog: Original02 Item 2
(Shrine Sceptre, `Purity of Lightning`) and Original04 Item 16 (Ashen Staff,
`Jiquani's Soul Core of Rippling`, alongside a known named rune). Preserve their
source evidence; a known sibling must not silently authorize reconstruction of
the whole host. Original04 Item 11 combines Fox Idol and Perfect Iron Rune,
providing a separate mixed-family/Bonded control.

The original historical inspection of `runs/owned-actor-baseline-native-02/package`
and its fresh drafts found no `SocketSlot` definitions or registry entries and no
Greater Iron Rune item-template mapping. Both weapon templates had empty **Partial**
socket declarations with `base-sockets-unconverted`; their destination lists also
remained Partial. The identities below belong to that historical endpoint:

| Original | Source item | Owned item local ID | Owned host-use local ID | Known host template |
| --- | --- | --- | --- | --- |
| 02 | XML Item 26, Grand Spear | `025f` | `0329` | `def.0000000000001ed9` |
| 03 | XML Item 17, Sinister Quarterstaff | `00fe` | `011f` | `def.000000000000232d` |

IDs are abbreviated only in this table. Original 02 has lineage `5074dac8b664540c967ed1bceccd1719`; original 03 has lineage `4a8052bc9d6c8941652c5e9d5e11cfed`. Their host uses have known selected-loadout scopes. Original 02 declares one Greater Iron Rune; original 03 declares two. The saved normal lines are 18% and 36% increased Physical Damage, with separate Bonded lines. These strings do not establish activation or effective magnitude.

The prior drafts contained 34 items/61 equipment uses and 17 items/17 equipment uses respectively, with **no** `ItemSocket` destination. The three pending socket destinations in each are source tree sockets, not materialized rune occurrences. Rune headers therefore have no owned child item/use identity yet.

Fresh normalization now retains all enumerated records but makes top-level item/equipment membership Pending when Rune headers or unresolved `RuneLifecycle` evidence can imply missing child records. The two collection issues link back to each contributing source item. XML Item/Slot enumeration alone cannot prove semantic membership closed. Unreviewed `Rune: None` also cannot prove empty membership; source syntax/default handling needs its own proof. Ordinary non-rune inputs and presentation titles remain unaffected. This changes no global evaluation or allocation availability gate.

## Owned representation: open model decision

A broader inventory audit invalidated the earlier assumption that existing records alone
were sufficient. `ItemRecord` describes a rolled item, while `InventoryItem` identifies
physical stock. `EquipmentUse::ItemSocket` connects child and host **uses**; it does not
preserve an unequipped host's socket configuration. The same descriptor may also appear
in alternative setups. Physical exclusivity must not be inferred from a descriptor ID.

The proposed [persistent socket configuration](owned-socket-configurations.md) separates
host rolls, ordered contents, receiving uses and physical supply. It is awaiting the
owner's design decision. Do not implement dependent child materialization against the old
assumption or declare the proposal accepted. A configured-item alternative remains an
explicit option for that decision.

Under the proposal, a complete ordered configuration establishes its authored slots,
including explicit empty slots. There must not be a second independently authoritative
socket count. Schema capacity, slot kinds, allowed destinations and modifiers affecting
usable capacity remain injected definitions/rules. Import needs a reviewed layout mapping;
absence or Partial membership never proves an empty configuration.

The five originals contain 43 host rows with 76 Rune headers: 36 exact `None` and 40 named
entries. Two names are outside the acquired catalog (`Purity of Lightning` and
`Jiquani's Soul Core of Rippling`); another host needs proper magic-base-name resolution.
Hosts are reused across saved alternatives. Source parsing accumulates uppercase `S`
socket markers and records `J` separately; UI editing limits and base metadata must not
be substituted for the actual imported layout. The read-only evidence is in
`runs/owned-augment-binding-01/source-inventory.json` and `source-semantics.json`.

The sequence below is conditional on settling the owned configuration/identity contract.
Its binding API must derive occurrences from that owned structure, never from an Import
sidecar or an invented inventory copy.

The configuration choice was re-presented to the owner on 2026-10-01 and remains
unanswered. The independent complete-source lifecycle witness now passes for the
actual selected hosts. It does not materialize owned children or choose the
configuration model. Execution verifies a critical distinction: a list containing
only known names and `None` permits
replacement of saved rune lines, while any unknown header, even beyond active
socket capacity, prevents that replacement. A missing header list enters a
separate inference path. The witness establishes those contrasts, disabled-line
transfer, multiplicity, Bonded activation and separately rounded effect additions.

### Complete-source evidence

`owned_occupied_item_augments_source.rs` runs both JIT modes with the complete
pinned runtime and authenticates the original Item, parser, modifier-store and
calculation methods. Each mode covers fourteen complete cases/fifteen loads,
including all five unchanged originals, twenty-four isolated Original03 rows
(baseline plus 23 controls) and six Original04 family controls. Actual MAIN/CALCS
delivery, selected sets, item identities, allocated-node scopes and full output
state are preserved. The resulting files are byte-identical; see the
[implementation checkpoint](implementation.md) for the hashes and run artifacts.

The two unknown-name originals require different correspondence. Original02's
sceptre has no saved `{rune}` lines: its ordinary implicit Purity skill grant is
delivered once, not manufactured from its Rune header. Original04's staff retains
the saved Spell+1 and Nova+1 GemProperty lines despite its unknown header. Both
still count occupied positions in the source. Do not discard those behaviors or
guess a replacement item identity; their eventual explicit fallback conversion
is separate from the first known-name reconstruction lane.

Numerical controls prove 18 plus a separately rounded 4 for one Greater Iron
weapon rune with 25% extra effect, versus 36 plus 9 for two runes. Armour Bonded
Life/Mana 40 gains 5 at 13% extra effect, rather than two independently rounded
increments of 2. Nested gem-quality scaling retains the observed fractional
increment 1.25. The pinned extra-effect loop also omits the ordinary disabled-line
check: a disabled normal line can lose its nominal 36 yet retain the extra 4.
These source quirks must remain explicit in conversion; a more intuitive formula
would not satisfy pinned-reference parity.

Source anchors: `Item.lua` 803–815 (headers), 1363–1402 (all-name guard and disabled
transfer), 1543–1604 (inference), 2106–2178 (grouping), 2198–2219 (Bonded),
2338–2352 (categories), 2723–2798 (effect lists); `ModStore.lua` 82–117 (scaling);
`CalcSetup.lua` 1001, 1321 and 1356 (unlock, receiving slot and occupied counts).
These are reference/import details, not evaluator inputs. The witness grants no
prepared binding authority, contributor closure or whole-build native parity.

## Import and reconciliation sequence

1. **Declare finite data.** Add reviewed augment templates, host socket declarations, allowed child destinations, the accepted persistent configuration declarations, and the Import layout policy through the existing registry/schema extension path. Retain all Partial gaps that are not proved. Do not add synthetic slot IDs only in a CLI request.
2. **Interpret source evidence.** Decode the selected item's `Sockets` and ordered `Rune` headers with a bounded, reviewed source policy. Preserve explicit empty markers, duplicates, surplus entries, unsupported header shapes and incomplete source lifecycles. Unknown data must remain unresolved. A decoded header is source evidence until mapped and installed into owned records.
3. **Allocate owned children.** After the model decision, use the owned allocator to create configuration selections and checked per-use projections; retain source correspondence in the origin sidecar. Do this before closing semantic collections. Emit no child whose template or socket declaration is missing. Re-import must be deterministic for identical inputs and dependency identities; edits must use the existing identity-preserving authoring boundary.
4. **Reconstruct and reconcile.** Prepare catalog-derived lines for the bound ordered socket occurrences. Saved `{rune}` lines are evidence to reconcile with reconstructed lines, not a second source of additive modifiers. Preserve the normal/Bonded lane, contributing occurrences, numeric grouping key/order and original line provenance. Transfer source disabled/display state only through a reviewed correspondence rule; ambiguity, stale saved lines or unknown names stays explicit.
5. **Convert effects.** Reviewed owned line recipes produce modifier occurrences and nominal rolls. Bind activation and ordered magnitude transformations separately, including global Bonded versus Idol-only unlock behavior. Extra augment effect is a separately rounded contribution in the source; it is not automatically multiplication by a single factor. Positive nominal text alone establishes no effective weapon or character stat.

Repeated rune contributions are grouped before the extra-effect calculation.
The future owned consumer must preserve that semantic aggregation boundary;
applying rounding independently to each child can change the result. Source
stat-order numbers and display strings remain import correspondence, not native
grouping authority. Declare the shared semantic contribution and its magnitude
stage through owned definitions/programs, and prove the mapping against source
controls before admitting it. Do not attach both saved host lines and rebuilt
child contributions as independent additive effects.

Inference needs a separate identity policy. Removing the original body armour's
headers makes the pinned source infer Perfect Iron Rune plus Iron Rune, although
its saved configuration contained two Greater Iron runes. The witness checks the
exact inferred names in isolated controls and the complete loaded build, and
proves equal displayed effects, actual delivery and full outputs. Equal totals
therefore cannot establish which physical items were present. Do not infer stock
availability or claim preserved socket identities from an effect match.

A source-only acquisition or reconciliation receipt does not close rule coverage. Removing obsolete source runtime consumers follows a real owned consumer and numerical parity tests, not the existence of another import artifact.

## Binding API contract

Use a private-constructor binding result, conceptually `BoundAugmentPreparation`. Construction accepts the validated owned draft/build, validated schema package, exact equipment-preset/host-use selection, finite catalog and validated layout/conversion policy. The caller chooses a host-use identity; it does not supply authoritative `AugmentHost` or `SocketedAugmentOccurrence` DTOs.

The constructor derives and verifies:

- Exact draft/build revision and digest, schema/data identities, catalog digest and policy identity.
- Selected preset membership, host item/use correspondence, actual template and loadout scope.
- Complete ordered configuration slots, explicit empty contents and their exact schema owner/kind.
- Every occupied child item's actual template, destination, container ancestry, scope and allowed destination membership.
- Completeness of relevant collections, missing/duplicate socket uses, surplus headers, unresolved child references and pending schema membership.

Known local facts may be reported alongside explicit pending dependencies. They do not make the binding globally complete. A partially known collection never yields an invented empty socket or an implicit zero. The binding can expose preparation data after checking it; downstream evaluation consumes compiled owned modifier/rule inputs and continues to enforce its normal completeness and legality gates.

The native evaluator does not take saved display text, source rune names, UI indices, PoB flags or caller assertions of activation/scaling as authority. The Import boundary may retain all of them as provenance for reconciliation and parity diagnostics.

## Required validation

Keep validation in Rust. Use the supplied original 02/03 weapons and their armour rune combinations, plus independent synthetic contrasts covering:

- Same augment repeated, host reused across alternatives, distinct containers, explicit empties, missing/surplus socket headers and unknown augment names.
- Partial declarations and item/preset membership, absent/Partial configuration, authored-layout/capacity mismatch, undeclared slot, wrong owner, incompatible child destination, stale package/draft identities and invalid ancestry.
- Saved/reconstructed line agreement and disagreement; disabled lines; normal/Bonded separation; first-line meaning; exact multi-digit decimal grouping and repeated formatting.
- Socket-configuration/quality/magnitude edits, augment removal/replacement, changes in host template, exact modifier ordering and effect activation.
- No duplicate effects from saved lines plus reconstructed lines, no automatic closure from a source acquisition receipt, and unchanged whole-plan/allocation gates.
- Bounded input/output/work, deterministic preparation, fresh-versus-reused execution and eventual parallel native numerical parity against the optional authenticated PoB oracle.

The current collection-closure regression tests preserve the existing imported Item/Slot records and origin accounting, compare real originals against otherwise equivalent sources without rune evidence, and prove that the missing rune identities are never fabricated.
