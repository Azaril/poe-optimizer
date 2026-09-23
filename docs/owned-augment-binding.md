# Binding socketed augments to owned builds

## Status and purpose

The native augment reconstruction component prepares source text from an explicit finite catalog, policy and request. It preserves ordered contributing socket occurrences and the source grouping behavior, including its unusual decimal tokenization. It does **not** establish that caller-supplied item IDs, slot IDs, categories, activation or scaling facts belong to an owned build. Its `PreparedAugmentReport` is a preparation report, not evaluation authority. The CLI remains an explicit preview of that preparation step.

This document defines the next connection to real build imports. The intended runtime has owned item, socket, modifier and rule data; PoB header parsing and source-selector names remain in Import. The evaluator must never read PoB text, execute a source parser, or accept a preview report as proof that an effect applies.

## Current evidence

Inspection of `runs/owned-actor-baseline-native-02/package` and its fresh drafts found no `SocketSlot` definitions or registry entries and no Greater Iron Rune item-template mapping. Both weapon templates have empty **Partial** socket declarations with `base-sockets-unconverted`; their destination lists also remain Partial.

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

## Import and reconciliation sequence

1. **Declare finite data.** Add reviewed augment templates, host socket declarations, allowed child destinations, the accepted persistent configuration declarations, and the Import layout policy through the existing registry/schema extension path. Retain all Partial gaps that are not proved. Do not add synthetic slot IDs only in a CLI request.
2. **Interpret source evidence.** Decode the selected item's `Sockets` and ordered `Rune` headers with a bounded, reviewed source policy. Preserve explicit empty markers, duplicates, surplus entries, unsupported header shapes and incomplete source lifecycles. Unknown data must remain unresolved. A decoded header is source evidence until mapped and installed into owned records.
3. **Allocate owned children.** After the model decision, use the owned allocator to create configuration selections and checked per-use projections; retain source correspondence in the origin sidecar. Do this before closing semantic collections. Emit no child whose template or socket declaration is missing. Re-import must be deterministic for identical inputs and dependency identities; edits must use the existing identity-preserving authoring boundary.
4. **Reconstruct and reconcile.** Prepare catalog-derived lines for the bound ordered socket occurrences. Saved `{rune}` lines are evidence to reconcile with reconstructed lines, not a second source of additive modifiers. Preserve the normal/Bonded lane, contributing occurrences, numeric grouping key/order and original line provenance. Transfer source disabled/display state only through a reviewed correspondence rule; ambiguity, stale saved lines or unknown names stays explicit.
5. **Convert effects.** Reviewed owned line recipes produce modifier occurrences and nominal rolls. Bind activation and ordered magnitude transformations separately, including global Bonded versus Idol-only unlock behavior. Extra augment effect is a separately rounded contribution in the source; it is not automatically multiplication by a single factor. Positive nominal text alone establishes no effective weapon or character stat.

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
