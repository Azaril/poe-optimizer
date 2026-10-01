# Ordinary ItemSet membership

This policy supplies eight reusable item-template base facts to the bounded
`PobOrdinaryItemSetsV1` normalizer. It allows the normalizer to prove an ordinary
saved ItemSet's equipment inventory complete when every item reference is known
and every referenced item's augment inventory is proven empty. It does not
complete item modifiers, item parameters, legality, calculations, or other presets.

The template predicates are exact typed records from the pinned item-loading
catalogue. Missing source predicates mean Lua `nil` (false in these conditionals),
not an assumed owned socket capability. The 1,756 recognized source base names
serve only as a rejection inventory for later base replacement; they do not grant
facts for other templates. Two injected title keys reject `ItemsTab.Load`'s
implicit jewel-socket fallback, which also applies to rare items with those titles.
No build IDs, physical item IDs, or selected build names enter the policy.

`authoring.json` binds the exact predecessor, source revision, full source
manifest, module pins, and prior base conversion. The default Rust test rejoins
all facts to the tracked owned base policy and authenticated typed source data.
The optional `owned_empty_equipment_augments` source test executes complete
original `Item.ParseRaw`, `Item.BuildModList`, and `ItemsTab.Load` in both JIT modes;
it includes occupied, unknown, reconstructed, missing-header, and loader-title
contrasts. The production normalizer runs without PoB.

The full release test uses the existing checked normalization publication seam,
then assembles a full endpoint preserving prior provenance plus this explicit
authoring entry. The compact intermediate is not the final baseline. It compares
all predecessor artifact bytes, all 110 saved queries, rebuilt publication bytes,
and all five freshly normalized drafts. Saved source selectors determine each
checked request; no selected request is replaced.

Reproduce with an explicitly provided checked predecessor and a new output path:

```powershell
cargo test --test owned_equipment_membership_cli
$env:POE_OPTIMIZER_TEST_EQUIPMENT_MEMBERSHIP_PRIOR = 'runs/owned-support-origin-order-02/package'
$env:POE_OPTIMIZER_TEST_EQUIPMENT_MEMBERSHIP_OUTPUT = 'runs/owned-equipment-membership-03'
cargo test --test owned_equipment_membership_cli -- --ignored --exact real_publication_closes_only_proven_equipment_membership
```

The real publication test is explicitly ignored without its predecessor. Its
`validation.json` and individual selected reports record the executable results;
no completed-build numerical parity is claimed.

The first real authoring attempt was rejected before publication: the canonical
policy plus the largest original query set required 1,051,913 bytes, exceeding
the previous 1 MiB pair limit. Its input and failure receipt are retained under
`runs/owned-equipment-membership-01`; it is not a release baseline. The complete
source rejection inventory and every original query remain required.

The validated baseline is `runs/owned-equipment-membership-03/package`, input
`5045973b11caa31e185cdbf2894f223b80d7400be5b0a2f8c47a2dbcd6220884`.
Its schema `cc4ebdffaead1b2aa58802b3a5ade812ceb58d39d8134aaf5939e4a651faa054`
and registry `f98c0b22c2d6f1bcf43a790937ac8398c9df1e822e10ce1ee0dc302d937f4437`
are unchanged. All eight prior provenance entries remain, followed by one explicit
normalization entry. Rebuilding the full package is byte-identical.

The focused default test passed with the real test explicitly ignored; the real
test then passed against the provided predecessor. Original05's saved ItemSet2
now has Complete equipment membership over its same nine receiving uses and eight
items, including the two distinct ring uses of one item. Only its selected
`equipment-membership-not-converted` issue disappears. The five selected issue
counts are **314, 322, 313, 379, 153**; all other issue records, source links,
allocator IDs, queries and original selections are preserved. Unknown Rune,
missing explicit None, and both loader-title mutations retain Pending membership.

The earlier `02` publication retains the same data identity but failed the
positive normalization regression because the first strict grammar omitted the
saved `SocketIdURL.name` metadata. Original `ItemsTab.Load` ignores that field,
while `Save` emits it. The corrected grammar admits that metadata without creating
an equipment member; `03` retains the passing binary receipt and selected reports.
All five requests still remain Pending, with calculation `not_run` and completed
build numerical coverage **0/5**.
