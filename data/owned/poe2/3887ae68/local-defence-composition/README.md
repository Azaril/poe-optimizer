# Local defence composition

Two shared EquipmentUse reducers compose rounded Armour and Energy Shield before per-level additions and property overrides. Registration initially covers exactly Iron Crown and Cryptic Leggings. The existing template owners and incoming contributor inventories remain Partial; the two new reducer owners describe only their finite guarded programs.

Single-stat, paired-defence and all-defence streams retain distinct identities and explicit units. Armour flat additions use single plus raw, then Armour/Evasion, then Armour/Energy Shield. Energy Shield flat additions use single plus raw, then Evasion/Energy Shield, then Armour/Energy Shield; its increases instead visit Armour/Energy Shield before Evasion/Energy Shield. Two successive scaling operations and add-half/floor rounding preserve the reviewed calculation order. Paired flat magnitudes project explicitly into each output's unit.

Standard quality comes from the existing explicit item-quality input `2427`. Unknown or nonzero crafted-quality contributions withhold output pending mechanical quality preparation; no editor state, default quality or UI normalization enters native evaluation. Alternate-quality contributions suppress ordinary defence quality only when positive. Zero and negative values do not suppress it. Contribution identities apply only after complete contributor inventories are established.

The source witness passed with byte-identical fresh JIT-off/on reports in source03. Six cases cover independent replay, warm restoration, explicit qualities zero/twenty/thirty and nonzero local modifier controls. Twelve native case vectors bind the exact selected Item objects, original local group returns, original pre-override assignments and original MAIN/CALCS slot consumers. Old warm objects remain diagnostic evidence. Direct constructors and explicit source quality normalization are recorded separately. Synthetic controls establish numerical behavior, not item obtainability or source admission. Failed source01/02 remain framing and observer-assumption diagnostics.

This packet adds no modifier delivery, final item/Actor defence, mitigation or whole-build coverage. Per-level additions and property overrides are later stages, so these intermediate outputs do not ignore or implement them. The next step is to reconcile the selected item owners and their actual input inventories. None of the 19 existing numeric-only defence modifier families appears on the selected Crown, Leggings or Solar Amulet; their delivery remains a broader backlog. Crown and Leggings each have three explicit empty sockets, which require the accepted socket-configuration model rather than an empty declaration copied from Solar Amulet. Empty selected modifier groups alone do not establish global contributor completeness.

Validation passes: original-function source03 (455.29 seconds), authored checks,
publication02 (26.87 seconds), and seven native component tests (2.15 seconds).
The JIT-off/on reports are byte-identical at 9,528,071 bytes, SHA256
`a4e1c0ca6baad3905a279592ad6800c3d51b7c9fdf7f38f55933e4b464eac62d`.
Native checks include all twelve vectors, signed/fractional and order-sensitive
controls, quality gates, missing inputs/coverage and serial/Rayon replay.
Targeted strict CLI/PoB Clippy and formatting pass.

Publication uses the existing V5 migration and exact append inverse, preserving
all five unchanged original requests and all 110 queries. Publication01 exposed
a helper comparing authored receiver-target order with canonical order; both
endpoint and inverse checks now canonicalize expected targets before full-record
comparison. No formula or compiler behavior changed. The checked successor is
`runs/owned-local-defence-composition-02/package`; see
[implementation](../../../../../docs/implementation.md) for its receipt and
resume point. Selected issues remain 106/117/109/122/5 and complete native builds
remain 0/5. No runtime source parser or new public operation is introduced.
